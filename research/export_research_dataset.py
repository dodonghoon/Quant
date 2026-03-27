"""
Research-Ready Dataset Exporter
================================
Joins AI regime decisions (SQLite) with live tick data (QuestDB)
and exports flat CSVs for ML model fine-tuning and backtesting.

Data sources:
  PRIMARY   — SQLite  audit.db            → regime + rationale + market context
  SECONDARY — QuestDB upbit_tickers       → nanosecond-precision OHLCV ticks
  TERTIARY  — Redis   quant:market_data   → latest live snapshot (for diagnostics)

QuestDB is queried via HTTP REST API (port 9000) — zero extra Python dependencies.
PostgreSQL wire protocol (port 8812) is supported but requires asyncpg/psycopg2.

Usage:
    cd C:\\Users\\tohno\\OneDrive\\Desktop\\quant
    python research/export_research_dataset.py [--days 7] [--out research/Backtest_Data/]

Output files:
    regime_decisions.csv      — One row per AI classification cycle (SQLite only)
    tick_data_joined.csv      — Regime decisions + OHLCV per symbol per decision window
    export_summary.txt        — Record counts, date range, regime distribution, QuestDB status
"""

import os
import sys
import io
import json
import csv
import sqlite3
import argparse
import asyncio
import urllib.request
import urllib.parse
from pathlib import Path
from datetime import datetime, timezone, timedelta
from collections import Counter, defaultdict

if sys.platform == "win32":
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding="utf-8", errors="replace")

# ── Configuration ─────────────────────────────────────────────────────────────
REPO_ROOT = Path(__file__).parent.parent
ENV_FILE  = REPO_ROOT / "config" / ".env.production"


def _load_env() -> None:
    if not ENV_FILE.exists():
        return
    for line in ENV_FILE.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if line and not line.startswith("#") and "=" in line:
            k, _, v = line.partition("=")
            os.environ.setdefault(k.strip(), v.strip())


_load_env()

AUDIT_DB_PATH     = REPO_ROOT / os.getenv("AUDIT_DB_PATH", "audit.db")
REDIS_URL         = os.getenv("REDIS_URL",         "redis://127.0.0.1:6379")
QUESTDB_HOST      = os.getenv("QUESTDB_HOST",      "localhost")
QUESTDB_HTTP_PORT = int(os.getenv("QUESTDB_HTTP_PORT", "9000"))
QUESTDB_BASE_URL  = f"http://{QUESTDB_HOST}:{QUESTDB_HTTP_PORT}"

# Join window: for each regime decision ts, pull ticks ±N minutes
TICK_WINDOW_MINUTES = 5

# Symbols to include in the joined output
JOIN_SYMBOLS = ["KRWBTC", "KRWETH", "KRWXRP", "KRWSOL"]


# ── SQLite ─────────────────────────────────────────────────────────────────────

def fetch_regime_decisions(days: int = 30) -> list[dict]:
    if not AUDIT_DB_PATH.exists():
        print(f"[WARN] audit.db not found at {AUDIT_DB_PATH}")
        print("       Run `python research/llm_regime_engine.py` at least once.")
        return []

    cutoff = (datetime.now(timezone.utc) - timedelta(days=days)).strftime(
        "%Y-%m-%d %H:%M:%S"
    )
    con = sqlite3.connect(str(AUDIT_DB_PATH), timeout=10)
    con.row_factory = sqlite3.Row
    cur = con.cursor()
    cur.execute(
        """
        SELECT id, ts, regime, rationale, detail
        FROM   audit_log
        WHERE  action = 'REGIME_CLASSIFICATION'
          AND  ts    >= ?
        ORDER  BY ts ASC
        """,
        (cutoff,),
    )
    rows = cur.fetchall()
    con.close()

    records = []
    for row in rows:
        try:
            ctx = json.loads(row["detail"]) if row["detail"] else {}
        except json.JSONDecodeError:
            ctx = {}

        records.append({
            "id":              row["id"],
            "ts":              row["ts"],
            "regime":          row["regime"],
            "regime_label":    _regime_to_int(row["regime"]),
            "rationale":       row["rationale"],
            "btc_24h_pct":     ctx.get("btc_7d_pct",         ""),
            "eth_24h_pct":     ctx.get("eth_7d_pct",         ""),
            "altcoin_avg_pct": ctx.get("altcoin_avg_7d_pct", ""),
            "divergence_pct":  ctx.get("divergence_pct",     ""),
            "data_source":     ctx.get("data_source",        "unknown"),
            "symbols_tracked": ctx.get("symbols_tracked",    ""),
            "updated_at_unix": ctx.get("updated_at",         ""),
        })

    return records


def _regime_to_int(regime: str) -> int:
    return {
        "altseason": 0, "btc_dominance": 1, "ranging": 2,
        "high_risk": 3, "neutral": 4, "DATA_STALE": -1,
    }.get(regime, -99)


# ── QuestDB HTTP REST ─────────────────────────────────────────────────────────

def questdb_query(sql: str, limit: int = 10000) -> tuple[list[str], list[list]]:
    """
    Execute SQL against QuestDB HTTP REST API (port 9000).
    Returns (column_names, rows).
    Raises ConnectionError if QuestDB is unreachable.
    """
    params = urllib.parse.urlencode({"query": sql, "limit": limit})
    url = f"{QUESTDB_BASE_URL}/exec?{params}"

    req = urllib.request.Request(url, headers={"Accept": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=10) as resp:
            payload = json.loads(resp.read().decode("utf-8"))
    except Exception as e:
        raise ConnectionError(f"QuestDB HTTP request failed: {e}") from e

    if "error" in payload:
        raise ValueError(f"QuestDB query error: {payload['error']}")

    cols = [c["name"] for c in payload.get("columns", [])]
    rows = payload.get("dataset", [])
    return cols, rows


def questdb_is_available() -> bool:
    try:
        questdb_query("SELECT 1", limit=1)
        return True
    except Exception:
        return False


def questdb_tick_count() -> int:
    """Return total rows in upbit_tickers, or -1 if table doesn't exist yet."""
    try:
        _, rows = questdb_query("SELECT count() FROM upbit_tickers", limit=1)
        return int(rows[0][0]) if rows else 0
    except Exception:
        return -1


def fetch_ohlcv_for_window(
    symbol: str, start_ts: str, end_ts: str
) -> dict:
    """
    Fetch OHLCV summary for a symbol within a time window.
    start_ts / end_ts: ISO8601 strings like '2026-03-27T05:12:00.000Z'

    Returns dict with keys: open, high, low, close, volume, tick_count
    """
    sql = f"""
        SELECT
            first(price)      AS open,
            max(price)        AS high,
            min(price)        AS low,
            last(price)       AS close,
            sum(volume)       AS volume,
            count()           AS tick_count
        FROM upbit_tickers
        WHERE symbol = '{symbol}'
          AND ts >= '{start_ts}'
          AND ts <= '{end_ts}'
    """
    try:
        cols, rows = questdb_query(sql, limit=1)
        if not rows or rows[0][0] is None:
            return {"open": "", "high": "", "low": "", "close": "", "volume": "", "tick_count": 0}
        row = rows[0]
        return dict(zip(cols, row))
    except Exception as e:
        debug_log = f"[QDB] OHLCV query failed for {symbol}: {e}"
        return {"open": "", "high": "", "low": "", "close": "", "volume": "", "tick_count": 0,
                "_error": str(e)}


def build_joined_records(
    decisions: list[dict],
    window_minutes: int = TICK_WINDOW_MINUTES,
    symbols: list[str] = JOIN_SYMBOLS,
) -> list[dict]:
    """
    For each regime decision, query QuestDB for OHLCV of each symbol
    in a [ts - window, ts + window] window and flatten into one row.
    """
    joined = []
    for d in decisions:
        # Parse decision timestamp → UTC window bounds
        try:
            # SQLite stores KST-local or UTC depending on how it was written
            # Our engine inserts via datetime('now') which is UTC in SQLite
            dt = datetime.fromisoformat(d["ts"].replace(" ", "T"))
            if dt.tzinfo is None:
                dt = dt.replace(tzinfo=timezone.utc)
        except Exception:
            joined.append({**d})
            continue

        window = timedelta(minutes=window_minutes)
        start_str = (dt - window).strftime("%Y-%m-%dT%H:%M:%S.000Z")
        end_str   = (dt + window).strftime("%Y-%m-%dT%H:%M:%S.000Z")

        row = dict(d)  # start with all SQLite fields
        row["qdb_window_start"] = start_str
        row["qdb_window_end"]   = end_str

        for sym in symbols:
            ohlcv = fetch_ohlcv_for_window(sym, start_str, end_str)
            prefix = sym.lower()  # e.g. "krwbtc"
            row[f"{prefix}_open"]       = ohlcv.get("open",       "")
            row[f"{prefix}_high"]       = ohlcv.get("high",       "")
            row[f"{prefix}_low"]        = ohlcv.get("low",        "")
            row[f"{prefix}_close"]      = ohlcv.get("close",      "")
            row[f"{prefix}_volume"]     = ohlcv.get("volume",     "")
            row[f"{prefix}_tick_count"] = ohlcv.get("tick_count", 0)

        joined.append(row)

    return joined


# ── Redis snapshot ─────────────────────────────────────────────────────────────

async def fetch_latest_redis_snapshot() -> dict:
    try:
        import redis.asyncio as aioredis
        r = aioredis.from_url(REDIS_URL)
        raw = await r.get("quant:market_data")
        await r.aclose()
        if raw:
            return json.loads(raw)
    except Exception as e:
        print(f"[WARN] Redis GET failed: {e}")
    return {}


# ── CSV writers ───────────────────────────────────────────────────────────────

REGIME_COLUMNS = [
    "id", "ts", "regime", "regime_label", "rationale",
    "btc_24h_pct", "eth_24h_pct", "altcoin_avg_pct", "divergence_pct",
    "data_source", "symbols_tracked", "updated_at_unix",
]


def _joined_columns(symbols: list[str]) -> list[str]:
    base = REGIME_COLUMNS + ["qdb_window_start", "qdb_window_end"]
    for sym in symbols:
        p = sym.lower()
        base += [
            f"{p}_open", f"{p}_high", f"{p}_low", f"{p}_close",
            f"{p}_volume", f"{p}_tick_count",
        ]
    return base


def write_csv(records: list[dict], path: Path, columns: list[str]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "w", newline="", encoding="utf-8") as f:
        writer = csv.DictWriter(f, fieldnames=columns, extrasaction="ignore")
        writer.writeheader()
        writer.writerows(records)
    print(f"[OK]  Written {len(records):>5} rows  →  {path}")


def write_summary(
    decisions: list[dict],
    joined: list[dict],
    redis_snap: dict,
    qdb_available: bool,
    qdb_tick_total: int,
    out_dir: Path,
) -> None:
    path = out_dir / "export_summary.txt"
    regime_counts = Counter(r["regime"] for r in decisions)
    data_sources  = Counter(r["data_source"] for r in decisions)

    lines = [
        "=" * 62,
        "  Quant Research Dataset — Export Summary",
        f"  Generated  : {datetime.now().isoformat()}",
        f"  Audit DB   : {AUDIT_DB_PATH}",
        f"  QuestDB    : {QUESTDB_BASE_URL}",
        "=" * 62,
        "",
        f"  SQLite decisions : {len(decisions)}",
        f"  Joined rows      : {len(joined)}",
        f"  Date range       : "
        + (f"{decisions[0]['ts']}  →  {decisions[-1]['ts']}" if decisions else "N/A"),
        "",
        "  Regime distribution:",
    ]
    for regime, cnt in sorted(regime_counts.items(), key=lambda x: -x[1]):
        pct = cnt / len(decisions) * 100 if decisions else 0
        lines.append(f"    {regime:<22} {cnt:>4}  ({pct:5.1f}%)")

    lines += ["", "  Data sources (SQLite):"]
    for src, cnt in data_sources.items():
        lines.append(f"    {src:<22} {cnt:>4}")

    lines += [
        "",
        "  QuestDB Status:",
        f"    Available        : {'YES' if qdb_available else 'NO — run setup_questdb.ps1'}",
        f"    upbit_tickers    : {qdb_tick_total if qdb_tick_total >= 0 else 'table not yet created'} rows",
        f"    Join window      : +/- {TICK_WINDOW_MINUTES} min per regime decision",
        f"    Symbols joined   : {', '.join(JOIN_SYMBOLS)}",
        "",
        "  Latest Redis snapshot:",
        f"    btc_24h_pct      : {redis_snap.get('btc_7d_pct', 'N/A')}",
        f"    eth_24h_pct      : {redis_snap.get('eth_7d_pct', 'N/A')}",
        f"    data_source      : {redis_snap.get('data_source', 'N/A')}",
        f"    symbols_tracked  : {redis_snap.get('symbols_tracked', 'N/A')}",
        "",
        "  Output CSV columns (regime_decisions.csv):",
    ]
    for col in REGIME_COLUMNS:
        lines.append(f"    {col}")

    if qdb_available:
        lines += ["", "  Additional columns (tick_data_joined.csv):"]
        for col in _joined_columns(JOIN_SYMBOLS):
            if col not in REGIME_COLUMNS:
                lines.append(f"    {col}")

    lines += ["", "=" * 62]
    path.write_text("\n".join(lines), encoding="utf-8")
    print(f"[OK]  Summary  →  {path}")


# ── Main ──────────────────────────────────────────────────────────────────────

async def _main(days: int, out_dir: Path) -> None:
    print(f"\n{'=' * 62}")
    print(f"  Research Dataset Export")
    print(f"  Audit DB  : {AUDIT_DB_PATH}")
    print(f"  QuestDB   : {QUESTDB_BASE_URL}")
    print(f"  Output    : {out_dir}")
    print(f"  Lookback  : last {days} day(s)")
    print(f"{'=' * 62}\n")

    # 1. SQLite regime decisions
    decisions = fetch_regime_decisions(days=days)
    print(f"[INFO] Loaded {len(decisions)} regime decisions from SQLite")

    # 2. Redis snapshot (diagnostic only)
    redis_snap = await fetch_latest_redis_snapshot()
    print(f"[INFO] Redis snapshot: data_source={redis_snap.get('data_source', 'unavailable')}")

    # 3. QuestDB availability check
    qdb_ok = questdb_is_available()
    qdb_tick_total = questdb_tick_count() if qdb_ok else -1
    if qdb_ok:
        print(f"[INFO] QuestDB ONLINE — upbit_tickers has {qdb_tick_total} rows")
    else:
        print(f"[WARN] QuestDB OFFLINE — run setup_questdb.ps1 and data-ingestion")
        print(f"       Tick-level join will be skipped.")

    out_dir.mkdir(parents=True, exist_ok=True)

    # 4. Write regime decisions CSV (always available)
    write_csv(decisions, out_dir / "regime_decisions.csv", REGIME_COLUMNS)

    # 5. Joined CSV (only if QuestDB is live AND has data)
    joined: list[dict] = []
    if qdb_ok and qdb_tick_total > 0 and decisions:
        print(f"[INFO] Joining {len(decisions)} decisions with QuestDB ticks "
              f"(+/-{TICK_WINDOW_MINUTES}min, symbols: {', '.join(JOIN_SYMBOLS)}) ...")
        joined = build_joined_records(decisions)
        write_csv(joined, out_dir / "tick_data_joined.csv", _joined_columns(JOIN_SYMBOLS))
    elif qdb_ok and qdb_tick_total == 0:
        print(f"[INFO] QuestDB online but upbit_tickers is empty.")
        print(f"       Start `cargo run -p data-ingestion` to populate ticks.")
    else:
        print(f"[INFO] Skipping tick join (QuestDB offline).")

    # 6. Summary
    write_summary(decisions, joined, redis_snap, qdb_ok, qdb_tick_total, out_dir)

    print(f"\n[DONE] Export complete.")
    if decisions:
        top = Counter(r["regime"] for r in decisions).most_common(1)[0]
        print(f"       Dominant regime : {top[0]} ({top[1]} / {len(decisions)} decisions)")
    if qdb_ok:
        print(f"       QuestDB ticks   : {qdb_tick_total:,}")
    else:
        print(f"       QuestDB ticks   : N/A (offline)")


def main() -> None:
    parser = argparse.ArgumentParser(description="Export regime decisions + tick data to CSV")
    parser.add_argument("--days", type=int, default=30,
                        help="Lookback window in days (default: 30)")
    parser.add_argument("--out", type=Path,
                        default=REPO_ROOT / "research" / "Backtest_Data",
                        help="Output directory (default: research/Backtest_Data/)")
    args = parser.parse_args()
    asyncio.run(_main(days=args.days, out_dir=args.out))


if __name__ == "__main__":
    main()

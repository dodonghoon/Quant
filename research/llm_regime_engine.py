import os
import io
import sys
import json
import sqlite3
import asyncio
import redis.asyncio as redis
from anthropic import AsyncAnthropic
from datetime import datetime, timezone, timedelta
from pathlib import Path

# Force UTF-8 on Windows (prevents cp949 encode errors)
if sys.platform == "win32":
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding="utf-8", errors="replace")
    sys.stderr = io.TextIOWrapper(sys.stderr.buffer, encoding="utf-8", errors="replace")

KST = timezone(timedelta(hours=9))

# Maps each regime to directional execution signals.
# These are published to `quant:execution_signals` so the Rust
# execution-engine (redis_bridge.rs) can place orders immediately.
# Signal range: [-1.0, 1.0]  |  0.0 = no trade for this symbol
REGIME_SIGNALS: dict = {
    "btc_dominance": {"KRW-BTC": 0.80, "KRW-ETH": 0.25},
    "altseason":     {
        "KRW-BTC": 0.20, "KRW-ETH": 0.30,
        "KRW-XRP": 0.70, "KRW-SOL": 0.65, "KRW-ADA": 0.55,
    },
    "ranging":    {},   # No directional edge — stay flat
    "high_risk":  {},   # Capital preservation — no new entries
    "neutral":    {},   # Insufficient evidence — stay flat
    "DATA_STALE": {},   # Never trade on stale data
}

# Neutral fallback regime when the API is unreachable
NEUTRAL_FALLBACK = {
    "regime": "neutral",
    "rationale": "LLM API unreachable - defaulting to neutral regime (no directional bias)",
}


class LLMRegimeEngine:
    MAX_RETRIES = 3
    RETRY_DELAY_SEC = 5

    def __init__(self):
        from dotenv import load_dotenv
        load_dotenv("../config/.env.production")

        self.redis_url = os.getenv("REDIS_URL", "redis://127.0.0.1:6379")
        api_key = os.getenv("CLAUDE_API_KEY", "")
        self.model = os.getenv("CLAUDE_MODEL", "claude-haiku-4-5-20251001")

        # SQLite audit DB — resolved relative to the repo root (one level above research/)
        raw_db_path = os.getenv("AUDIT_DB_PATH", "audit.db")
        self.audit_db_path = str(
            (Path(__file__).parent.parent / raw_db_path).resolve()
        )

        print(f"[SERVICE_STARTED] LLM Regime Engine")
        print(f"[INIT] Model : {self.model}")
        key_preview = f"{api_key[:20]}...{api_key[-6:]}" if len(api_key) > 26 else "(too short or missing!)"
        print(f"[INIT] Key   : {key_preview}")
        print(f"[INIT] Redis : {self.redis_url}")
        print(f"[INIT] DB    : {self.audit_db_path}")

        # Health-check: verify Redis is reachable at startup
        try:
            import redis as _redis_sync
            _r = _redis_sync.from_url(self.redis_url)
            pong = _r.ping()
            print(f"[HEALTH] Redis PING → {'PONG' if pong else 'NO RESPONSE'}")
            _r.close()
        except Exception as e:
            print(f"[HEALTH] Redis unreachable: {e} — market data will use static stubs")

        self.client = AsyncAnthropic(api_key=api_key)

    # ------------------------------------------------------------------
    # PHASE 2: Live data fetch — tries Redis first, falls back to stubs
    # ------------------------------------------------------------------
    # ------------------------------------------------------------------
    # SQLite audit logging (research data persistence)
    # ------------------------------------------------------------------
    def _write_to_audit_db_sync(self, regime: str, rationale: str, context: dict) -> None:
        """
        Synchronous SQLite write — always runs in a thread pool via asyncio.to_thread().
        Inserts a REGIME_CLASSIFICATION row into audit_log with dedicated
        regime / rationale columns (schema created/migrated by Rust web-dashboard).
        """
        detail_json = json.dumps(context)
        try:
            con = sqlite3.connect(self.audit_db_path, timeout=5)
            cur = con.cursor()

            # Ensure table & columns exist (mirrors Rust logger.rs schema)
            cur.execute("""
                CREATE TABLE IF NOT EXISTS audit_log (
                    id        INTEGER PRIMARY KEY AUTOINCREMENT,
                    ts        TEXT    NOT NULL DEFAULT (datetime('now')),
                    user      TEXT    NOT NULL,
                    action    TEXT    NOT NULL,
                    detail    TEXT    NOT NULL DEFAULT '{}',
                    ip        TEXT             DEFAULT '',
                    regime    TEXT    NOT NULL DEFAULT '',
                    rationale TEXT    NOT NULL DEFAULT ''
                )
            """)
            for col_def in [
                "ALTER TABLE audit_log ADD COLUMN regime TEXT NOT NULL DEFAULT ''",
                "ALTER TABLE audit_log ADD COLUMN rationale TEXT NOT NULL DEFAULT ''",
            ]:
                try:
                    cur.execute(col_def)
                except sqlite3.OperationalError:
                    pass  # Column already exists — safe to ignore

            cur.execute(
                """
                INSERT INTO audit_log (user, action, detail, ip, regime, rationale)
                VALUES ('llm_engine', 'REGIME_CLASSIFICATION', ?, '', ?, ?)
                """,
                (detail_json, regime, rationale),
            )
            con.commit()
            con.close()
        except Exception as e:
            print(f"[AUDIT] SQLite write error: {e}  (path={self.audit_db_path})")

    async def _write_to_audit_db(self, regime: str, rationale: str, context: dict) -> None:
        """Non-blocking wrapper — runs the sync SQLite write in a thread pool."""
        await asyncio.to_thread(
            self._write_to_audit_db_sync, regime, rationale, context
        )

    async def fetch_market_context(self, redis_conn) -> dict:
        """
        Attempt to read the latest market snapshot published by the
        data-ingestion service on the Redis key `quant:market_data`.
        Falls back to static stubs when the key is absent.
        """
        try:
            raw = await redis_conn.get("quant:market_data")
            if raw:
                data = json.loads(raw)
                src = data.get("data_source", "unknown")
                tracked = data.get("symbols_tracked", "?")
                updated = data.get("updated_at", "?")
                print(
                    f"[DATA] LIVE_REDIS — source={src}  "
                    f"symbols_tracked={tracked}  updated_at={updated}"
                )
                return data
            else:
                print(
                    "[DATA] LIVE_DATA_MISSING — key 'quant:market_data' not found in Redis. "
                    "Ensure `cargo run -p data-ingestion` is running and Upbit WS is connected."
                )
        except Exception as e:
            print(f"[DATA] LIVE_DATA_MISSING — Redis GET failed: {e}")

        # Static stubs — clearly labelled so operators know it's not live
        print("[DATA] Falling back to STATIC_STUB market context.")
        return {
            "btc_7d_pct": 2.1,
            "eth_7d_pct": 1.5,
            "altcoin_avg_7d_pct": 5.4,
            "divergence_pct": 3.3,
            "data_source": "static_stub",
        }

    # ------------------------------------------------------------------
    # Main cycle
    # ------------------------------------------------------------------
    async def run_cycle(self):
        redis_conn = redis.from_url(self.redis_url)

        # Capture the real wall-clock time BEFORE calling the LLM
        now_kst = datetime.now(KST)
        current_time_str = now_kst.strftime("%Y-%m-%dT%H:%M:%S%z")  # e.g. 2026-03-27T13:46:00+0900

        context = await self.fetch_market_context(redis_conn)

        # PHASE 3: Inject actual current time so the LLM cannot hallucinate a past date.
        # Also instruct it to emit DATA_STALE if inputs look inconsistent.
        system_prompt = (
            "You are a quantitative crypto strategist analyzing the Korean Upbit market. "
            "Analyze the provided technical snapshots and output ONLY a JSON object "
            "classifying the current market regime. "
            "Do not output trading signals, position sizes, or confidence scores."
        )

        user_prompt = (
            f"Current system time (KST): {current_time_str}\n\n"
            f"BTC 7d: {context['btc_7d_pct']}%, "
            f"ETH 7d: {context['eth_7d_pct']}%\n"
            f"Altcoin avg 7d: {context['altcoin_avg_7d_pct']}%\n"
            f"Divergence: {context['divergence_pct']}%\n\n"
            "Rules:\n"
            f"- Use exactly {current_time_str} as the timestamp value.\n"
            "- If the provided market data appears stale or inconsistent with the current time, "
            'set regime to "DATA_STALE" and explain in rationale.\n\n'
            "Produce exactly this JSON (no markdown fences, no extra keys):\n"
            "{\n"
            f'  "timestamp": "{current_time_str}",\n'
            '  "regime": "altseason | btc_dominance | ranging | high_risk | DATA_STALE",\n'
            '  "rationale": "<1 sentence rationale>"\n'
            "}"
        )

        result_json = None
        last_error = None

        for attempt in range(1, self.MAX_RETRIES + 1):
            try:
                print(
                    f"[{datetime.now(KST).isoformat()}] "
                    f"Attempt {attempt}/{self.MAX_RETRIES} - calling Anthropic API..."
                )

                response = await self.client.messages.create(
                    model=self.model,
                    max_tokens=300,
                    system=system_prompt,
                    messages=[{"role": "user", "content": user_prompt}],
                )

                result_text = response.content[0].text
                print(f"[{datetime.now(KST).isoformat()}] Raw LLM response: {result_text!r}")

                if not result_text or not result_text.strip():
                    print("[CRITICAL] Empty response from Anthropic. Check API key / network / proxy.")
                    raise ValueError("Empty response body from LLM")

                # Strip markdown code fences (```json ... ```) if the model adds them
                cleaned = result_text.strip()
                if cleaned.startswith("```"):
                    first_newline = cleaned.index("\n")
                    cleaned = cleaned[first_newline + 1:]
                    if cleaned.rstrip().endswith("```"):
                        cleaned = cleaned.rstrip()[:-3].rstrip()

                parsed = json.loads(cleaned)

                # ALWAYS override the timestamp with the authoritative system clock.
                # The LLM timestamp is advisory only; we never trust it for accuracy.
                parsed["timestamp"] = current_time_str
                parsed["data_source"] = context.get("data_source", "live")

                result_json = parsed
                print(
                    f"[{datetime.now(KST).isoformat()}] "
                    f"Parsed OK — regime={result_json['regime']}  "
                    f"ts={result_json['timestamp']}"
                )
                break  # success

            except json.JSONDecodeError as e:
                last_error = e
                raw = locals().get("result_text", "<unavailable>")
                print(
                    f"[{datetime.now(KST).isoformat()}] "
                    f"Attempt {attempt}/{self.MAX_RETRIES} - JSONDecodeError: {e}"
                )
                print(f"  Raw text was: {raw!r}")
                if attempt < self.MAX_RETRIES:
                    print(f"  Retrying in {self.RETRY_DELAY_SEC}s...")
                    await asyncio.sleep(self.RETRY_DELAY_SEC)

            except Exception as e:
                last_error = e
                print(
                    f"[{datetime.now(KST).isoformat()}] "
                    f"Attempt {attempt}/{self.MAX_RETRIES} - {type(e).__name__}: {e}"
                )
                if attempt < self.MAX_RETRIES:
                    print(f"  Retrying in {self.RETRY_DELAY_SEC}s...")
                    await asyncio.sleep(self.RETRY_DELAY_SEC)

        # Fallback: neutral regime with real system timestamp
        if result_json is None:
            print(
                f"[{datetime.now(KST).isoformat()}] [FALLBACK] API unavailable after "
                f"{self.MAX_RETRIES} retries. Last error: {last_error}"
            )
            result_json = {
                **NEUTRAL_FALLBACK,
                "timestamp": current_time_str,
                "data_source": context.get("data_source", "unknown"),
            }
            print(f"[{datetime.now(KST).isoformat()}] Using neutral fallback regime.")

        # ── Publish to Redis (real-time consumers) ───────────────────────────
        try:
            regime_payload = json.dumps(result_json)

            # 1) Broadcast to pub/sub subscribers (e.g. dashboards)
            await redis_conn.publish("quant:macro_regime", regime_payload)

            # 2) Cache as a readable KEY so signal_bridge._get_regime() can poll it
            #    (TTL 600s — long enough to survive a brief execution-engine restart)
            await redis_conn.set("quant:macro_regime:latest", regime_payload, ex=600)
            ttl_check = await redis_conn.ttl("quant:macro_regime:latest")
            print(
                f"[{datetime.now(KST).isoformat()}] "
                f"Published + cached regime: {result_json['regime']}  "
                f"timestamp: {result_json['timestamp']}  "
                f"key_ttl={ttl_check}s"  # confirms SET succeeded
            )

            # 3) Publish regime-derived execution signals → execution-engine
            #    (crates/execution-engine/src/redis_bridge.rs listens on quant:execution_signals)
            regime      = result_json["regime"]
            ts          = result_json["timestamp"]
            signals     = REGIME_SIGNALS.get(regime, {})

            if signals:
                for symbol, signal_val in signals.items():
                    exec_payload = json.dumps({
                        "symbol":    symbol,
                        "signal":    signal_val,
                        "regime":    regime,
                        "timestamp": ts,
                    })
                    await redis_conn.publish("quant:execution_signals", exec_payload)
                    print(
                        f"[{datetime.now(KST).isoformat()}] "
                        f"[SIGNAL] {symbol}  signal={signal_val:+.2f}  regime={regime}"
                    )
            else:
                print(
                    f"[{datetime.now(KST).isoformat()}] "
                    f"[SIGNAL] No execution signals for regime='{regime}' (flat)"
                )

        except Exception as e:
            print(f"[{datetime.now(KST).isoformat()}] Redis publish error: {e}")
        finally:
            await redis_conn.aclose()

        # ── Persist to SQLite audit DB (research / ML fine-tuning) ───────────
        # Writes regime + rationale as dedicated columns alongside the full
        # market context JSON in `detail`, enabling indexed SQL queries later.
        try:
            regime    = result_json.get("regime", "unknown")
            rationale = result_json.get("rationale", "")
            await self._write_to_audit_db(regime, rationale, context)
            print(
                f"[{datetime.now(KST).isoformat()}] "
                f"[AUDIT] Regime decision persisted to SQLite  "
                f"regime={regime}  db={self.audit_db_path}"
            )
        except Exception as e:
            print(f"[{datetime.now(KST).isoformat()}] [AUDIT] Write failed: {e}")


if __name__ == "__main__":
    engine = LLMRegimeEngine()
    asyncio.run(engine.run_cycle())

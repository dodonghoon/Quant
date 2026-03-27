"""
Upbit API Connectivity Test
===========================
Verifies that the Upbit REST API is reachable and that your credentials
are valid BEFORE committing real capital.

Modes:
  DRY_RUN=true  (default) — calls GET /v1/accounts (read-only, no cost)
  DRY_RUN=false           — places a real 1,000 KRW market BUY on KRW-XRP
                            (cheapest tradeable coin on Upbit as of 2026)

Usage:
  python test_upbit_connectivity.py              # dry-run (safe)
  DRY_RUN=false python test_upbit_connectivity.py  # live 1,000 KRW order
"""

import os
import sys
import io
import json
import uuid
import hashlib
import hmac
import time
import base64

from datetime import datetime, timezone, timedelta
from pathlib import Path

# Force UTF-8 on Windows
if sys.platform == "win32":
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding="utf-8", errors="replace")
    sys.stderr = io.TextIOWrapper(sys.stderr.buffer, encoding="utf-8", errors="replace")

try:
    import requests
    import jwt                  # PyJWT
    from dotenv import load_dotenv
except ImportError as e:
    print(f"[ERROR] Missing dependency: {e}")
    print("Run: pip install requests PyJWT python-dotenv")
    sys.exit(1)

KST = timezone(timedelta(hours=9))

# ── Load credentials ─────────────────────────────────────────────────────────
load_dotenv(str(Path(__file__).parent.parent / "config" / ".env.production"))

ACCESS_KEY  = os.getenv("UPBIT_ACCESS_KEY", "")
SECRET_KEY  = os.getenv("UPBIT_SECRET_KEY", "")
BASE_URL    = os.getenv("UPBIT_BASE_URL", "https://api.upbit.com")
DRY_RUN     = os.getenv("DRY_RUN", "true").lower() != "false"


def _build_token(query_string: str | None = None) -> str:
    """Generate an HS512 JWT for Upbit REST API authentication."""
    payload: dict = {
        "access_key": ACCESS_KEY,
        "nonce": str(uuid.uuid4()),
    }
    if query_string:
        import hashlib
        h = hashlib.sha512(query_string.encode()).hexdigest()
        payload["query_hash"]     = h
        payload["query_hash_alg"] = "SHA512"

    return jwt.encode(payload, SECRET_KEY, algorithm="HS512")


def test_credentials() -> bool:
    """GET /v1/accounts — verifies credentials without side effects."""
    print("\n[TEST 1] Credential check — GET /v1/accounts")
    token = _build_token()
    r = requests.get(
        f"{BASE_URL}/v1/accounts",
        headers={"Authorization": f"Bearer {token}"},
        timeout=10,
    )
    if r.status_code == 200:
        accounts = r.json()
        krw = next((a for a in accounts if a.get("currency") == "KRW"), None)
        balance = float(krw["balance"]) if krw else 0.0
        print(f"  [OK] Credentials valid — KRW balance: {balance:,.0f} KRW")
        return True
    else:
        print(f"  [FAIL] HTTP {r.status_code}: {r.text[:300]}")
        return False


def test_market_info() -> bool:
    """GET /v1/market/all — no auth required, checks network reachability."""
    print("\n[TEST 2] Network reachability — GET /v1/market/all")
    r = requests.get(f"{BASE_URL}/v1/market/all", timeout=10)
    if r.status_code == 200:
        markets = r.json()
        krw_count = sum(1 for m in markets if m["market"].startswith("KRW-"))
        print(f"  [OK] Upbit reachable — {krw_count} KRW markets listed")
        return True
    else:
        print(f"  [FAIL] HTTP {r.status_code}: {r.text[:200]}")
        return False


def test_order_1000_krw() -> bool:
    """
    POST /v1/orders — places a real 1,000 KRW market BUY on KRW-BTC.
    ord_type='price' means "spend exactly this many KRW at market price".
    """
    print("\n[TEST 3] Live order test — 1,000 KRW market BUY on KRW-BTC")
    import urllib.parse

    params = {
        "market":   "KRW-BTC",
        "side":     "bid",
        "price":    "5000",
        "ord_type": "price",   # 'price' = market buy by KRW amount
    }
    query_string = urllib.parse.urlencode(params)
    token = _build_token(query_string)

    r = requests.post(
        f"{BASE_URL}/v1/orders",
        params=params,
        headers={"Authorization": f"Bearer {token}"},
        timeout=10,
    )
    if r.status_code in (200, 201):
        resp = r.json()
        print(f"  [OK] Order accepted — uuid: {resp.get('uuid')}")
        print(f"       state={resp.get('state')}  market={resp.get('market')}")
        print(f"       side={resp.get('side')}  price={resp.get('price')}")
        return True
    else:
        print(f"  [FAIL] HTTP {r.status_code}: {r.text[:400]}")
        return False


def main() -> None:
    now = datetime.now(KST).strftime("%Y-%m-%dT%H:%M:%S %Z")
    print("=" * 60)
    print(f"  UPBIT CONNECTIVITY TEST  [{now}]")
    print(f"  BASE_URL   : {BASE_URL}")
    print(f"  ACCESS_KEY : {ACCESS_KEY[:12]}..." if ACCESS_KEY else "  ACCESS_KEY : (NOT SET)")
    print(f"  MODE       : {'DRY_RUN (read-only)' if DRY_RUN else 'LIVE — will place a 1,000 KRW order!'}")
    print("=" * 60)

    if not ACCESS_KEY or not SECRET_KEY:
        print("\n[ERROR] UPBIT_ACCESS_KEY / UPBIT_SECRET_KEY not found in config/.env.production")
        sys.exit(1)

    results = []
    results.append(("Network reachability", test_market_info()))
    results.append(("Credential validation", test_credentials()))

    if not DRY_RUN:
        results.append(("1,000 KRW live order KRW-BTC", test_order_1000_krw()))
    else:
        print("\n[TEST 3] Live order test — SKIPPED (DRY_RUN=true, set DRY_RUN=false to enable)")

    print("\n" + "=" * 60)
    print("  RESULTS SUMMARY")
    print("=" * 60)
    all_pass = True
    for name, ok in results:
        status = "PASS" if ok else "FAIL"
        print(f"  [{status}] {name}")
        if not ok:
            all_pass = False

    print("=" * 60)
    if all_pass:
        print("  Connectivity test PASSED — execution-engine can reach Upbit.")
    else:
        print("  One or more tests FAILED — check credentials and network.")
    print()


if __name__ == "__main__":
    main()

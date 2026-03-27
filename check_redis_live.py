"""
Redis Live Data Diagnostic
Checks whether the Rust data-ingestion service is publishing to Redis.

Usage:
    python check_redis_live.py
"""
import asyncio
import json
import os
import sys
import io
from datetime import datetime

if sys.platform == "win32":
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding="utf-8", errors="replace")

try:
    from dotenv import load_dotenv
    load_dotenv("config/.env.production")
except Exception:
    pass

import redis.asyncio as redis

REDIS_URL    = os.getenv("REDIS_URL", "redis://127.0.0.1:6379")
LISTEN_SECS  = 10
MARKET_KEY   = "quant:market_data"
LIVE_CHANNEL = "ticks:live"


async def check_static_key(r):
    """Check if quant:market_data GET key exists (written by Rust SET)."""
    print(f"\n[1] Checking Redis key GET '{MARKET_KEY}' ...")
    raw = await r.get(MARKET_KEY)
    if raw:
        data = json.loads(raw)
        src      = data.get("data_source", "unknown")
        updated  = data.get("updated_at", "?")
        btc      = data.get("btc_7d_pct", "?")
        tracked  = data.get("symbols_tracked", "?")
        print(f"    [OK] Key exists!")
        print(f"         data_source     : {src}")
        print(f"         symbols_tracked : {tracked}")
        print(f"         btc_24h_pct     : {btc}")
        print(f"         updated_at      : {updated}")
        return True
    else:
        print(f"    [MISSING] Key '{MARKET_KEY}' not found.")
        print(f"    -> Is 'cargo run -p data-ingestion' running?")
        return False


async def check_pubsub(r):
    """Listen to ticks:live Pub/Sub for LISTEN_SECS seconds."""
    print(f"\n[2] Subscribing to Pub/Sub channel '{LIVE_CHANNEL}' for {LISTEN_SECS}s ...")
    pubsub = r.pubsub()
    await pubsub.subscribe(LIVE_CHANNEL)

    received = 0
    deadline = asyncio.get_event_loop().time() + LISTEN_SECS

    try:
        while asyncio.get_event_loop().time() < deadline:
            try:
                msg = await asyncio.wait_for(pubsub.get_message(ignore_subscribe_messages=True), timeout=1.0)
            except asyncio.TimeoutError:
                continue

            if msg and msg.get("type") == "message":
                received += 1
                try:
                    payload = json.loads(msg["data"])
                    code  = payload.get("code", "?")
                    price = payload.get("price", "?")
                    chg   = payload.get("change_pct_24h", "?")
                    print(f"    [TICK #{received}] {code:12s}  price={price:>12,.0f} KRW  24h={chg:+.2f}%")
                except Exception:
                    print(f"    [TICK #{received}] raw={msg['data']!r:.80}")

                if received >= 5:
                    print(f"    ... (stopping after 5 messages)")
                    break
    finally:
        await pubsub.unsubscribe(LIVE_CHANNEL)
        await pubsub.aclose()

    if received == 0:
        print(f"    [NO DATA] No messages on '{LIVE_CHANNEL}' in {LISTEN_SECS}s.")
        print(f"    -> Rust feed.rs is not publishing ticks. Is data-ingestion running?")
    else:
        print(f"\n    [OK] Received {received} live tick(s) from Upbit via Rust.")

    return received > 0


async def main():
    print("=" * 60)
    print(f"  Redis Live Data Diagnostic — {datetime.now().isoformat()}")
    print(f"  REDIS_URL: {REDIS_URL}")
    print("=" * 60)

    try:
        r = redis.from_url(REDIS_URL)
        pong = await r.ping()
        print(f"\n[0] Redis PING: {'PONG (connected)' if pong else 'FAILED'}")
    except Exception as e:
        print(f"\n[0] Redis PING FAILED: {e}")
        print("    -> Is Redis/Memurai running? Check 'net start Memurai' or 'redis-server'.")
        return

    key_ok    = await check_static_key(r)
    pubsub_ok = await check_pubsub(r)

    await r.aclose()

    print("\n" + "=" * 60)
    print("  SUMMARY")
    print(f"  Redis Data Flow  : {'ACTIVE' if pubsub_ok else 'NONE'}")
    print(f"  quant:market_data: {'LIVE' if key_ok else 'MISSING'}")
    print(f"  Data Source      : {'LIVE_REDIS' if key_ok else 'STUB (not yet live)'}")
    print(f"  System Ready     : {'YES' if (key_ok and pubsub_ok) else 'NO — start data-ingestion first'}")
    print("=" * 60)


if __name__ == "__main__":
    asyncio.run(main())

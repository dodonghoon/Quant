"""
Python → Rust Signal Bridge
============================
앙상블 전략 신호를 계산하고 Redis Pub/Sub으로
Rust execution-engine에 전달합니다.

채널: quant:execution_signals
Rust 구독자: crates/execution-engine/src/redis_bridge.rs

발행 페이로드:
    {
      "symbol": "KRW-XRP",
      "signal": 0.72,       # [-1.0, 1.0]
      "regime": "altseason"
    }
"""

from __future__ import annotations

import asyncio
import json
import os
from datetime import datetime
from typing import Optional

import redis.asyncio as redis
from dotenv import load_dotenv

# Rust 브리지와 반드시 동일한 채널명
SIGNAL_CHANNEL = "quant:execution_signals"
REGIME_CHANNEL = "quant:macro_regime"

# 업비트 KRW 페어 목록
SYMBOLS = [
    "KRW-XRP", "KRW-SOL", "KRW-ADA", "KRW-DOGE",
    "KRW-AVAX", "KRW-LINK", "KRW-DOT", "KRW-ATOM",
    "KRW-BTC", "KRW-ETH",
]


class SignalBridge:
    """
    Redis 브리지 — 앙상블 신호를 quant:execution_signals에 발행합니다.
    """

    def __init__(self) -> None:
        load_dotenv("../config/.env.production")
        self.redis_url = os.getenv("REDIS_URL", "redis://127.0.0.1:6379")
        self._current_regime: str = "neutral"

    async def _get_regime(self, redis_conn: redis.Redis) -> str:
        """quant:macro_regime에서 최신 레짐을 읽습니다 (최근 캐시)."""
        cached = await redis_conn.get("quant:macro_regime:latest")
        if cached:
            try:
                data = json.loads(cached)
                return data.get("regime", "neutral")
            except Exception:
                pass
        return self._current_regime

    async def publish_signal(
        self,
        symbol: str,
        signal: float,
        regime: Optional[str] = None,
    ) -> None:
        """
        단일 신호를 quant:execution_signals에 발행합니다.

        Parameters
        ----------
        symbol  : Upbit 마켓 코드 (예: "KRW-XRP")
        signal  : 앙상블 신호 [-1.0, 1.0]
        regime  : 현재 레짐 문자열 (None이면 Redis 캐시에서 조회)
        """
        r = redis.from_url(self.redis_url)
        try:
            if regime is None:
                regime = await self._get_regime(r)

            payload = {
                "symbol": symbol,
                "signal": round(float(signal), 6),
                "regime": regime,
                "timestamp": datetime.utcnow().isoformat(),
            }
            await r.publish(SIGNAL_CHANNEL, json.dumps(payload))
        finally:
            await r.aclose()

    async def publish_batch(
        self,
        signals: dict[str, float],
        regime: Optional[str] = None,
    ) -> None:
        """
        여러 심볼의 신호를 한 번에 발행합니다.

        Parameters
        ----------
        signals : {symbol: signal_float} 딕셔너리
        regime  : 현재 레짐
        """
        r = redis.from_url(self.redis_url)
        try:
            if regime is None:
                regime = await self._get_regime(r)

            for symbol, signal in signals.items():
                if abs(signal) < 0.10:   # MIN_SIGNAL_THRESHOLD — Rust 쪽과 동일
                    continue
                payload = {
                    "symbol": symbol,
                    "signal": round(float(signal), 6),
                    "regime": regime,
                    "timestamp": datetime.utcnow().isoformat(),
                }
                await r.publish(SIGNAL_CHANNEL, json.dumps(payload))
                print(
                    f"[{datetime.now().isoformat()}] Published | "
                    f"{symbol} signal={signal:.3f} regime={regime}"
                )
        finally:
            await r.aclose()


# ── 빠른 테스트 / 단독 실행 ────────────────────────────────────────────────

async def _demo() -> None:
    """mock 신호를 발행하는 데모 (Redis 서버 필요)."""
    bridge = SignalBridge()
    mock_signals = {
        "KRW-XRP":  0.72,
        "KRW-SOL":  0.55,
        "KRW-ADA": -0.30,
        "KRW-DOGE": 0.08,   # 임계값 미만 → 필터됨
    }
    print(f"Publishing {len(mock_signals)} signals to '{SIGNAL_CHANNEL}' ...")
    await bridge.publish_batch(mock_signals, regime="altseason")
    print("Done.")


if __name__ == "__main__":
    asyncio.run(_demo())

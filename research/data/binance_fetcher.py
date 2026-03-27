"""
Binance Public API Data Fetcher
================================
바이낸스 선물 공개 API에서 펀딩레이트 및 미결제약정(OI) 데이터를 수집합니다.
API Key / Secret 불필요 — 모든 엔드포인트는 퍼블릭입니다.

지원 데이터:
  - 펀딩레이트: GET https://fapi.binance.com/fapi/v1/fundingRate
  - 미결제약정 (1h): GET https://fapi.binance.com/futures/data/openInterestHist

캐시 정책:
  - CSV 파일이 존재하고 마지막 수정시간이 1시간 이내이면 재수집 생략
  - force_refresh=True 시 캐시 무시
"""

from __future__ import annotations

import os
import time
from datetime import datetime, timezone, timedelta
from pathlib import Path
from typing import Optional

import pandas as pd
import requests
from loguru import logger

# ---------------------------------------------------------------------------
# 상수
# ---------------------------------------------------------------------------
_FAPI_BASE = "https://fapi.binance.com"
_FUNDING_ENDPOINT = f"{_FAPI_BASE}/fapi/v1/fundingRate"
_OI_ENDPOINT = f"{_FAPI_BASE}/futures/data/openInterestHist"

_KST = timezone(timedelta(hours=9))
_DATA_DIR = Path(__file__).parent
_FUNDING_CSV = _DATA_DIR / "funding_rates.csv"
_OI_CSV = _DATA_DIR / "open_interest.csv"

_CACHE_TTL_SECONDS = 3600  # 1시간
_REQUEST_INTERVAL = 0.1    # Rate limit 준수용 sleep (초)
_TIMEOUT = 10              # requests timeout (초)


# ---------------------------------------------------------------------------
# 심볼 매핑
# ---------------------------------------------------------------------------
def map_upbit_to_binance(upbit_ticker: str) -> Optional[str]:
    """
    업비트 KRW 마켓 티커를 바이낸스 선물 심볼로 변환합니다.

    Parameters
    ----------
    upbit_ticker : str
        업비트 티커 (예: "KRW-BTC", "KRW-ETH").

    Returns
    -------
    str or None
        바이낸스 선물 심볼 (예: "BTCUSDT").
        바이낸스 선물에 미상장된 경우 None 반환.
    """
    if not upbit_ticker.startswith("KRW-"):
        return None

    base = upbit_ticker.split("-", 1)[1]  # "KRW-BTC" → "BTC"
    return f"{base}USDT"


# ---------------------------------------------------------------------------
# 펀딩레이트 수집
# ---------------------------------------------------------------------------
def get_funding_rates(symbol: str, limit: int = 500) -> pd.DataFrame:
    """
    바이낸스 선물 펀딩레이트 히스토리를 수집합니다.

    Parameters
    ----------
    symbol : str
        바이낸스 선물 심볼 (예: "BTCUSDT").
    limit : int
        수집할 최대 레코드 수. 최대 500. 기본값 500.

    Returns
    -------
    pd.DataFrame
        컬럼: [symbol, fundingTime, fundingRate]
        fundingTime은 KST (Asia/Seoul) datetime으로 변환됩니다.

    Raises
    ------
    requests.HTTPError
        HTTP 4xx/5xx 응답 시 발생.
    """
    params = {"symbol": symbol, "limit": limit}
    resp = requests.get(_FUNDING_ENDPOINT, params=params, timeout=_TIMEOUT)
    resp.raise_for_status()

    raw: list[dict] = resp.json()
    if not raw:
        return pd.DataFrame(columns=["symbol", "fundingTime", "fundingRate"])

    df = pd.DataFrame(raw)

    # fundingTime: Unix ms → KST datetime
    df["fundingTime"] = pd.to_datetime(df["fundingTime"], unit="ms", utc=True).dt.tz_convert(_KST)

    df["fundingRate"] = df["fundingRate"].astype(float)
    df["symbol"] = symbol

    return df[["symbol", "fundingTime", "fundingRate"]].reset_index(drop=True)


# ---------------------------------------------------------------------------
# 미결제약정 수집
# ---------------------------------------------------------------------------
def get_open_interest_hist(
    symbol: str,
    period: str = "1h",
    limit: int = 500,
) -> pd.DataFrame:
    """
    바이낸스 선물 미결제약정(Open Interest) 히스토리를 수집합니다.

    Parameters
    ----------
    symbol : str
        바이낸스 선물 심볼 (예: "BTCUSDT").
    period : str
        집계 주기. 가능한 값: "5m", "15m", "30m", "1h", "2h", "4h", "6h", "12h", "1d".
        기본값 "1h".
    limit : int
        수집할 최대 레코드 수. 최대 500. 기본값 500.

    Returns
    -------
    pd.DataFrame
        컬럼: [symbol, timestamp, sumOpenInterest, sumOpenInterestValue]
        timestamp는 KST datetime입니다.

    Raises
    ------
    requests.HTTPError
        HTTP 4xx/5xx 응답 시 발생.
    """
    params = {"symbol": symbol, "period": period, "limit": limit}
    resp = requests.get(_OI_ENDPOINT, params=params, timeout=_TIMEOUT)
    resp.raise_for_status()

    raw: list[dict] = resp.json()
    if not raw:
        return pd.DataFrame(columns=["symbol", "timestamp", "sumOpenInterest", "sumOpenInterestValue"])

    df = pd.DataFrame(raw)

    # timestamp: Unix ms → KST datetime
    df["timestamp"] = pd.to_datetime(df["timestamp"], unit="ms", utc=True).dt.tz_convert(_KST)

    df["sumOpenInterest"] = df["sumOpenInterest"].astype(float)
    df["sumOpenInterestValue"] = df["sumOpenInterestValue"].astype(float)
    df["symbol"] = symbol

    return df[["symbol", "timestamp", "sumOpenInterest", "sumOpenInterestValue"]].reset_index(drop=True)


# ---------------------------------------------------------------------------
# 캐시 유효성 확인
# ---------------------------------------------------------------------------
def _is_cache_valid(path: Path) -> bool:
    """
    CSV 캐시 파일이 존재하고 1시간 이내에 수정되었으면 True 반환.

    Parameters
    ----------
    path : Path
        CSV 파일 경로.

    Returns
    -------
    bool
        캐시가 유효하면 True.
    """
    if not path.exists():
        return False

    mtime = path.stat().st_mtime
    age_seconds = time.time() - mtime
    return age_seconds < _CACHE_TTL_SECONDS


# ---------------------------------------------------------------------------
# 전체 알트코인 배치 수집
# ---------------------------------------------------------------------------
def fetch_all(
    upbit_tickers: list[str],
    force_refresh: bool = False,
) -> tuple[pd.DataFrame, pd.DataFrame]:
    """
    업비트 KRW 마켓 전체 티커를 대상으로 펀딩레이트 및 OI를 배치 수집합니다.

    바이낸스 선물에 미상장된 심볼(업비트 전용 잡코인)은 자동으로 스킵됩니다.
    수집 실패한 심볼은 경고 로그를 출력하고 계속 진행합니다.

    캐시 정책:
      - CSV가 이미 존재하고 1시간 이내이면 재수집 없이 캐시 반환.
      - force_refresh=True이면 캐시를 무시하고 전체 재수집.

    Parameters
    ----------
    upbit_tickers : list[str]
        업비트 KRW 마켓 티커 리스트 (예: ["KRW-BTC", "KRW-ETH", ...]).
    force_refresh : bool
        True이면 캐시를 무시하고 강제 재수집. 기본값 False.

    Returns
    -------
    tuple[pd.DataFrame, pd.DataFrame]
        (funding_df, oi_df)
        funding_df 컬럼: [symbol, fundingTime, fundingRate]
        oi_df 컬럼: [symbol, timestamp, sumOpenInterest, sumOpenInterestValue]
    """
    # 캐시 확인
    if not force_refresh and _is_cache_valid(_FUNDING_CSV) and _is_cache_valid(_OI_CSV):
        logger.info("캐시 유효 — CSV 파일 재사용 (force_refresh=False)")
        return pd.read_csv(_FUNDING_CSV), pd.read_csv(_OI_CSV)

    # 바이낸스 심볼로 매핑
    symbol_map: dict[str, str] = {}
    for ticker in upbit_tickers:
        binance_sym = map_upbit_to_binance(ticker)
        if binance_sym is not None:
            symbol_map[ticker] = binance_sym

    logger.info(f"총 {len(upbit_tickers)}개 티커 중 {len(symbol_map)}개 매핑 성공, 수집 시작")

    funding_frames: list[pd.DataFrame] = []
    oi_frames: list[pd.DataFrame] = []

    for upbit_ticker, binance_sym in symbol_map.items():
        # --- 펀딩레이트 ---
        try:
            df_fr = get_funding_rates(binance_sym)
            if not df_fr.empty:
                funding_frames.append(df_fr)
        except Exception as exc:
            logger.warning(f"[펀딩레이트] {binance_sym} 수집 실패: {exc}")

        time.sleep(_REQUEST_INTERVAL)

        # --- 미결제약정 ---
        try:
            df_oi = get_open_interest_hist(binance_sym)
            if not df_oi.empty:
                oi_frames.append(df_oi)
        except Exception as exc:
            logger.warning(f"[OI] {binance_sym} 수집 실패: {exc}")

        time.sleep(_REQUEST_INTERVAL)

    funding_df = pd.concat(funding_frames, ignore_index=True) if funding_frames else pd.DataFrame(
        columns=["symbol", "fundingTime", "fundingRate"]
    )
    oi_df = pd.concat(oi_frames, ignore_index=True) if oi_frames else pd.DataFrame(
        columns=["symbol", "timestamp", "sumOpenInterest", "sumOpenInterestValue"]
    )

    # CSV 저장
    _DATA_DIR.mkdir(parents=True, exist_ok=True)
    funding_df.to_csv(_FUNDING_CSV, index=False)
    oi_df.to_csv(_OI_CSV, index=False)
    logger.info(f"저장 완료: {_FUNDING_CSV} ({len(funding_df)}행), {_OI_CSV} ({len(oi_df)}행)")

    return funding_df, oi_df

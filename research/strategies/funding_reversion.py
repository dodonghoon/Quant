"""
Funding Rate Mean Reversion Strategy
======================================
바이낸스 선물 펀딩레이트의 과도한 이탈을 역추세 매매 신호로 변환합니다.

Signal Logic:
  - 168시간(7일) 롤링 평균 및 표준편차로 Z-스코어 산출.
  - Z-스코어를 tanh로 압축하여 [-1, 1] 정규화:
      signal_raw = -tanh(z * 0.5)
  - Long-only 제약: 음수 신호(Short) → 0으로 강제.
      signal = clip(signal_raw, 0, 1)
  - 음수 신호는 "진입 자제" 의미로 재해석하며, 청산 로직과 별도로 관리됩니다.

Regime Fit:
  - 펀딩레이트가 극단적으로 양수(롱 과열)인 환경에서 역추세 Long 진입.
  - 'high_risk' 및 'ranging' 레짐에서 유효성 높음.

데이터 소스:
  - research/data/binance_fetcher.py의 get_funding_rates() 사용.
"""

from __future__ import annotations

import numpy as np
import pandas as pd
from loguru import logger

from research.data.binance_fetcher import get_funding_rates, map_upbit_to_binance


class FundingReversionStrategy:
    """
    펀딩레이트 평균회귀 전략.

    Parameters
    ----------
    rolling_window : int
        Z-스코어 산출에 사용할 롤링 윈도우 크기 (단위: 펀딩레이트 기간 수).
        펀딩레이트는 8시간마다 정산되므로, 168 = 7일. 기본값 168.
    tanh_scale : float
        tanh 압축 강도. 클수록 극단값에 더 빠르게 포화됨. 기본값 0.5.
    """

    def __init__(
        self,
        rolling_window: int = 168,
        tanh_scale: float = 0.5,
    ) -> None:
        self.rolling_window = rolling_window
        self.tanh_scale = tanh_scale

    def generate_signal(self, funding_df: pd.DataFrame) -> pd.Series:
        """
        펀딩레이트 DataFrame으로부터 Long-only 역추세 신호를 생성합니다.

        Parameters
        ----------
        funding_df : pd.DataFrame
            컬럼: [symbol, fundingTime, fundingRate].
            단일 심볼 데이터여야 합니다.

        Returns
        -------
        pd.Series
            신호 시리즈 [0.0, 1.0]. 인덱스는 fundingTime.
            1.0 = 강한 Long 진입 신호.
            0.0 = 진입 자제.
        """
        if funding_df.empty:
            logger.warning("funding_df가 비어 있습니다. 빈 시리즈 반환.")
            return pd.Series(dtype=float, name="funding_reversion")

        df = funding_df.copy().sort_values("fundingTime").reset_index(drop=True)
        rates: pd.Series = df["fundingRate"].astype(float)

        rolling_mean = rates.rolling(window=self.rolling_window, min_periods=1).mean()
        rolling_std = rates.rolling(window=self.rolling_window, min_periods=1).std().fillna(1e-8)

        # Z-스코어: 현재 펀딩레이트가 최근 7일 대비 얼마나 이탈했는지
        z: pd.Series = (rates - rolling_mean) / (rolling_std + 1e-8)

        # 역추세 신호: 펀딩이 높을수록(롱 과열) 음의 raw signal → 0으로 강제
        signal_raw: pd.Series = -np.tanh(z * self.tanh_scale)

        # Long-only 제약: 음수(Short 신호) → 0으로 강제
        signal: pd.Series = signal_raw.clip(lower=0.0, upper=1.0)

        signal.index = df["fundingTime"]
        return signal.rename("funding_reversion")

    def generate_signal_for_ticker(
        self,
        upbit_ticker: str,
        limit: int = 500,
    ) -> pd.Series:
        """
        업비트 티커를 입력받아 바이낸스 API에서 데이터를 직접 수집 후 신호를 반환합니다.

        Parameters
        ----------
        upbit_ticker : str
            업비트 KRW 마켓 티커 (예: "KRW-BTC").
        limit : int
            펀딩레이트 히스토리 최대 수집 개수. 기본값 500.

        Returns
        -------
        pd.Series
            신호 시리즈 [0.0, 1.0]. 바이낸스 미상장 심볼이면 빈 시리즈 반환.
        """
        binance_sym = map_upbit_to_binance(upbit_ticker)
        if binance_sym is None:
            logger.warning(f"{upbit_ticker} → 바이낸스 매핑 실패. 빈 시리즈 반환.")
            return pd.Series(dtype=float, name="funding_reversion")

        try:
            funding_df = get_funding_rates(binance_sym, limit=limit)
        except Exception as exc:
            logger.error(f"{binance_sym} 펀딩레이트 수집 실패: {exc}")
            return pd.Series(dtype=float, name="funding_reversion")

        return self.generate_signal(funding_df)

"""
공적분 검정 모듈 — Engle-Granger & Johansen

기술문서 §3.2 "statsmodels: 공적분(Cointegration) 테스트" 구현.
Pairs Trading 대상 종목 선별 시 사용됩니다.

Rust Strategy Engine의 ou_model.rs와 연계:
- 공적분이 확인된 페어 → OU 프로세스 파라미터 추정 → 실시간 트레이딩

사용 예시:
    tester = CointegrationTester()
    result = tester.engle_granger(price_a, price_b)
    if result.is_cointegrated:
        print(f"Half-life: {result.half_life:.1f} bars")
"""

from __future__ import annotations

from dataclasses import dataclass

import numpy as np
import pandas as pd
import statsmodels.api as sm
from statsmodels.tsa.stattools import adfuller, coint
from loguru import logger


@dataclass
class CointegrationResult:
    """공적분 검정 결과."""
    # 검정 통계량
    test_statistic: float = 0.0
    p_value: float = 1.0
    critical_values: dict = None  # {"1%": ..., "5%": ..., "10%": ...}

    # 판정
    is_cointegrated: bool = False
    significance_level: str = "none"  # "1%", "5%", "10%", "none"

    # 페어 파라미터 (Rust ou_model.rs OuParams와 대응)
    hedge_ratio: float = 0.0       # β (OLS 회귀 계수)
    spread_mean: float = 0.0       # μ (OU long-term mean)
    spread_std: float = 0.0        # σ 관련
    half_life: float = float("inf")  # OU half-life (bars)
    kappa: float = 0.0             # OU mean reversion speed

    # 모델 적합도
    r_squared: float = 0.0

    def summary(self) -> str:
        status = "COINTEGRATED" if self.is_cointegrated else "NOT cointegrated"
        return (
            f"=== Cointegration Test ===\n"
            f"Status:          {status} ({self.significance_level})\n"
            f"Test Statistic:  {self.test_statistic:.4f}\n"
            f"P-value:         {self.p_value:.4f}\n"
            f"Hedge Ratio (β): {self.hedge_ratio:.6f}\n"
            f"Half-life:       {self.half_life:.1f} bars\n"
            f"Kappa (θ):       {self.kappa:.6f}\n"
            f"R²:              {self.r_squared:.4f}\n"
        )


@dataclass
class PairScanResult:
    """페어 스캔 결과."""
    pair: tuple[str, str]
    result: CointegrationResult


class CointegrationTester:
    """Engle-Granger 공적분 검정 + OU 파라미터 추정.

    Rust ou_model.rs와 동일한 수학적 기반:
    1. OLS 회귀 → 헤지 비율 (β)
    2. 잔차(스프레드) ADF 검정 → 공적분 여부
    3. OU 파라미터 추정: κ, μ, σ, half-life
    """

    def __init__(self, significance: float = 0.05) -> None:
        """
        Args:
            significance: 유의수준 (기본 5%)
        """
        self.significance = significance

    def engle_granger(
        self,
        price_a: pd.Series | np.ndarray,
        price_b: pd.Series | np.ndarray,
        dt: float = 1.0,
    ) -> CointegrationResult:
        """Engle-Granger 2단계 공적분 검정.

        Step 1: price_a = α + β × price_b + ε (OLS 회귀)
        Step 2: ADF test on residuals (ε)

        Args:
            price_a: 종속 변수 가격 시리즈
            price_b: 독립 변수 가격 시리즈
            dt: 시간 간격 (Rust ou_model.rs OuConfig.dt와 동일)

        Returns:
            CointegrationResult
        """
        a = np.asarray(price_a, dtype=np.float64)
        b = np.asarray(price_b, dtype=np.float64)

        if len(a) != len(b):
            raise ValueError("두 시리즈의 길이가 같아야 합니다")

        # ── Step 1: OLS 회귀 ──
        b_with_const = sm.add_constant(b)
        model = sm.OLS(a, b_with_const).fit()
        hedge_ratio = model.params[1]
        intercept = model.params[0]
        r_squared = model.rsquared

        # 스프레드 = A - β × B
        spread = a - hedge_ratio * b

        # ── Step 2: ADF 검정 on spread ──
        adf_result = adfuller(spread, maxlag=None, autolag="AIC")
        test_stat = adf_result[0]
        p_value = adf_result[1]
        critical_values = {
            f"{k}": v for k, v in adf_result[4].items()
        }

        # 유의수준 판정
        sig_level = "none"
        is_coint = False
        if p_value < 0.01:
            sig_level = "1%"
            is_coint = True
        elif p_value < 0.05:
            sig_level = "5%"
            is_coint = True
        elif p_value < 0.10:
            sig_level = "10%"
            is_coint = p_value < self.significance

        # ── OU 파라미터 추정 (Rust ou_model.rs와 동일) ──
        kappa, ou_mu, ou_sigma, half_life, ou_r2 = self._estimate_ou(spread, dt)

        return CointegrationResult(
            test_statistic=float(test_stat),
            p_value=float(p_value),
            critical_values=critical_values,
            is_cointegrated=is_coint,
            significance_level=sig_level,
            hedge_ratio=float(hedge_ratio),
            spread_mean=float(np.mean(spread)),
            spread_std=float(np.std(spread)),
            half_life=half_life,
            kappa=kappa,
            r_squared=ou_r2,
        )

    def _estimate_ou(
        self, spread: np.ndarray, dt: float = 1.0
    ) -> tuple[float, float, float, float, float]:
        """OU 프로세스 파라미터 추정.

        Rust ou_model.rs와 동일한 이산화 OLS:
            ΔX = a + b × X_{t-1} + ε
            κ = -b / dt
            μ = -a / b
            σ = std(ε) / √dt
            half_life = ln(2) / κ
        """
        x = spread[:-1]
        dx = np.diff(spread)

        # OLS: ΔX = a + b × X_{t-1}
        x_with_const = sm.add_constant(x)
        model = sm.OLS(dx, x_with_const).fit()

        a = model.params[0]
        b = model.params[1]
        residuals = model.resid
        r_squared = model.rsquared

        # OU 파라미터 변환
        if b >= 0:
            # 평균 회귀 아님 → 발산
            logger.warning("OU estimation: b >= 0 (no mean reversion)")
            return 0.0, float(np.mean(spread)), float(np.std(spread)), float("inf"), r_squared

        kappa = -b / dt
        mu = -a / b
        sigma = float(np.std(residuals)) / np.sqrt(dt)
        half_life = np.log(2) / kappa

        # Rust ou_model.rs 검증과 동일: min_kappa, max_half_life
        if kappa < 0.01:
            logger.warning(f"OU: kappa={kappa:.6f} too small (< 0.01)")
        if half_life > 86400:
            logger.warning(f"OU: half_life={half_life:.0f} exceeds 24h")

        return float(kappa), float(mu), float(sigma), float(half_life), float(r_squared)

    def scan_pairs(
        self,
        prices: dict[str, pd.Series],
        min_observations: int = 500,
    ) -> list[PairScanResult]:
        """여러 종목에서 공적분 페어를 자동 탐색.

        Args:
            prices: {심볼: 가격 시리즈} 딕셔너리
            min_observations: 최소 관측 수

        Returns:
            공적분이 확인된 페어 목록 (p-value 오름차순)
        """
        symbols = list(prices.keys())
        results: list[PairScanResult] = []
        n_pairs = len(symbols) * (len(symbols) - 1) // 2

        logger.info(f"Scanning {n_pairs} pairs from {len(symbols)} symbols")

        for i in range(len(symbols)):
            for j in range(i + 1, len(symbols)):
                sym_a, sym_b = symbols[i], symbols[j]
                pa, pb = prices[sym_a], prices[sym_b]

                # 공통 인덱스
                common = pa.dropna().index.intersection(pb.dropna().index)
                if len(common) < min_observations:
                    continue

                try:
                    result = self.engle_granger(
                        pa.loc[common].values,
                        pb.loc[common].values,
                    )
                    if result.is_cointegrated:
                        results.append(PairScanResult(
                            pair=(sym_a, sym_b),
                            result=result,
                        ))
                except Exception as e:
                    logger.debug(f"Pair ({sym_a}, {sym_b}) failed: {e}")

        results.sort(key=lambda r: r.result.p_value)
        logger.info(f"Found {len(results)} cointegrated pairs out of {n_pairs}")
        return results

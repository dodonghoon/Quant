"""
GARCH 변동성 모델링 — arch 패키지 기반

기술문서 §4.2 "GARCH 모델: 변동성 예측, 변동성 클러스터링 반영" 구현.
수익률 시계열의 조건부 분산을 모델링하여 리스크 관리에 활용합니다.

수학적 모델:
    r_t = μ + ε_t,  ε_t = σ_t × z_t,  z_t ~ N(0,1)

    GARCH(1,1):
    σ²_t = ω + α × ε²_{t-1} + β × σ²_{t-1}

    여기서:
    - ω (omega): 장기 분산 기여분
    - α (alpha): 직전 충격(뉴스) 반응 계수
    - β (beta):  이전 분산 지속 계수
    - α + β < 1: 정상성(stationarity) 조건

사용 예시:
    modeler = GarchModeler()
    result = modeler.fit(returns)
    forecast = modeler.forecast(horizon=5)
"""

from __future__ import annotations

from dataclasses import dataclass

import numpy as np
import pandas as pd
from arch import arch_model
from loguru import logger


@dataclass
class GarchParams:
    """GARCH(1,1) 추정 파라미터."""
    omega: float = 0.0      # ω: 장기 분산 기여
    alpha: float = 0.0      # α: ARCH 효과 (뉴스 반응)
    beta: float = 0.0       # β: GARCH 효과 (분산 지속)
    mu: float = 0.0         # 조건부 평균

    @property
    def persistence(self) -> float:
        """α + β: 변동성 지속성 (1에 가까울수록 높은 지속성)."""
        return self.alpha + self.beta

    @property
    def long_run_variance(self) -> float:
        """장기 무조건부 분산: ω / (1 - α - β)."""
        denom = 1.0 - self.persistence
        if denom <= 0:
            return float("inf")
        return self.omega / denom

    @property
    def long_run_volatility(self) -> float:
        """장기 무조건부 변동성 (연율화 전)."""
        return np.sqrt(self.long_run_variance)

    @property
    def half_life(self) -> float:
        """변동성 충격의 반감기 (bars).

        Rust ou_model.rs half-life 계산과 동일한 원리:
            half_life = ln(2) / ln(1 / (α + β))
        """
        if self.persistence <= 0 or self.persistence >= 1:
            return float("inf")
        return np.log(2) / np.log(1.0 / self.persistence)


@dataclass
class GarchResult:
    """GARCH 모델 적합 결과."""
    params: GarchParams
    aic: float = 0.0
    bic: float = 0.0
    log_likelihood: float = 0.0
    conditional_volatility: pd.Series = None  # σ_t 시리즈
    standardized_residuals: pd.Series = None  # z_t = ε_t / σ_t
    model_obj: object = None  # arch 모델 객체

    def summary(self) -> str:
        p = self.params
        return (
            f"=== GARCH(1,1) Result ===\n"
            f"ω (omega):       {p.omega:.8f}\n"
            f"α (alpha):       {p.alpha:.6f}\n"
            f"β (beta):        {p.beta:.6f}\n"
            f"Persistence:     {p.persistence:.6f}\n"
            f"Long-run Vol:    {p.long_run_volatility:.6f}\n"
            f"Half-life:       {p.half_life:.1f} bars\n"
            f"AIC:             {self.aic:.2f}\n"
            f"BIC:             {self.bic:.2f}\n"
        )


@dataclass
class VolatilityForecast:
    """변동성 예측 결과."""
    horizon: int
    variance_forecast: np.ndarray    # σ²_t+h
    volatility_forecast: np.ndarray  # σ_t+h
    annualized_vol: np.ndarray       # 연율화 변동성

    def to_dataframe(self) -> pd.DataFrame:
        return pd.DataFrame({
            "step": range(1, self.horizon + 1),
            "variance": self.variance_forecast,
            "volatility": self.volatility_forecast,
            "annualized_vol": self.annualized_vol,
        })


class GarchModeler:
    """GARCH(1,1) 변동성 모델러.

    기술문서 §4.2의 변동성 클러스터링 분석 및
    Rust risk.rs의 리스크 한도 설정에 필요한 변동성 추정치를 제공합니다.
    """

    def __init__(
        self,
        p: int = 1,
        q: int = 1,
        dist: str = "normal",
        rescale: bool = True,
    ) -> None:
        """
        Args:
            p: GARCH order (default 1)
            q: ARCH order (default 1)
            dist: 오차 분포 ("normal", "t", "skewt")
            rescale: 수익률 스케일링 (수렴 안정성)
        """
        self.p = p
        self.q = q
        self.dist = dist
        self.rescale = rescale
        self._model_result = None
        self._returns = None

    def fit(
        self,
        returns: pd.Series | np.ndarray,
        mean_model: str = "Constant",
    ) -> GarchResult:
        """GARCH 모델 적합.

        Args:
            returns: 수익률 시리즈 (로그 수익률 또는 단순 수익률)
            mean_model: 조건부 평균 모형 ("Constant", "Zero", "AR")

        Returns:
            GarchResult
        """
        if isinstance(returns, np.ndarray):
            returns = pd.Series(returns)

        self._returns = returns.dropna()
        n = len(self._returns)
        logger.info(f"GARCH({self.p},{self.q}) fitting on {n} observations")

        # arch 모델 생성
        am = arch_model(
            self._returns,
            mean=mean_model,
            vol="Garch",
            p=self.p,
            q=self.q,
            dist=self.dist,
            rescale=self.rescale,
        )

        res = am.fit(disp="off", show_warning=False)
        self._model_result = res

        # 파라미터 추출
        params = GarchParams(
            omega=float(res.params.get("omega", 0)),
            alpha=float(res.params.get("alpha[1]", 0)),
            beta=float(res.params.get("beta[1]", 0)),
            mu=float(res.params.get("mu", res.params.get("Const", 0))),
        )

        # rescale 보정
        if self.rescale and hasattr(res, "scale"):
            scale = res.scale
            if scale != 1.0:
                params.omega *= scale**2

        result = GarchResult(
            params=params,
            aic=float(res.aic),
            bic=float(res.bic),
            log_likelihood=float(res.loglikelihood),
            conditional_volatility=res.conditional_volatility,
            standardized_residuals=res.std_resid,
            model_obj=res,
        )

        logger.info(
            f"GARCH fit: α={params.alpha:.4f}, β={params.beta:.4f}, "
            f"persistence={params.persistence:.4f}"
        )
        return result

    def forecast(
        self,
        horizon: int = 5,
        annualize_factor: float = 252.0,
    ) -> VolatilityForecast:
        """다기간 변동성 예측.

        Args:
            horizon: 예측 기간 (bars)
            annualize_factor: 연율화 팩터 (일봉=252, 분봉=252*390)

        Returns:
            VolatilityForecast
        """
        if self._model_result is None:
            raise RuntimeError("먼저 fit()을 호출하세요")

        fcast = self._model_result.forecast(horizon=horizon)
        var_forecast = fcast.variance.iloc[-1].values  # 마지막 관측 기준

        vol_forecast = np.sqrt(var_forecast)
        annual_vol = vol_forecast * np.sqrt(annualize_factor)

        return VolatilityForecast(
            horizon=horizon,
            variance_forecast=var_forecast,
            volatility_forecast=vol_forecast,
            annualized_vol=annual_vol,
        )

    def rolling_forecast(
        self,
        returns: pd.Series,
        window: int = 500,
        step: int = 1,
    ) -> pd.DataFrame:
        """롤링 윈도우 1-step-ahead 변동성 예측.

        Rust features.rs RollingWindow와 유사한 슬라이딩 윈도우 패턴.

        Returns:
            DataFrame with columns: [actual_return, predicted_vol, realized_vol]
        """
        returns = returns.dropna()
        n = len(returns)
        results = []

        logger.info(f"Rolling GARCH forecast: window={window}, n={n}")

        for i in range(window, n, step):
            train = returns.iloc[i - window : i]
            try:
                self.fit(train)
                fcast = self.forecast(horizon=1)
                pred_vol = float(fcast.volatility_forecast[0])
            except Exception:
                pred_vol = np.nan

            results.append({
                "date": returns.index[i] if hasattr(returns.index, "__getitem__") else i,
                "actual_return": float(returns.iloc[i]),
                "predicted_vol": pred_vol,
            })

        df = pd.DataFrame(results)
        if not df.empty:
            df["realized_vol"] = df["actual_return"].rolling(20).std()

        return df

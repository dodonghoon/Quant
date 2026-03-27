"""
Vectorized Backtesting — vectorbt 기반 고속 백테스팅 엔진

기술문서 §3.2 "vectorbt: 벡터화된 고속 백테스팅 엔진" 구현.
Rust Strategy Engine (signal.rs)의 시그널 로직과 동일한 파라미터를
Python에서 프로토타이핑할 수 있도록 합니다.

사용 예시:
    bt = PairsBacktester(
        price_a=df_btc["close"],
        price_b=df_eth["close"],
    )
    result = bt.run()
    result.plot()
"""

from __future__ import annotations

from dataclasses import dataclass, field

import numpy as np
import pandas as pd
import vectorbt as vbt
from loguru import logger


# ─────────────────────────────────────────
# Rust signal.rs와 일치하는 상수
# ─────────────────────────────────────────
DEFAULT_ENTRY_Z = 1.5       # signal.rs: entry_threshold
DEFAULT_STRONG_Z = 2.5      # signal.rs: strong_entry_threshold
DEFAULT_EXIT_Z = 0.5        # signal.rs: exit_threshold
DEFAULT_OU_WEIGHT = 0.7     # signal.rs: ou_weight
DEFAULT_KALMAN_WEIGHT = 0.3  # signal.rs: kalman_weight


@dataclass
class BacktestConfig:
    """백테스트 설정.

    Rust SignalConfig와 동일한 파라미터 체계를 사용합니다.
    """
    # Z-score 진입/청산 임계값
    entry_z: float = DEFAULT_ENTRY_Z
    exit_z: float = DEFAULT_EXIT_Z
    strong_z: float = DEFAULT_STRONG_Z

    # 수수료 (편도, bps)
    commission_bps: float = 10.0  # 0.1%

    # 슬리피지 (편도, bps)
    slippage_bps: float = 5.0

    # OU 추정 윈도우 (Rust ou_model.rs: estimation_window = 500)
    lookback: int = 500

    # EMA 기간 (Rust features.rs)
    ema_fast: int = 10
    ema_slow: int = 20

    # Kelly fraction (Rust kelly.rs: default fractional_kelly = 0.25)
    kelly_fraction: float = 0.25

    # 초기 자본
    init_cash: float = 100_000.0


@dataclass
class BacktestResult:
    """백테스트 결과 요약."""
    total_return: float = 0.0
    sharpe_ratio: float = 0.0
    max_drawdown: float = 0.0
    win_rate: float = 0.0
    num_trades: int = 0
    profit_factor: float = 0.0
    annual_return: float = 0.0
    annual_volatility: float = 0.0

    # vectorbt Portfolio 객체 (상세 분석용)
    portfolio: object = None

    # 시그널 데이터
    spread: pd.Series = None
    z_score: pd.Series = None
    entries_long: pd.Series = None
    entries_short: pd.Series = None

    def plot(self) -> None:
        """수익률 곡선 및 시그널 시각화."""
        if self.portfolio is not None:
            self.portfolio.plot().show()

    def summary(self) -> str:
        """텍스트 요약."""
        return (
            f"=== Backtest Summary ===\n"
            f"Total Return:     {self.total_return:>10.2%}\n"
            f"Annual Return:    {self.annual_return:>10.2%}\n"
            f"Sharpe Ratio:     {self.sharpe_ratio:>10.2f}\n"
            f"Max Drawdown:     {self.max_drawdown:>10.2%}\n"
            f"Win Rate:         {self.win_rate:>10.2%}\n"
            f"Profit Factor:    {self.profit_factor:>10.2f}\n"
            f"Total Trades:     {self.num_trades:>10d}\n"
        )


class PairsBacktester:
    """Pairs Trading 벡터화 백테스터.

    Rust Strategy Engine의 파이프라인을 Python으로 재현합니다:
    1. 스프레드 계산 (A - ratio × B)
    2. Rolling Z-Score (features.rs RollingWindow와 동일 로직)
    3. Z-Score 기반 진입/청산 시그널 생성 (signal.rs 임계값)
    4. vectorbt를 통한 벡터화 시뮬레이션
    """

    def __init__(
        self,
        price_a: pd.Series,
        price_b: pd.Series,
        config: BacktestConfig | None = None,
    ) -> None:
        if len(price_a) != len(price_b):
            raise ValueError("price_a와 price_b의 길이가 같아야 합니다")

        self.price_a = price_a.astype(float)
        self.price_b = price_b.astype(float)
        self.config = config or BacktestConfig()
        logger.info(
            f"PairsBacktester: {len(price_a)} bars, "
            f"lookback={self.config.lookback}, "
            f"entry_z={self.config.entry_z}"
        )

    def _compute_spread(self) -> tuple[pd.Series, pd.Series]:
        """롤링 OLS로 헤지 비율 계산 후 스프레드 생성.

        Rust ou_model.rs의 OLS 추정과 동일한 방식:
            X_t - X_{t-1} = a + b * X_{t-1}
        """
        lookback = self.config.lookback

        # 롤링 OLS 헤지 비율: β = cov(A, B) / var(B)
        rolling_cov = self.price_a.rolling(lookback).cov(self.price_b)
        rolling_var = self.price_b.rolling(lookback).var()
        hedge_ratio = rolling_cov / rolling_var

        spread = self.price_a - hedge_ratio * self.price_b
        return spread, hedge_ratio

    def _compute_zscore(self, spread: pd.Series) -> pd.Series:
        """롤링 Z-Score 계산.

        Rust features.rs RollingWindow.z_score()와 동일:
            z = (last_value - mean) / std_dev
        """
        lookback = self.config.lookback
        mean = spread.rolling(lookback).mean()
        std = spread.rolling(lookback).std()
        z = (spread - mean) / std.replace(0, np.nan)
        return z

    def run(self) -> BacktestResult:
        """벡터화 백테스트 실행."""
        spread, hedge_ratio = self._compute_spread()
        z_score = self._compute_zscore(spread)

        # ── 시그널 생성 (signal.rs 임계값과 동일) ──
        entry_z = self.config.entry_z
        exit_z = self.config.exit_z

        # Long spread: z < -entry_z → 매수 진입, z > -exit_z → 청산
        entries_long = z_score < -entry_z
        exits_long = z_score > -exit_z

        # Short spread: z > entry_z → 매도 진입, z < exit_z → 청산
        entries_short = z_score > entry_z
        exits_short = z_score < exit_z

        # ── vectorbt 포트폴리오 시뮬레이션 ──
        commission = (self.config.commission_bps + self.config.slippage_bps) / 10_000
        close = spread.dropna()

        # NaN 제거 후 인덱스 정렬
        valid_idx = close.index
        el = entries_long.reindex(valid_idx).fillna(False)
        xl = exits_long.reindex(valid_idx).fillna(False)
        es = entries_short.reindex(valid_idx).fillna(False)
        xs = exits_short.reindex(valid_idx).fillna(False)

        # Long + Short 통합 시그널
        entries = el | es
        exits = xl | xs

        try:
            pf = vbt.Portfolio.from_signals(
                close=close,
                entries=el,
                exits=xl,
                short_entries=es,
                short_exits=xs,
                init_cash=self.config.init_cash,
                fees=commission,
                freq="1T",  # 1분 봉 기본
            )

            stats = pf.stats()
            trades = pf.trades.records_readable if hasattr(pf, "trades") else None
            n_trades = int(pf.trades.count()) if trades is not None else 0

            # 수익/손실 분리
            if trades is not None and n_trades > 0:
                pnls = pf.trades.pnl.values
                wins = pnls[pnls > 0]
                losses = pnls[pnls < 0]
                win_rate = len(wins) / n_trades if n_trades > 0 else 0.0
                profit_factor = (
                    abs(wins.sum() / losses.sum()) if len(losses) > 0 else float("inf")
                )
            else:
                win_rate = 0.0
                profit_factor = 0.0

            result = BacktestResult(
                total_return=float(pf.total_return()),
                sharpe_ratio=float(pf.sharpe_ratio()),
                max_drawdown=float(pf.max_drawdown()),
                win_rate=win_rate,
                num_trades=n_trades,
                profit_factor=profit_factor,
                annual_return=float(pf.annualized_return()),
                annual_volatility=float(pf.annualized_volatility()),
                portfolio=pf,
                spread=spread,
                z_score=z_score,
                entries_long=entries_long,
                entries_short=entries_short,
            )
        except Exception as e:
            logger.error(f"Backtest failed: {e}")
            result = BacktestResult(
                spread=spread,
                z_score=z_score,
                entries_long=entries_long,
                entries_short=entries_short,
            )

        logger.info(f"Backtest complete: {result.num_trades} trades")
        return result

    def optimize(
        self,
        entry_z_range: tuple[float, float, float] = (1.0, 3.0, 0.5),
        exit_z_range: tuple[float, float, float] = (0.0, 1.5, 0.5),
        lookback_range: tuple[int, int, int] = (200, 800, 200),
    ) -> pd.DataFrame:
        """그리드 서치로 최적 파라미터 탐색.

        Returns:
            파라미터 조합별 Sharpe Ratio 결과 DataFrame
        """
        results = []

        entry_vals = np.arange(*entry_z_range)
        exit_vals = np.arange(*exit_z_range)
        lookback_vals = range(*lookback_range)

        total = len(entry_vals) * len(exit_vals) * len(lookback_vals)
        logger.info(f"Optimization: {total} combinations")

        for lb in lookback_vals:
            for ez in entry_vals:
                for xz in exit_vals:
                    if xz >= ez:  # exit은 entry보다 작아야 함
                        continue
                    cfg = BacktestConfig(
                        entry_z=float(ez),
                        exit_z=float(xz),
                        lookback=lb,
                        init_cash=self.config.init_cash,
                        commission_bps=self.config.commission_bps,
                        slippage_bps=self.config.slippage_bps,
                    )
                    bt = PairsBacktester(self.price_a, self.price_b, config=cfg)
                    res = bt.run()
                    results.append({
                        "lookback": lb,
                        "entry_z": ez,
                        "exit_z": xz,
                        "sharpe": res.sharpe_ratio,
                        "return": res.total_return,
                        "max_dd": res.max_drawdown,
                        "trades": res.num_trades,
                    })

        df = pd.DataFrame(results).sort_values("sharpe", ascending=False)
        logger.info(f"Best params: {df.iloc[0].to_dict()}")
        return df

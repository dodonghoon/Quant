"""
Vectorized Backtesting Engine for Altcoin Strategy Ensemble
=============================================================
Tests the StrategyEnsemble against historical Upbit data.

Key Features
------------
- Fully vectorized using vectorbt for high performance.
- Accounts for Upbit's exact maker/taker fee (0.05%).
- Simulates realistic slippage for mid-cap altcoins (0.10% assumed).
- Supports per-bar regime weighting via a regime_series input.
- Outputs comprehensive performance statistics.

Fee & Slippage Model
--------------------
  Upbit Taker Fee : 0.05% per side (0.10% round-trip)
  Slippage Model  : 0.10% per side for mid-cap altcoins (e.g., ADA, DOT, ATOM)
                    0.05% per side for large-cap altcoins (e.g., XRP, SOL, DOGE)
  Total Cost      : fee + slippage applied symmetrically to both entry and exit

Usage
-----
  from research.backtesting.altcoin_engine import AltcoinBacktester
  bt = AltcoinBacktester(df_alt=df_doge, df_btc=df_btc, initial_cash=10_000_000)
  stats, pf = bt.run(regime_series=regime_series)
  print(stats)
"""

from __future__ import annotations

import logging
from dataclasses import dataclass
from typing import Optional

import numpy as np
import pandas as pd
import vectorbt as vbt

from research.strategies.ema_momentum import EMAMomentumStrategy
from research.strategies.bollinger_reversion import BollingerReversionStrategy
from research.strategies.atr_breakout import ATRBreakoutStrategy
from research.strategies.btc_divergence import BTCDivergenceStrategy
from research.strategies.ensemble import StrategyEnsemble

logger = logging.getLogger(__name__)

# ── Fee & Slippage Constants ───────────────────────────────────────────────

UPBIT_FEE_PCT = 0.0005          # 0.05% taker fee (per side)
SLIPPAGE_MID_CAP_PCT = 0.0010   # 0.10% slippage for mid-cap (ADA, DOT, ATOM)
SLIPPAGE_LARGE_CAP_PCT = 0.0005 # 0.05% slippage for large-cap (XRP, SOL, DOGE)

# ── Backtester Configuration ───────────────────────────────────────────────

@dataclass
class BacktestConfig:
    """Configuration for a single backtest run."""
    initial_cash: float = 10_000_000    # 10M KRW default
    fee_pct: float = UPBIT_FEE_PCT
    slippage_pct: float = SLIPPAGE_MID_CAP_PCT
    long_entry_threshold: float = 0.40   # Signal > this → enter long
    long_exit_threshold: float = 0.10    # Signal < this → exit long
    short_entry_threshold: float = -0.40 # Signal < this → enter short
    short_exit_threshold: float = -0.10  # Signal > this → exit short
    freq: str = "1T"                     # Bar frequency (1-minute default)


# ── Main Backtester ────────────────────────────────────────────────────────

class AltcoinBacktester:
    """
    Vectorized backtester for the Altcoin Strategy Ensemble.

    Parameters
    ----------
    df_alt : pd.DataFrame
        OHLCV data for the target altcoin. Index must be DatetimeIndex.
    df_btc : pd.DataFrame
        OHLCV data for BTC (macro reference). Must be aligned to df_alt's index.
    config : BacktestConfig, optional
        Backtesting configuration. Defaults to BacktestConfig().
    """

    def __init__(
        self,
        df_alt: pd.DataFrame,
        df_btc: pd.DataFrame,
        config: Optional[BacktestConfig] = None,
    ) -> None:
        self.df_alt = df_alt
        self.df_btc = df_btc
        self.config = config or BacktestConfig()

        # Total cost per side: fee + slippage
        self.total_cost_per_side = self.config.fee_pct + self.config.slippage_pct

    # ── Signal Generation ──────────────────────────────────────────────────

    def _generate_signals(self) -> pd.DataFrame:
        """Generate all four strategy signals and return as a DataFrame."""
        ema_sig = EMAMomentumStrategy().generate_signal(self.df_alt)
        boll_sig = BollingerReversionStrategy().generate_signal(self.df_alt)
        atr_sig = ATRBreakoutStrategy().generate_signal(self.df_alt)
        btc_div_sig = BTCDivergenceStrategy().generate_signal(self.df_alt, self.df_btc)

        return pd.DataFrame({
            "ema": ema_sig,
            "boll": boll_sig,
            "atr": atr_sig,
            "btc_div": btc_div_sig,
        })

    # ── Ensemble Aggregation ───────────────────────────────────────────────

    def _aggregate_ensemble(
        self,
        signals_df: pd.DataFrame,
        regime_series: Optional[pd.Series] = None,
    ) -> pd.Series:
        """Aggregate individual signals into a single ensemble signal."""
        ensemble = StrategyEnsemble()
        return ensemble.aggregate_series(
            ema=signals_df["ema"],
            boll=signals_df["boll"],
            atr=signals_df["atr"],
            btc_div=signals_df["btc_div"],
            regime_series=regime_series,
        )

    # ── Entry / Exit Logic ─────────────────────────────────────────────────

    def _build_entry_exit(self, ensemble_signal: pd.Series) -> dict[str, pd.Series]:
        """Convert ensemble signal to boolean entry/exit arrays."""
        cfg = self.config
        return {
            "entries":      ensemble_signal > cfg.long_entry_threshold,
            "exits":        ensemble_signal < cfg.long_exit_threshold,
            "short_entries": ensemble_signal < cfg.short_entry_threshold,
            "short_exits":  ensemble_signal > cfg.short_exit_threshold,
        }

    # ── Main Run ───────────────────────────────────────────────────────────

    def run(
        self,
        regime_series: Optional[pd.Series] = None,
    ) -> tuple[pd.Series, vbt.Portfolio]:
        """
        Execute the vectorized backtest.

        Parameters
        ----------
        regime_series : pd.Series, optional
            Series of regime strings (e.g., 'altseason', 'ranging') indexed
            like df_alt. If None, uses neutral equal weights.

        Returns
        -------
        stats : pd.Series
            Comprehensive performance statistics from vectorbt.
        pf : vbt.Portfolio
            The full portfolio object for further analysis.
        """
        logger.info("Generating strategy signals...")
        signals_df = self._generate_signals()

        logger.info("Aggregating ensemble signal...")
        ensemble_signal = self._aggregate_ensemble(signals_df, regime_series)

        logger.info("Building entry/exit arrays...")
        ee = self._build_entry_exit(ensemble_signal)

        close = self.df_alt["close"].dropna()
        valid_idx = close.index

        entries      = ee["entries"].reindex(valid_idx).fillna(False)
        exits        = ee["exits"].reindex(valid_idx).fillna(False)
        short_entries = ee["short_entries"].reindex(valid_idx).fillna(False)
        short_exits  = ee["short_exits"].reindex(valid_idx).fillna(False)

        logger.info("Running vectorbt portfolio simulation...")
        pf = vbt.Portfolio.from_signals(
            close=close,
            entries=entries,
            exits=exits,
            short_entries=short_entries,
            short_exits=short_exits,
            init_cash=self.config.initial_cash,
            fees=self.total_cost_per_side,
            freq=self.config.freq,
        )

        stats = pf.stats()
        logger.info(
            "Backtest complete | Trades: %d | Sharpe: %.3f | Max DD: %.2f%%",
            int(pf.trades.count()),
            float(pf.sharpe_ratio()),
            float(pf.max_drawdown()) * 100,
        )
        return stats, pf

    # ── Parameter Optimization ────────────────────────────────────────────

    def optimize_thresholds(
        self,
        regime_series: Optional[pd.Series] = None,
        entry_range: tuple[float, float, float] = (0.2, 0.7, 0.1),
        exit_range:  tuple[float, float, float] = (0.0, 0.3, 0.1),
    ) -> pd.DataFrame:
        """
        Grid search over entry/exit thresholds to maximize Sharpe Ratio.

        Parameters
        ----------
        regime_series : pd.Series, optional
            Regime series for ensemble weighting.
        entry_range : tuple
            (start, stop, step) for long entry threshold.
        exit_range : tuple
            (start, stop, step) for long exit threshold.

        Returns
        -------
        pd.DataFrame
            Results sorted by Sharpe Ratio descending.
        """
        signals_df = self._generate_signals()
        ensemble_signal = self._aggregate_ensemble(signals_df, regime_series)
        close = self.df_alt["close"].dropna()
        valid_idx = close.index

        results = []
        entry_vals = np.arange(*entry_range)
        exit_vals  = np.arange(*exit_range)

        total = len(entry_vals) * len(exit_vals)
        logger.info("Threshold optimization: %d combinations", total)

        for entry_t in entry_vals:
            for exit_t in exit_vals:
                if exit_t >= entry_t:
                    continue

                entries      = (ensemble_signal > entry_t).reindex(valid_idx).fillna(False)
                exits        = (ensemble_signal < exit_t).reindex(valid_idx).fillna(False)
                short_entries = (ensemble_signal < -entry_t).reindex(valid_idx).fillna(False)
                short_exits  = (ensemble_signal > -exit_t).reindex(valid_idx).fillna(False)

                try:
                    pf = vbt.Portfolio.from_signals(
                        close=close,
                        entries=entries,
                        exits=exits,
                        short_entries=short_entries,
                        short_exits=short_exits,
                        init_cash=self.config.initial_cash,
                        fees=self.total_cost_per_side,
                        freq=self.config.freq,
                    )
                    results.append({
                        "entry_threshold": round(entry_t, 2),
                        "exit_threshold":  round(exit_t, 2),
                        "sharpe":          round(float(pf.sharpe_ratio()), 4),
                        "total_return":    round(float(pf.total_return()), 4),
                        "max_drawdown":    round(float(pf.max_drawdown()), 4),
                        "num_trades":      int(pf.trades.count()),
                    })
                except Exception as exc:
                    logger.warning("Optimization run failed: %s", exc)

        df = pd.DataFrame(results).sort_values("sharpe", ascending=False)
        if not df.empty:
            logger.info("Best params: %s", df.iloc[0].to_dict())
        return df

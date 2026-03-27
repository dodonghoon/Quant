"""
Strategy Ensemble with Regime-Based Dynamic Weighting
=======================================================
Aggregates signals from the four deterministic strategies using weights
that are dynamically adjusted based on the macro market regime classified
by the LLM (Claude Haiku).

Mathematical Rationale for Regime Weights
------------------------------------------
The weights are derived from the expected alpha generation of each strategy
in a given market environment:

  - 'altseason': High momentum, altcoins broadly outperform BTC.
    → Weight EMA Momentum (0.40) and ATR Breakout (0.30) heavily.
      Mean reversion fails in strong trends. BTC Divergence is secondary.

  - 'btc_dominance': BTC rallies while altcoins bleed. Momentum is negative.
    → Weight BTC Divergence (0.60) to find the few altcoins with relative
      strength. Reduce all other weights significantly.

  - 'ranging': Sideways, choppy market. Momentum strategies generate whipsaws.
    → Weight Bollinger Reversion (0.60) heavily. Use BTC Divergence (0.20)
      as a secondary filter.

  - 'high_risk': High volatility, potential flash crashes, low liquidity.
    → Reduce all weights significantly (sum < 1.0 to reduce overall exposure).
      Lean slightly on mean reversion (0.40) as the safest strategy.

  - 'neutral': Default state when no regime signal is available or the signal
    has fully decayed. Equal weights across all four strategies (0.25 each).

Time Decay
----------
If the LLM regime signal is older than 2 hours (7200 seconds), the weights
exponentially decay towards the neutral equal-weight distribution.

The decay follows an exponential function with a half-life of 3600 seconds (1 hour):
  decay_factor = exp(-ln(2) * t / half_life)
  current_weights = target_weights * decay_factor + neutral_weights * (1 - decay_factor)

At t=0:    current_weights = target_weights  (full regime weight)
At t=3600: current_weights = midpoint between target and neutral
At t=7200: weights are forced to neutral (hard cutoff)
"""

import time
from typing import Optional

import numpy as np
import pandas as pd


# Regime weight definitions
# Order: [EMA Momentum, Bollinger Reversion, ATR Breakout, BTC Divergence]
REGIME_WEIGHTS: dict[str, np.ndarray] = {
    "altseason":     np.array([0.40, 0.10, 0.30, 0.20]),
    "btc_dominance": np.array([0.10, 0.20, 0.10, 0.60]),
    "ranging":       np.array([0.10, 0.60, 0.10, 0.20]),
    "high_risk":     np.array([0.05, 0.40, 0.05, 0.10]),  # sum=0.60, reduces exposure
    "neutral":       np.array([0.25, 0.25, 0.25, 0.25]),
}

DECAY_HARD_CUTOFF_SEC = 7200   # 2 hours: force neutral after this
DECAY_HALF_LIFE_SEC   = 3600   # 1 hour: half-life for exponential decay


class StrategyEnsemble:
    """
    Regime-Aware Strategy Ensemble.

    Aggregates signals from EMAMomentum, BollingerReversion, ATRBreakout,
    and BTCDivergence strategies using dynamically adjusted weights.

    Parameters
    ----------
    decay_half_life : int
        Half-life in seconds for the exponential weight decay. Default 3600.
    hard_cutoff : int
        Seconds after which weights are forced to neutral. Default 7200.
    """

    def __init__(
        self,
        decay_half_life: int = DECAY_HALF_LIFE_SEC,
        hard_cutoff: int = DECAY_HARD_CUTOFF_SEC,
    ) -> None:
        self.regime_weights = REGIME_WEIGHTS
        self.decay_half_life = decay_half_life
        self.hard_cutoff = hard_cutoff

        self._current_regime: str = "neutral"
        self._last_regime_update: float = 0.0

    # ── Regime Management ──────────────────────────────────────────────────

    def update_regime(self, regime: str, timestamp: Optional[float] = None) -> None:
        """
        Update the current macro regime from the LLM output.

        Parameters
        ----------
        regime : str
            One of: 'altseason', 'btc_dominance', 'ranging', 'high_risk', 'neutral'.
        timestamp : float, optional
            Unix timestamp of the regime classification. Defaults to now.
        """
        if regime not in self.regime_weights:
            raise ValueError(
                f"Unknown regime '{regime}'. Valid: {list(self.regime_weights.keys())}"
            )
        self._current_regime = regime
        self._last_regime_update = timestamp if timestamp is not None else time.time()

    # ── Weight Calculation ─────────────────────────────────────────────────

    def get_current_weights(self, current_time: Optional[float] = None) -> np.ndarray:
        """
        Compute the current strategy weights with time-decay applied.

        Parameters
        ----------
        current_time : float, optional
            Unix timestamp for weight calculation. Defaults to now.

        Returns
        -------
        np.ndarray
            Weight array of shape (4,) summing to ≤ 1.0.
        """
        if current_time is None:
            current_time = time.time()

        target_weights = self.regime_weights[self._current_regime]
        neutral_weights = self.regime_weights["neutral"]

        time_elapsed = current_time - self._last_regime_update

        # Hard cutoff: force neutral after 2 hours
        if time_elapsed >= self.hard_cutoff:
            return neutral_weights.copy()

        # Exponential decay towards neutral:
        # decay_factor = exp(-ln(2) * t / half_life)
        decay_factor = np.exp(-np.log(2) * time_elapsed / self.decay_half_life)

        current_weights = (target_weights * decay_factor) + (
            neutral_weights * (1.0 - decay_factor)
        )
        return current_weights

    # ── Signal Aggregation ─────────────────────────────────────────────────

    def aggregate_signals(
        self,
        ema_signal: float,
        boll_signal: float,
        atr_signal: float,
        btc_div_signal: float,
        current_time: Optional[float] = None,
    ) -> float:
        """
        Aggregate individual strategy signals into a single ensemble signal.

        Parameters
        ----------
        ema_signal : float
            EMA Momentum signal in [-1.0, 1.0].
        boll_signal : float
            Bollinger Reversion signal in [-1.0, 1.0].
        atr_signal : float
            ATR Breakout signal in [-1.0, 1.0].
        btc_div_signal : float
            BTC Divergence signal in [-1.0, 1.0].
        current_time : float, optional
            Unix timestamp for weight calculation. Defaults to now.

        Returns
        -------
        float
            Final ensemble signal in [-1.0, 1.0].
        """
        signals = np.array([ema_signal, boll_signal, atr_signal, btc_div_signal])
        weights = self.get_current_weights(current_time)

        # Weighted sum (dot product)
        final_signal = float(np.dot(signals, weights))

        # Strict normalization
        return float(np.clip(final_signal, -1.0, 1.0))

    def aggregate_series(
        self,
        ema: pd.Series,
        boll: pd.Series,
        atr: pd.Series,
        btc_div: pd.Series,
        regime_series: Optional[pd.Series] = None,
    ) -> pd.Series:
        """
        Vectorized aggregation for backtesting.

        Parameters
        ----------
        ema, boll, atr, btc_div : pd.Series
            Individual strategy signal series, all sharing the same index.
        regime_series : pd.Series, optional
            Series of regime strings indexed like the signal series.
            If None, uses the current regime with no decay.

        Returns
        -------
        pd.Series
            Ensemble signal series in [-1.0, 1.0].
        """
        result = pd.Series(0.0, index=ema.index)

        if regime_series is None:
            # Use current weights for all bars
            weights = self.get_current_weights()
            result = (
                ema * weights[0]
                + boll * weights[1]
                + atr * weights[2]
                + btc_div * weights[3]
            )
        else:
            # Apply per-regime weights vectorially
            for regime, weights in self.regime_weights.items():
                mask = regime_series == regime
                if mask.any():
                    result.loc[mask] = (
                        ema.loc[mask] * weights[0]
                        + boll.loc[mask] * weights[1]
                        + atr.loc[mask] * weights[2]
                        + btc_div.loc[mask] * weights[3]
                    )

        return result.clip(-1.0, 1.0).rename("ensemble_signal")

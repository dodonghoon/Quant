"""
EMA Momentum Strategy
======================
Volatility-adjusted momentum using EMA crossover, normalized via tanh.

Signal Logic:
  - Compute short-term and long-term EMA difference (MACD-like).
  - Divide by ATR to normalize for current volatility regime.
  - Apply tanh to bound the signal to [-1.0, 1.0].

Regime Fit:
  - Best in 'altseason' and trending markets.
  - Underperforms in 'ranging' or 'high_risk' regimes.
"""

import numpy as np
import pandas as pd
import pandas_ta as ta


class EMAMomentumStrategy:
    """
    Volatility-Adjusted EMA Momentum Strategy.

    Parameters
    ----------
    short_window : int
        Period for the short-term EMA. Default 12.
    long_window : int
        Period for the long-term EMA. Default 26.
    atr_window : int
        Period for the ATR used in volatility normalization. Default 14.
    signal_scaling : float
        Scaling factor for tanh normalization. Higher values make the signal
        less sensitive (slower to reach ±1.0). Default 2.0.
    """

    def __init__(
        self,
        short_window: int = 12,
        long_window: int = 26,
        atr_window: int = 14,
        signal_scaling: float = 2.0,
    ) -> None:
        self.short_window = short_window
        self.long_window = long_window
        self.atr_window = atr_window
        self.signal_scaling = signal_scaling

    def generate_signal(self, df: pd.DataFrame) -> pd.Series:
        """
        Generate a normalized momentum signal.

        Parameters
        ----------
        df : pd.DataFrame
            OHLCV DataFrame with columns ['open', 'high', 'low', 'close', 'volume'].

        Returns
        -------
        pd.Series
            Signal series in [-1.0, 1.0]. Index matches df.index.
        """
        ema_short = ta.ema(df["close"], length=self.short_window)
        ema_long = ta.ema(df["close"], length=self.long_window)
        atr = ta.atr(df["high"], df["low"], df["close"], length=self.atr_window)

        # Raw momentum: EMA crossover difference
        macd_raw = ema_short - ema_long

        # Volatility-adjusted momentum: normalize by ATR
        # This prevents over-signaling during high-volatility spikes
        vol_adj_momentum = macd_raw / (atr + 1e-8)

        # Normalize to [-1.0, 1.0] using tanh
        # tanh(x) ≈ x for small x, saturates at ±1 for large x
        signal = np.tanh(vol_adj_momentum / self.signal_scaling)

        return signal.fillna(0.0).rename("ema_momentum")

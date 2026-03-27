"""
ATR Breakout Strategy
======================
Identifies strong directional moves that exceed a multiple of the ATR,
confirming a genuine breakout rather than noise.

Signal Logic:
  - Track the highest high and lowest low over a lookback period.
  - If the current close exceeds the prior high by more than (ATR * multiplier),
    generate a positive (long) signal proportional to the excess.
  - If the current close falls below the prior low by more than (ATR * multiplier),
    generate a negative (short) signal.
  - Apply tanh to bound the signal to [-1.0, 1.0].

Regime Fit:
  - Best in 'altseason' and strong trending regimes.
  - Generates many false signals in 'ranging' regimes.
"""

import numpy as np
import pandas as pd
import pandas_ta as ta


class ATRBreakoutStrategy:
    """
    ATR-Confirmed Breakout Strategy.

    Parameters
    ----------
    lookback : int
        Rolling window for highest high / lowest low calculation. Default 20.
    atr_window : int
        Period for the ATR. Default 14.
    breakout_multiplier : float
        Minimum ATR multiple required to confirm a breakout. Default 1.5.
    """

    def __init__(
        self,
        lookback: int = 20,
        atr_window: int = 14,
        breakout_multiplier: float = 1.5,
    ) -> None:
        self.lookback = lookback
        self.atr_window = atr_window
        self.breakout_multiplier = breakout_multiplier

    def generate_signal(self, df: pd.DataFrame) -> pd.Series:
        """
        Generate a normalized breakout signal.

        Parameters
        ----------
        df : pd.DataFrame
            OHLCV DataFrame with columns ['open', 'high', 'low', 'close', 'volume'].

        Returns
        -------
        pd.Series
            Signal series in [-1.0, 1.0]. Index matches df.index.
            Positive signal = confirmed upside breakout.
            Negative signal = confirmed downside breakout.
        """
        # Use prior-bar values to avoid look-ahead bias
        highest_high = df["high"].rolling(window=self.lookback).max().shift(1)
        lowest_low = df["low"].rolling(window=self.lookback).min().shift(1)
        atr = ta.atr(df["high"], df["low"], df["close"], length=self.atr_window).shift(1)

        threshold = atr * self.breakout_multiplier

        # Distance above the breakout level
        up_excess = df["close"] - highest_high - threshold
        down_excess = lowest_low - threshold - df["close"]

        signal = pd.Series(0.0, index=df.index)

        # Upside breakout: scale excess by ATR, then apply tanh
        up_mask = up_excess > 0
        signal.loc[up_mask] = np.tanh(up_excess.loc[up_mask] / (atr.loc[up_mask] + 1e-8))

        # Downside breakout: scale excess by ATR, then apply tanh (negative)
        down_mask = down_excess > 0
        signal.loc[down_mask] = -np.tanh(
            down_excess.loc[down_mask] / (atr.loc[down_mask] + 1e-8)
        )

        return signal.fillna(0.0).rename("atr_breakout")

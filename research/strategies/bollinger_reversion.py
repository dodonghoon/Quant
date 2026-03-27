"""
Bollinger Band Mean Reversion Strategy
========================================
Dynamically adjusts the Bollinger Band width multiplier based on recent ATR
percentile, rather than using a static 2.0 multiplier.

Signal Logic:
  - Compute SMA and rolling std.
  - Scale the band multiplier by (current ATR / ATR moving average).
  - Measure the price's distance from SMA relative to the dynamic band width.
  - Invert the signal (contrarian): price above upper band → short signal.

Regime Fit:
  - Best in 'ranging' and 'high_risk' regimes.
  - Underperforms in strong trending 'altseason' regimes.
"""

import numpy as np
import pandas as pd
import pandas_ta as ta


class BollingerReversionStrategy:
    """
    Dynamic Bollinger Band Mean Reversion Strategy.

    Parameters
    ----------
    window : int
        Rolling window for SMA and standard deviation. Default 20.
    base_std : float
        Base standard deviation multiplier for band width. Default 2.0.
    atr_window : int
        Period for the ATR used in dynamic multiplier calculation. Default 14.
    atr_ma_window : int
        Period for the ATR moving average used to compute the relative ATR. Default 50.
    multiplier_min : float
        Minimum allowed dynamic multiplier. Default 1.5.
    multiplier_max : float
        Maximum allowed dynamic multiplier. Default 3.5.
    """

    def __init__(
        self,
        window: int = 20,
        base_std: float = 2.0,
        atr_window: int = 14,
        atr_ma_window: int = 50,
        multiplier_min: float = 1.5,
        multiplier_max: float = 3.5,
    ) -> None:
        self.window = window
        self.base_std = base_std
        self.atr_window = atr_window
        self.atr_ma_window = atr_ma_window
        self.multiplier_min = multiplier_min
        self.multiplier_max = multiplier_max

    def generate_signal(self, df: pd.DataFrame) -> pd.Series:
        """
        Generate a normalized mean-reversion signal.

        Parameters
        ----------
        df : pd.DataFrame
            OHLCV DataFrame with columns ['open', 'high', 'low', 'close', 'volume'].

        Returns
        -------
        pd.Series
            Signal series in [-1.0, 1.0]. Index matches df.index.
            Positive signal = price below lower band (buy dip).
            Negative signal = price above upper band (sell rally).
        """
        sma = ta.sma(df["close"], length=self.window)
        rolling_std = df["close"].rolling(window=self.window).std()
        atr = ta.atr(df["high"], df["low"], df["close"], length=self.atr_window)
        atr_ma = ta.sma(atr, length=self.atr_ma_window)

        # Dynamic multiplier: scale base_std by the ratio of current ATR
        # to its moving average. High vol → wider bands → fewer false signals.
        dynamic_multiplier = self.base_std * (atr / (atr_ma + 1e-8))
        dynamic_multiplier = np.clip(
            dynamic_multiplier, self.multiplier_min, self.multiplier_max
        )

        # Compute dynamic bands
        upper_band = sma + (dynamic_multiplier * rolling_std)
        band_width = upper_band - sma  # = dynamic_multiplier * rolling_std

        # Normalized distance from SMA: +1.0 at upper band, -1.0 at lower band
        z_dist = (df["close"] - sma) / (band_width + 1e-8)

        # Contrarian: invert the z-score
        raw_signal = -z_dist

        # Strict normalization
        signal = np.clip(raw_signal, -1.0, 1.0)

        return signal.fillna(0.0).rename("bollinger_reversion")

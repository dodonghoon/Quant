"""
BTC-Divergence Strategy
========================
Compares the altcoin's return to BTC's return to find relative strength
or weakness. BTC and ETH are strictly used as macro references and are
never traded directly.

Signal Logic:
  - Compute the rolling return spread: ret_alt - ret_btc.
  - Calculate the rolling Z-score of this spread.
  - Apply tanh to normalize to [-1.0, 1.0].
  - Positive signal: altcoin outperforming BTC (relative strength).
  - Negative signal: altcoin underperforming BTC (relative weakness).

Regime Fit:
  - Best in 'btc_dominance' regime to identify altcoins with hidden strength.
  - Also useful in 'altseason' to rank altcoins by relative momentum.
"""

import numpy as np
import pandas as pd


class BTCDivergenceStrategy:
    """
    BTC-Relative Return Divergence Strategy.

    Parameters
    ----------
    window : int
        Rolling window for Z-score calculation. Default 20.
    zscore_scaling : float
        Scaling factor for tanh normalization of the Z-score. Default 2.0.
    """

    def __init__(self, window: int = 20, zscore_scaling: float = 2.0) -> None:
        self.window = window
        self.zscore_scaling = zscore_scaling

    def generate_signal(
        self, df_alt: pd.DataFrame, df_btc: pd.DataFrame
    ) -> pd.Series:
        """
        Generate a normalized BTC-divergence signal.

        Parameters
        ----------
        df_alt : pd.DataFrame
            OHLCV DataFrame for the target altcoin.
        df_btc : pd.DataFrame
            OHLCV DataFrame for BTC (macro reference, never traded).
            Must be aligned to the same index as df_alt.

        Returns
        -------
        pd.Series
            Signal series in [-1.0, 1.0]. Index matches df_alt.index.
            Positive signal = altcoin outperforming BTC.
            Negative signal = altcoin underperforming BTC.
        """
        ret_alt = df_alt["close"].pct_change()
        ret_btc = df_btc["close"].reindex(df_alt.index).pct_change()

        # Return spread: positive when altcoin outperforms BTC
        spread = ret_alt - ret_btc

        # Rolling Z-score of the spread
        spread_mean = spread.rolling(window=self.window).mean()
        spread_std = spread.rolling(window=self.window).std()
        z_score = (spread - spread_mean) / (spread_std + 1e-8)

        # Normalize to [-1.0, 1.0]
        signal = np.tanh(z_score / self.zscore_scaling)

        return signal.fillna(0.0).rename("btc_divergence")

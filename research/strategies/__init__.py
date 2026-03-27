"""
Altcoin Quantitative Trading Strategies
========================================
Target Assets: XRP, SOL, ADA, DOGE, AVAX, LINK, DOT, ATOM (Upbit KRW pairs)
Macro Reference: BTC, ETH (regime classification only, never traded)

All strategies output a signal in the range [-1.0, 1.0].
  +1.0 = Maximum Long
  -1.0 = Maximum Short
   0.0 = No Position
"""

from .ema_momentum import EMAMomentumStrategy
from .bollinger_reversion import BollingerReversionStrategy
from .atr_breakout import ATRBreakoutStrategy
from .btc_divergence import BTCDivergenceStrategy
from .ensemble import StrategyEnsemble

__all__ = [
    "EMAMomentumStrategy",
    "BollingerReversionStrategy",
    "ATRBreakoutStrategy",
    "BTCDivergenceStrategy",
    "StrategyEnsemble",
]

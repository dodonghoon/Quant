# Final Implementation Blueprint: Upbit Altcoin Quantitative Trading System

This document serves as the definitive implementation blueprint for the junior coding agent (Claude Code) to execute the mathematical refinement, ensemble weighting, risk management, and backtesting pipeline for the Upbit Altcoin Quantitative Trading System.

## 1. Task A: Mathematical Refinement of Altcoin Strategies

The following four strategies have been mathematically refined to incorporate dynamic volatility adjustments and strict signal normalization to the `[-1.0, 1.0]` range.

### 1.1. EMA Momentum Strategy
**Logic:** Uses the difference between short-term and long-term EMAs, normalized by the asset's recent volatility (ATR) to prevent over-signaling during high-volatility spikes.
**Normalization:** Uses the hyperbolic tangent function (`tanh`) to smoothly bound the signal between -1.0 and 1.0.

```python
# research/strategies/ema_momentum.py
import pandas as pd
import pandas_ta as ta
import numpy as np

class EMAMomentumStrategy:
    def __init__(self, short_window=12, long_window=26, atr_window=14, signal_scaling=2.0):
        self.short_window = short_window
        self.long_window = long_window
        self.atr_window = atr_window
        self.signal_scaling = signal_scaling

    def generate_signal(self, df: pd.DataFrame) -> pd.Series:
        """
        Generates a normalized momentum signal [-1.0, 1.0].
        """
        ema_short = ta.ema(df['close'], length=self.short_window)
        ema_long = ta.ema(df['close'], length=self.long_window)
        atr = ta.atr(df['high'], df['low'], df['close'], length=self.atr_window)
        
        # Raw momentum: difference between EMAs
        macd_raw = ema_short - ema_long
        
        # Volatility-adjusted momentum (Z-score like)
        # Avoid division by zero
        vol_adj_momentum = macd_raw / (atr + 1e-8)
        
        # Normalize to [-1.0, 1.0] using tanh
        # signal_scaling controls how fast the signal reaches extremes
        signal = np.tanh(vol_adj_momentum / self.signal_scaling)
        
        return signal.fillna(0.0)
```

### 1.2. Bollinger Mean Reversion Strategy
**Logic:** Dynamically adjusts the Bollinger Band width multiplier based on recent ATR percentile. During high volatility, bands widen to prevent premature mean-reversion entries.
**Normalization:** Calculates the distance of the price from the moving average relative to the dynamic band width, then bounds it using `np.clip`.

```python
# research/strategies/bollinger_reversion.py
import pandas as pd
import pandas_ta as ta
import numpy as np

class BollingerReversionStrategy:
    def __init__(self, window=20, base_std=2.0, atr_window=14):
        self.window = window
        self.base_std = base_std
        self.atr_window = atr_window

    def generate_signal(self, df: pd.DataFrame) -> pd.Series:
        """
        Generates a normalized mean-reversion signal [-1.0, 1.0].
        Contrarian: Price > Upper Band -> Negative Signal (Short)
        """
        sma = ta.sma(df['close'], length=self.window)
        std = df['close'].rolling(window=self.window).std()
        atr = ta.atr(df['high'], df['low'], df['close'], length=self.atr_window)
        
        # Dynamic multiplier: scale base_std by the ratio of current ATR to its 50-period moving average
        atr_ma = ta.sma(atr, length=50)
        dynamic_multiplier = self.base_std * (atr / (atr_ma + 1e-8))
        # Cap the multiplier to avoid extreme bands
        dynamic_multiplier = np.clip(dynamic_multiplier, 1.5, 3.5)
        
        upper_band = sma + (dynamic_multiplier * std)
        lower_band = sma - (dynamic_multiplier * std)
        
        # Distance from SMA normalized by the dynamic band width
        # If price == upper_band, z_dist == 1.0
        # If price == lower_band, z_dist == -1.0
        band_width = upper_band - sma
        z_dist = (df['close'] - sma) / (band_width + 1e-8)
        
        # Mean reversion implies taking the opposite direction of the breakout
        raw_signal = -z_dist
        
        # Strict normalization
        signal = np.clip(raw_signal, -1.0, 1.0)
        
        return signal.fillna(0.0)
```

### 1.3. ATR Breakout Strategy
**Logic:** Identifies strong directional moves that exceed a multiple of the ATR.
**Normalization:** Scales the breakout magnitude and applies `tanh`.

```python
# research/strategies/atr_breakout.py
import pandas as pd
import pandas_ta as ta
import numpy as np

class ATRBreakoutStrategy:
    def __init__(self, lookback=20, atr_window=14, breakout_multiplier=1.5):
        self.lookback = lookback
        self.atr_window = atr_window
        self.breakout_multiplier = breakout_multiplier

    def generate_signal(self, df: pd.DataFrame) -> pd.Series:
        highest_high = df['high'].rolling(window=self.lookback).max().shift(1)
        lowest_low = df['low'].rolling(window=self.lookback).min().shift(1)
        atr = ta.atr(df['high'], df['low'], df['close'], length=self.atr_window).shift(1)
        
        # Calculate breakout distance
        up_break = df['close'] - highest_high
        down_break = lowest_low - df['close']
        
        # Signal strength based on how much it exceeds the ATR threshold
        # Positive for upside breakout, negative for downside
        signal = pd.Series(0.0, index=df.index)
        
        # Upside breakout
        up_condition = up_break > (atr * self.breakout_multiplier)
        signal.loc[up_condition] = np.tanh((up_break.loc[up_condition] - atr.loc[up_condition] * self.breakout_multiplier) / atr.loc[up_condition])
        
        # Downside breakout
        down_condition = down_break > (atr * self.breakout_multiplier)
        signal.loc[down_condition] = -np.tanh((down_break.loc[down_condition] - atr.loc[down_condition] * self.breakout_multiplier) / atr.loc[down_condition])
        
        return signal.fillna(0.0)
```

### 1.4. BTC-Divergence Strategy
**Logic:** Compares the altcoin's return to BTC's return. If the altcoin is showing relative strength while BTC is weak, it generates a positive signal.
**Normalization:** Z-score of the return spread, bounded by `tanh`.

```python
# research/strategies/btc_divergence.py
import pandas as pd
import numpy as np

class BTCDivergenceStrategy:
    def __init__(self, window=20):
        self.window = window

    def generate_signal(self, df_alt: pd.DataFrame, df_btc: pd.DataFrame) -> pd.Series:
        """
        Requires both Altcoin and BTC dataframes aligned by index.
        """
        ret_alt = df_alt['close'].pct_change()
        ret_btc = df_btc['close'].pct_change()
        
        # Spread of returns
        spread = ret_alt - ret_btc
        
        # Rolling Z-score of the spread
        spread_mean = spread.rolling(window=self.window).mean()
        spread_std = spread.rolling(window=self.window).std()
        
        z_score = (spread - spread_mean) / (spread_std + 1e-8)
        
        # Normalize
        signal = np.tanh(z_score / 2.0)
        
        return signal.fillna(0.0)
```

## 2. Task B: Regime-Based Weighting Optimization (Ensemble Logic)

The `StrategyEnsemble` dynamically adjusts the weights of the four strategies based on the macro regime classified by the LLM.

**Mathematical Rationale:**
- **Altseason:** High momentum, altcoins outperform BTC. Weight heavily towards `EMAMomentum` and `ATRBreakout`.
- **BTC Dominance:** Altcoins bleed against BTC. Rely on `BTCDivergence` to find relative strength, reduce momentum weights.
- **Ranging:** Choppy, sideways market. Momentum fails here. Weight heavily towards `BollingerReversion`.
- **High Risk:** High volatility, potential flash crashes. Reduce all weights, rely slightly on mean reversion.

**Time Decay:** If the LLM regime signal is older than 2 hours (7200 seconds), the weights exponentially decay towards a neutral, equal-weight distribution `[0.25, 0.25, 0.25, 0.25]`.

```python
# research/strategies/ensemble.py
import time
import numpy as np
import pandas as pd

class StrategyEnsemble:
    def __init__(self):
        # Base weights for different regimes
        # Order: [EMA Momentum, Bollinger Reversion, ATR Breakout, BTC Divergence]
        self.regime_weights = {
            'altseason': np.array([0.40, 0.10, 0.30, 0.20]),
            'btc_dominance': np.array([0.10, 0.20, 0.10, 0.60]),
            'ranging': np.array([0.10, 0.60, 0.10, 0.20]),
            'high_risk': np.array([0.05, 0.40, 0.05, 0.10]), # Sum < 1.0 to reduce overall exposure
            'neutral': np.array([0.25, 0.25, 0.25, 0.25])
        }
        self.last_regime_update = 0
        self.current_regime = 'neutral'
        self.decay_half_life = 3600  # 1 hour half-life for decay

    def update_regime(self, regime: str, timestamp: float):
        if regime in self.regime_weights:
            self.current_regime = regime
            self.last_regime_update = timestamp

    def get_current_weights(self, current_time: float) -> np.ndarray:
        target_weights = self.regime_weights.get(self.current_regime, self.regime_weights['neutral'])
        neutral_weights = self.regime_weights['neutral']
        
        time_elapsed = current_time - self.last_regime_update
        
        # If older than 2 hours (7200s), force neutral
        if time_elapsed > 7200:
            return neutral_weights
            
        # Exponential decay towards neutral weights
        # decay_factor is 1.0 when time_elapsed is 0, approaches 0 as time passes
        decay_factor = np.exp(-np.log(2) * time_elapsed / self.decay_half_life)
        
        current_weights = (target_weights * decay_factor) + (neutral_weights * (1 - decay_factor))
        return current_weights

    def aggregate_signals(self, signals: list[float], current_time: float) -> float:
        """
        signals: [ema_sig, boll_sig, atr_sig, btc_div_sig]
        Returns final ensemble signal [-1.0, 1.0]
        """
        weights = self.get_current_weights(current_time)
        
        # Dot product of signals and weights
        final_signal = np.dot(np.array(signals), weights)
        
        # Ensure strict bounds
        return float(np.clip(final_signal, -1.0, 1.0))
```

## 3. Task C: Advanced Risk Management & Position Sizing

### 3.1. Dynamic Position Sizing Logic
Instead of a static KRW size, the order size is determined by:
1. **Account Balance:** Total available KRW.
2. **Signal Strength:** The absolute value of the ensemble signal `|S| \in [0, 1]`.
3. **Volatility Scaling (Target Volatility):** Inversely proportional to the asset's ATR percentage. Higher volatility = smaller position size.

**Formula:**
`Position Size (KRW) = Total Equity * Base Risk Per Trade * |Signal| * (Target Volatility / Current Volatility)`

```python
# Pseudo-code / Python implementation for Position Sizing
def calculate_position_size(
    total_equity_krw: float,
    signal_strength: float,
    current_price: float,
    atr: float,
    base_risk_pct: float = 0.02,  # Risk 2% of equity per trade
    target_volatility_pct: float = 0.05 # Target 5% move
) -> float:
    if abs(signal_strength) < 0.1:
        return 0.0 # Ignore weak signals
        
    # Current volatility as a percentage of price
    current_vol_pct = atr / current_price
    
    # Volatility scalar (cap at 2.0 to prevent massive sizing in low vol)
    vol_scalar = min(target_volatility_pct / (current_vol_pct + 1e-8), 2.0)
    
    # Calculate target KRW allocation
    allocation_krw = total_equity_krw * base_risk_pct * abs(signal_strength) * vol_scalar
    
    # Apply Upbit constraints (e.g., min order size 5000 KRW)
    if allocation_krw < 5000:
        return 0.0
        
    return allocation_krw
```

### 3.2. Kill Switch Logic
The fail-safe mechanism monitors the system state and halts trading if extreme conditions are met.

**Triggers:**
1. **Consecutive Losses:** 5 consecutive realized losses across any assets.
2. **Extreme Slippage:** Execution price deviates from expected price by > 1.5%.
3. **Daily Drawdown:** Total equity drops by > 10% from the daily peak.

```rust
// Rust pseudo-code extension for kill_switch.rs
pub enum KillReason {
    DailyLossLimit,
    PositionLimit,
    FeedDisconnected,
    ManualIntervention,
    ConsecutiveOrderFailures,
    RiskAnomaly,
    ExtremeSlippage, // NEW
    ConsecutiveLosses, // NEW
}

// In execution gateway:
if (expected_price - executed_price).abs() / expected_price > 0.015 {
    kill_switch.activate(KillReason::ExtremeSlippage);
}

if consecutive_losses >= 5 {
    kill_switch.activate(KillReason::ConsecutiveLosses);
}
```

## 4. Task D: Backtesting Pipeline Setup

A highly efficient vectorized backtesting script using `vectorbt` to test the `StrategyEnsemble`. It accounts for Upbit's 0.05% fee and simulates slippage.

```python
# research/backtesting/engine.py
import pandas as pd
import numpy as np
import vectorbt as vbt
from strategies.ema_momentum import EMAMomentumStrategy
from strategies.bollinger_reversion import BollingerReversionStrategy
from strategies.atr_breakout import ATRBreakoutStrategy
from strategies.btc_divergence import BTCDivergenceStrategy
from strategies.ensemble import StrategyEnsemble

class VectorizedBacktester:
    def __init__(self, df_alt: pd.DataFrame, df_btc: pd.DataFrame, initial_cash=10000000):
        self.df_alt = df_alt
        self.df_btc = df_btc
        self.initial_cash = initial_cash
        # Upbit fee 0.05% + assumed slippage 0.1% for mid-cap altcoins
        self.total_fee_pct = 0.0005 + 0.0010 

    def run_backtest(self, regime_series: pd.Series):
        """
        regime_series: A pandas Series with the same index as df_alt, containing regime strings.
        """
        # 1. Generate individual signals
        ema = EMAMomentumStrategy().generate_signal(self.df_alt)
        boll = BollingerReversionStrategy().generate_signal(self.df_alt)
        atr = ATRBreakoutStrategy().generate_signal(self.df_alt)
        btc_div = BTCDivergenceStrategy().generate_signal(self.df_alt, self.df_btc)
        
        # 2. Apply Ensemble Weighting vectorially
        ensemble = StrategyEnsemble()
        final_signals = pd.Series(0.0, index=self.df_alt.index)
        
        # Vectorized application of weights based on regime
        for regime in ensemble.regime_weights.keys():
            mask = regime_series == regime
            weights = ensemble.regime_weights[regime]
            
            regime_signal = (
                ema.loc[mask] * weights[0] +
                boll.loc[mask] * weights[1] +
                atr.loc[mask] * weights[2] +
                btc_div.loc[mask] * weights[3]
            )
            final_signals.loc[mask] = regime_signal
            
        # Clip final signals
        final_signals = final_signals.clip(-1.0, 1.0)
        
        # 3. Generate Entry/Exit logic
        # Simple thresholding for demonstration: > 0.5 Long, < -0.5 Short
        entries = final_signals > 0.5
        exits = final_signals < 0.0
        short_entries = final_signals < -0.5
        short_exits = final_signals > 0.0
        
        # 4. Run vectorbt portfolio
        pf = vbt.Portfolio.from_signals(
            close=self.df_alt['close'],
            entries=entries,
            exits=exits,
            short_entries=short_entries,
            short_exits=short_exits,
            init_cash=self.initial_cash,
            fees=self.total_fee_pct,
            freq='1T' # Assuming 1-minute data
        )
        
        return pf.stats(), pf

# Example usage:
# stats, pf = VectorizedBacktester(df_doge, df_btc).run_backtest(regime_series)
# print(stats)
```

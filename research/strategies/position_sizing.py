"""
Dynamic Position Sizing & Risk Management
==========================================
Combines signal strength, account equity, and real-time ATR volatility
to compute the exact order size in KRW.

Formula
-------
Position Size (KRW) = Total Equity × Base Risk % × |Signal| × Vol Scalar

Where:
  Base Risk %  = Fixed fraction of equity risked per trade (default 2%)
  |Signal|     = Absolute ensemble signal strength in [0.0, 1.0]
  Vol Scalar   = min(Target Volatility % / Current ATR %, max_scalar)

Intuition:
  - A full-strength signal (|S|=1.0) with low volatility → large position.
  - A weak signal (|S|=0.2) with high volatility → very small position.
  - Vol Scalar is capped to prevent extreme sizing in low-volatility environments.

Kill Switch Triggers (Python-side monitoring)
---------------------------------------------
These complement the Rust kill_switch.rs and provide Python-level safety:
  1. Consecutive Losses  : 5 consecutive realized losses → halt
  2. Extreme Slippage    : |executed - expected| / expected > 1.5% → halt
  3. Daily Drawdown      : Equity drops > 10% from daily peak → halt
"""

from __future__ import annotations

import logging
from dataclasses import dataclass, field
from typing import Optional

logger = logging.getLogger(__name__)

# ── Constants ──────────────────────────────────────────────────────────────

UPBIT_MIN_ORDER_KRW = 5_000          # Upbit minimum order size
UPBIT_TAKER_FEE     = 0.0005         # 0.05% taker fee
MAX_VOL_SCALAR      = 2.0            # Cap on volatility scalar
MIN_SIGNAL_THRESHOLD = 0.10          # Ignore signals weaker than this


# ── Position Sizing ────────────────────────────────────────────────────────

def calculate_position_size(
    total_equity_krw: float,
    signal_strength: float,
    current_price: float,
    atr: float,
    base_risk_pct: float = 0.02,
    target_volatility_pct: float = 0.05,
    max_position_pct: float = 0.15,
) -> float:
    """
    Compute the target position size in KRW.

    Parameters
    ----------
    total_equity_krw : float
        Total account equity in KRW.
    signal_strength : float
        Ensemble signal in [-1.0, 1.0]. Sign indicates direction.
    current_price : float
        Current market price of the asset in KRW.
    atr : float
        Current ATR value in KRW (absolute, not percentage).
    base_risk_pct : float
        Base fraction of equity to risk per trade. Default 0.02 (2%).
    target_volatility_pct : float
        Target volatility as a fraction of price. Default 0.05 (5%).
    max_position_pct : float
        Maximum position size as a fraction of equity. Default 0.15 (15%).

    Returns
    -------
    float
        Order size in KRW. Returns 0.0 if below minimum or signal is too weak.
    """
    abs_signal = abs(signal_strength)

    # Ignore weak signals
    if abs_signal < MIN_SIGNAL_THRESHOLD:
        return 0.0

    if current_price <= 0 or atr <= 0:
        logger.warning("Invalid price or ATR: price=%s, atr=%s", current_price, atr)
        return 0.0

    # Current volatility as a fraction of price
    current_vol_pct = atr / current_price

    # Volatility scalar: inverse relationship with volatility
    # High volatility → small scalar → small position
    vol_scalar = min(target_volatility_pct / (current_vol_pct + 1e-8), MAX_VOL_SCALAR)

    # Target allocation
    allocation_krw = total_equity_krw * base_risk_pct * abs_signal * vol_scalar

    # Hard cap: never exceed max_position_pct of equity
    max_allocation = total_equity_krw * max_position_pct
    allocation_krw = min(allocation_krw, max_allocation)

    # Upbit minimum order check
    if allocation_krw < UPBIT_MIN_ORDER_KRW:
        return 0.0

    return round(allocation_krw, 0)


# ── Kill Switch Monitor (Python-side) ─────────────────────────────────────

@dataclass
class KillSwitchMonitor:
    """
    Python-side kill switch monitor that complements the Rust kill_switch.rs.

    Tracks consecutive losses, slippage events, and daily drawdown.
    Publishes a halt signal to Redis when thresholds are breached.

    Parameters
    ----------
    max_consecutive_losses : int
        Number of consecutive losses before halting. Default 5.
    max_slippage_pct : float
        Maximum acceptable slippage as a fraction of expected price. Default 0.015 (1.5%).
    max_daily_drawdown_pct : float
        Maximum daily drawdown as a fraction of daily peak equity. Default 0.10 (10%).
    """

    max_consecutive_losses: int = 5
    max_slippage_pct: float = 0.015
    max_daily_drawdown_pct: float = 0.10

    _consecutive_losses: int = field(default=0, init=False, repr=False)
    _daily_peak_equity: float = field(default=0.0, init=False, repr=False)
    _is_halted: bool = field(default=False, init=False, repr=False)
    _halt_reason: str = field(default="", init=False, repr=False)

    def reset_daily_peak(self, equity: float) -> None:
        """Call at the start of each trading day."""
        self._daily_peak_equity = equity
        self._consecutive_losses = 0

    def update_equity(self, current_equity: float) -> bool:
        """
        Update equity and check daily drawdown.

        Returns True if kill switch should be activated.
        """
        if current_equity > self._daily_peak_equity:
            self._daily_peak_equity = current_equity

        if self._daily_peak_equity > 0:
            drawdown = (self._daily_peak_equity - current_equity) / self._daily_peak_equity
            if drawdown >= self.max_daily_drawdown_pct:
                return self._halt(
                    f"DAILY_DRAWDOWN: {drawdown:.2%} exceeds limit {self.max_daily_drawdown_pct:.2%}"
                )
        return False

    def record_trade_result(self, pnl: float) -> bool:
        """
        Record a completed trade's PnL.

        Returns True if kill switch should be activated.
        """
        if pnl < 0:
            self._consecutive_losses += 1
        else:
            self._consecutive_losses = 0

        if self._consecutive_losses >= self.max_consecutive_losses:
            return self._halt(
                f"CONSECUTIVE_LOSSES: {self._consecutive_losses} consecutive losses"
            )
        return False

    def check_slippage(self, expected_price: float, executed_price: float) -> bool:
        """
        Check if slippage exceeds the threshold.

        Returns True if kill switch should be activated.
        """
        if expected_price <= 0:
            return False

        slippage = abs(executed_price - expected_price) / expected_price
        if slippage > self.max_slippage_pct:
            return self._halt(
                f"EXTREME_SLIPPAGE: {slippage:.4%} exceeds limit {self.max_slippage_pct:.4%}"
            )
        return False

    def _halt(self, reason: str) -> bool:
        if not self._is_halted:
            self._is_halted = True
            self._halt_reason = reason
            logger.critical("KILL SWITCH ACTIVATED [Python]: %s", reason)
        return True

    @property
    def is_halted(self) -> bool:
        return self._is_halted

    @property
    def halt_reason(self) -> str:
        return self._halt_reason

    def reset(self) -> None:
        """Manual reset by operator only."""
        self._is_halted = False
        self._halt_reason = ""
        self._consecutive_losses = 0
        logger.warning("KillSwitchMonitor RESET — trading may resume")

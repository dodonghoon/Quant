"""
30-Run Mass Backtest Runner
Executes PairsBacktester 30 times with parameter variations,
saves individual results and a summary report to research/Backtest_Data/.
"""

import os
import sys
import numpy as np
import pandas as pd
from datetime import datetime
from pathlib import Path

# Ensure research/ is importable
sys.path.insert(0, str(Path(__file__).parent))

OUTPUT_DIR = Path(__file__).parent / "Backtest_Data"
OUTPUT_DIR.mkdir(exist_ok=True)

# ── 30 parameter variations ──
RUNS = []
np.random.seed(42)
for i in range(30):
    RUNS.append({
        "run_id": i + 1,
        "entry_z":      round(1.0 + i * 0.07, 2),        # 1.00 → 3.03
        "exit_z":       round(0.3 + (i % 6) * 0.05, 2),  # cycle 0.30–0.55
        "lookback":     400 + i * 10,                      # 400 → 690
        "slippage_bps": 5.0 + (i % 5) * 2.0,              # 5–13 bps
        "commission_bps": 10.0,
        "random_seed":  42 + i,
    })


def generate_synthetic_prices(n: int = 2000, seed: int = 42) -> tuple:
    """Generate cointegrated price series for BTC/ETH pair."""
    rng = np.random.default_rng(seed)
    spread_noise = rng.normal(0, 1, n).cumsum() * 0.5
    common_trend = rng.normal(0, 1, n).cumsum() * 2.0
    price_a = 50000 + common_trend + spread_noise
    price_b = price_a / 15.0 + rng.normal(0, 50, n)
    idx = pd.date_range("2023-01-01", periods=n, freq="h")
    return pd.Series(price_a, index=idx, name="BTC"), pd.Series(price_b, index=idx, name="ETH")


def run_single_backtest(params: dict) -> dict:
    """Run one backtest with the given parameters. Returns metrics dict."""
    try:
        from backtesting.engine import PairsBacktester, BacktestConfig

        price_a, price_b = generate_synthetic_prices(
            n=2000, seed=params["random_seed"]
        )
        config = BacktestConfig(
            entry_z=params["entry_z"],
            exit_z=params["exit_z"],
            lookback=params["lookback"],
            slippage_bps=params["slippage_bps"],
            commission_bps=params["commission_bps"],
        )
        bt = PairsBacktester(price_a, price_b, config)
        result = bt.run()
        return {
            "run_id":         params["run_id"],
            "entry_z":        params["entry_z"],
            "exit_z":         params["exit_z"],
            "lookback":       params["lookback"],
            "slippage_bps":   params["slippage_bps"],
            "total_return":   result.total_return,
            "sharpe_ratio":   result.sharpe_ratio,
            "max_drawdown":   result.max_drawdown,
            "win_rate":       result.win_rate,
            "num_trades":     result.num_trades,
            "profit_factor":  result.profit_factor,
            "annual_return":  result.annual_return,
            "status":         "ok",
            "error":          "",
        }
    except Exception as exc:
        return {
            "run_id":        params["run_id"],
            "entry_z":       params["entry_z"],
            "exit_z":        params["exit_z"],
            "lookback":      params["lookback"],
            "slippage_bps":  params["slippage_bps"],
            "total_return":  0.0,
            "sharpe_ratio":  0.0,
            "max_drawdown":  0.0,
            "win_rate":      0.0,
            "num_trades":    0,
            "profit_factor": 0.0,
            "annual_return": 0.0,
            "status":        "error",
            "error":         str(exc),
        }


def save_run_result(metrics: dict) -> None:
    run_id = metrics["run_id"]
    filename = OUTPUT_DIR / f"run_{run_id:02d}.txt"
    lines = [
        f"=== Backtest Run {run_id:02d} ===",
        f"Timestamp      : {datetime.now().isoformat()}",
        f"Status         : {metrics['status']}",
        "",
        "--- Parameters ---",
        f"entry_z        : {metrics['entry_z']}",
        f"exit_z         : {metrics['exit_z']}",
        f"lookback       : {metrics['lookback']}",
        f"slippage_bps   : {metrics['slippage_bps']}",
        "",
        "--- Results ---",
        f"Total Return   : {metrics['total_return']:.4%}",
        f"Annual Return  : {metrics['annual_return']:.4%}",
        f"Sharpe Ratio   : {metrics['sharpe_ratio']:.4f}",
        f"Max Drawdown   : {metrics['max_drawdown']:.4%}",
        f"Win Rate       : {metrics['win_rate']:.4%}",
        f"Num Trades     : {metrics['num_trades']}",
        f"Profit Factor  : {metrics['profit_factor']:.4f}",
    ]
    if metrics["error"]:
        lines.append(f"Error          : {metrics['error']}")
    filename.write_text("\n".join(lines), encoding="utf-8")
    print(f"  [run {run_id:02d}] saved → {filename.name}")


def save_summary(all_metrics: list) -> None:
    ok_runs = [m for m in all_metrics if m["status"] == "ok" and m["num_trades"] > 0]

    if ok_runs:
        avg_win_rate   = np.mean([m["win_rate"] for m in ok_runs])
        max_drawdown   = max(abs(m["max_drawdown"]) for m in ok_runs)
        median_sharpe  = float(np.median([m["sharpe_ratio"] for m in ok_runs]))
        avg_return     = np.mean([m["total_return"] for m in ok_runs])
        avg_trades     = np.mean([m["num_trades"] for m in ok_runs])
    else:
        avg_win_rate = max_drawdown = median_sharpe = avg_return = avg_trades = 0.0

    best = max(ok_runs, key=lambda m: m["sharpe_ratio"], default=None)
    worst = min(ok_runs, key=lambda m: m["total_return"], default=None)

    lines = [
        "=" * 50,
        "  MASS BACKTEST SUMMARY REPORT — 30 RUNS",
        "=" * 50,
        f"Generated       : {datetime.now().isoformat()}",
        f"Total Runs      : {len(all_metrics)}",
        f"Successful Runs : {len(ok_runs)}",
        f"Failed Runs     : {len(all_metrics) - len(ok_runs)}",
        "",
        "--- Aggregate Metrics ---",
        f"Average Win Rate    : {avg_win_rate:.4%}",
        f"Max Drawdown (worst): {max_drawdown:.4%}",
        f"Median Sharpe Ratio : {median_sharpe:.4f}",
        f"Average Total Return: {avg_return:.4%}",
        f"Average Num Trades  : {avg_trades:.1f}",
    ]

    if best:
        lines += [
            "",
            f"--- Best Run (Sharpe) ---",
            f"  Run {best['run_id']:02d} | entry_z={best['entry_z']} "
            f"| Sharpe={best['sharpe_ratio']:.4f} | Return={best['total_return']:.4%}",
        ]
    if worst:
        lines += [
            f"--- Worst Run (Return) ---",
            f"  Run {worst['run_id']:02d} | entry_z={worst['entry_z']} "
            f"| Return={worst['total_return']:.4%} | Drawdown={worst['max_drawdown']:.4%}",
        ]

    lines += ["", "=" * 50]
    summary_path = OUTPUT_DIR / "Summary_Report.txt"
    summary_path.write_text("\n".join(lines), encoding="utf-8")
    print(f"\n[SUMMARY] saved → {summary_path}")
    print("\n".join(lines))


if __name__ == "__main__":
    print(f"[{datetime.now().isoformat()}] Starting 30-run mass backtest...")
    print(f"Output directory: {OUTPUT_DIR}\n")

    all_metrics = []
    for params in RUNS:
        print(f"Running backtest {params['run_id']:02d}/30 "
              f"(entry_z={params['entry_z']}, lookback={params['lookback']})...")
        metrics = run_single_backtest(params)
        save_run_result(metrics)
        all_metrics.append(metrics)

    save_summary(all_metrics)
    print(f"\n[DONE] All 30 backtests complete. Files in: {OUTPUT_DIR}")

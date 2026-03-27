'use client';

import { useTradingStore } from '@/stores/tradingStore';
import { formatPnl } from '@/lib/formatters';

function ProgressBar({ label, current, max, unit = '' }: { label: string; current: number; max: number; unit?: string }) {
  const pct = Math.min((Math.abs(current) / max) * 100, 100);
  const color = pct > 80 ? 'bg-red-500' : pct > 50 ? 'bg-yellow-500' : 'bg-green-500';

  return (
    <div className="space-y-1">
      <div className="flex justify-between text-xs">
        <span className="text-gray-400">{label}</span>
        <span className="font-mono text-gray-300">
          {typeof current === 'number' && unit === '$' ? formatPnl(current) : current.toFixed(1)}{unit} / {max}{unit}
        </span>
      </div>
      <div className="h-1.5 w-full rounded-full bg-gray-800">
        <div className={`h-1.5 rounded-full ${color} transition-all`} style={{ width: `${pct}%` }} />
      </div>
    </div>
  );
}

export default function RiskSummary() {
  const riskMetrics = useTradingStore((s) => s.riskMetrics);

  return (
    <div className="rounded-lg border border-gray-800 bg-bg-secondary p-4">
      <div className="mb-3 text-xs text-gray-500">리스크 요약</div>
      <div className="space-y-3">
        <ProgressBar
          label="일일 손실"
          current={riskMetrics?.daily_pnl ?? -342}
          max={riskMetrics?.max_daily_loss ?? 1000}
          unit="$"
        />
        <ProgressBar
          label="총 노출"
          current={riskMetrics?.total_exposure ?? 0.85}
          max={riskMetrics?.max_exposure ?? 2.0}
          unit="x"
        />
        <ProgressBar
          label="최대 포지션"
          current={riskMetrics?.max_position_used ?? 12}
          max={riskMetrics?.max_position_limit ?? 10}
          unit="%"
        />
        <ProgressBar
          label="주문률"
          current={riskMetrics?.order_rate ?? 12}
          max={riskMetrics?.max_order_rate ?? 50}
          unit="/s"
        />
      </div>
    </div>
  );
}

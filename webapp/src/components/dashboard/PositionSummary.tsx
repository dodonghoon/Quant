'use client';

import { useTradingStore } from '@/stores/tradingStore';
import { formatQuantity, formatPnl, pnlColorClass } from '@/lib/formatters';

const demoPositions = [
  { symbol: 'BTC', quantity: 0.15, unrealized_pnl: 20.18 },
  { symbol: 'ETH', quantity: -2.3, unrealized_pnl: -4.6 },
  { symbol: 'SOL', quantity: 45.0, unrealized_pnl: 11.25 },
];

export default function PositionSummary() {
  const positions = useTradingStore((s) => s.positions);
  const data = positions.length > 0 ? positions : demoPositions;

  return (
    <div className="rounded-lg border border-gray-800 bg-bg-secondary p-4">
      <div className="mb-3 text-xs text-gray-500">포지션</div>
      <div className="space-y-2">
        {data.map((pos) => (
          <div key={pos.symbol} className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <span className="text-sm font-medium text-white">{pos.symbol}</span>
              <span className="text-xs text-gray-500">
                {pos.quantity > 0 ? '+' : ''}{formatQuantity(pos.quantity)}
              </span>
            </div>
            <span className={`text-sm font-mono font-tabular ${pnlColorClass(pos.unrealized_pnl)}`}>
              {formatPnl(pos.unrealized_pnl)}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}

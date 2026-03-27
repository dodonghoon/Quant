'use client';

import { useTradingStore } from '@/stores/tradingStore';
import { formatPnl, formatPercent, pnlColorClass } from '@/lib/formatters';
import { TrendingUp, TrendingDown } from 'lucide-react';
import { LineChart, Line, ResponsiveContainer } from 'recharts';

// 데모 PnL 곡선 데이터
const demoPnlCurve = [
  { t: 1, v: 0 }, { t: 2, v: 50 }, { t: 3, v: 120 }, { t: 4, v: 80 },
  { t: 5, v: 200 }, { t: 6, v: 180 }, { t: 7, v: 250 }, { t: 8, v: 342 },
];

export default function PnlCard() {
  const dailyPnl = useTradingStore((s) => s.dailyPnl);
  const total = dailyPnl?.total ?? 342.33;
  const isPositive = total >= 0;

  return (
    <div className="rounded-lg border border-gray-800 bg-bg-secondary p-4">
      <div className="mb-1 text-xs text-gray-500">오늘의 PnL</div>
      <div className="flex items-center gap-2">
        <span className={`text-2xl font-bold font-tabular ${pnlColorClass(total)}`}>
          {formatPnl(total)}
        </span>
        {isPositive ? (
          <TrendingUp size={18} className="text-profit" />
        ) : (
          <TrendingDown size={18} className="text-loss" />
        )}
      </div>
      <div className={`text-xs ${pnlColorClass(total)}`}>
        {formatPercent(total / 100000)}
      </div>
      <div className="mt-3 h-16">
        <ResponsiveContainer width="100%" height="100%">
          <LineChart data={demoPnlCurve}>
            <Line
              type="monotone"
              dataKey="v"
              stroke={isPositive ? '#22c55e' : '#ef4444'}
              strokeWidth={2}
              dot={false}
            />
          </LineChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}

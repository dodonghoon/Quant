'use client';

import Link from 'next/link';
import { useTradingStore } from '@/stores/tradingStore';
import { formatPrice, pnlColorClass } from '@/lib/formatters';
import { TrendingUp, TrendingDown } from 'lucide-react';

const demoSymbols = [
  { symbol: 'BTC-USDT', price: 67234.5, change: 0.023, bid: 67233, ask: 67236 },
  { symbol: 'ETH-USDT', price: 3520.8, change: -0.011, bid: 3520, ask: 3521 },
  { symbol: 'SOL-USDT', price: 142.35, change: 0.045, bid: 142.3, ask: 142.4 },
  { symbol: 'AVAX-USDT', price: 38.92, change: 0.018, bid: 38.9, ask: 38.95 },
  { symbol: 'BNB-USDT', price: 612.4, change: -0.005, bid: 612.2, ask: 612.6 },
  { symbol: 'LINK-USDT', price: 18.75, change: 0.032, bid: 18.74, ask: 18.76 },
];

export default function MarketPage() {
  return (
    <div className="space-y-6">
      <h1 className="text-lg font-semibold text-white">시장 데이터</h1>
      <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
        {demoSymbols.map((s) => (
          <Link
            key={s.symbol}
            href={`/market/${s.symbol.split('-')[0]}`}
            className="rounded-lg border border-gray-800 bg-bg-secondary p-4 transition-colors hover:border-gray-700"
          >
            <div className="flex items-center justify-between">
              <span className="font-medium text-white">{s.symbol}</span>
              {s.change >= 0 ? (
                <TrendingUp size={16} className="text-profit" />
              ) : (
                <TrendingDown size={16} className="text-loss" />
              )}
            </div>
            <div className="mt-2">
              <span className="text-xl font-bold font-tabular text-white">
                ${formatPrice(s.price)}
              </span>
              <span className={`ml-2 text-sm ${pnlColorClass(s.change)}`}>
                {s.change >= 0 ? '+' : ''}{(s.change * 100).toFixed(1)}%
              </span>
            </div>
            <div className="mt-2 flex justify-between text-xs text-gray-500">
              <span>Bid: ${formatPrice(s.bid)}</span>
              <span>Ask: ${formatPrice(s.ask)}</span>
            </div>
          </Link>
        ))}
      </div>
    </div>
  );
}

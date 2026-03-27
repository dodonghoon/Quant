'use client';

import { useParams } from 'next/navigation';
import { useTradingStore } from '@/stores/tradingStore';
import PriceChart from '@/components/market/PriceChart';
import OrderBook from '@/components/market/OrderBook';
import TradeHistory from '@/components/market/TradeHistory';
import ModelPanel from '@/components/market/ModelPanel';

export default function SymbolDetailPage() {
  const params = useParams();
  const symbol = params.symbol as string;

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-lg font-semibold text-white">{symbol}-USDT</h1>
        <div className="flex items-center gap-4 text-sm">
          <span className="text-2xl font-bold font-tabular text-white">$67,234.50</span>
          <span className="text-profit">+2.3%</span>
        </div>
      </div>

      {/* 차트 영역 */}
      <PriceChart symbol={symbol} />

      {/* 하단 3열 */}
      <div className="grid grid-cols-1 gap-4 lg:grid-cols-3">
        <OrderBook symbol={symbol} />
        <TradeHistory symbol={symbol} />
        <ModelPanel symbol={symbol} />
      </div>
    </div>
  );
}

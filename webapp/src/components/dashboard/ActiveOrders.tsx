'use client';

import { useTradingStore } from '@/stores/tradingStore';
import { formatPrice, formatQuantity } from '@/lib/formatters';

const demoOrders = [
  { internal_id: 1042, symbol: 'BTC', side: 'Buy' as const, order_type: 'Limit' as const, quantity: 0.15, price: 67200, status: 'Sent' as const },
  { internal_id: 1041, symbol: 'ETH', side: 'Sell' as const, order_type: 'Market' as const, quantity: 2.3, price: null, status: 'Filled' as const },
];

export default function ActiveOrders() {
  const activeOrders = useTradingStore((s) => s.activeOrders);
  const orders = Object.values(activeOrders);
  const data = orders.length > 0 ? orders : demoOrders;

  const statusColor = (s: string) => {
    switch (s) {
      case 'Filled': return 'text-profit';
      case 'Cancelled': case 'Rejected': return 'text-loss';
      case 'Sent': case 'Pending': return 'text-yellow-400';
      default: return 'text-gray-400';
    }
  };

  return (
    <div className="rounded-lg border border-gray-800 bg-bg-secondary p-4">
      <div className="mb-3 text-xs text-gray-500">활성 주문</div>
      <div className="space-y-2">
        {data.map((order) => (
          <div key={order.internal_id} className="flex items-center justify-between rounded px-2 py-1.5 hover:bg-bg-tertiary">
            <div className="flex items-center gap-3">
              <span className="font-mono text-xs text-gray-500">#{order.internal_id}</span>
              <span className="text-sm font-medium text-white">{order.symbol}</span>
              <span className={`text-xs ${order.side === 'Buy' ? 'text-profit' : 'text-loss'}`}>
                {order.side}
              </span>
              <span className="text-xs text-gray-400">{formatQuantity(order.quantity)}</span>
            </div>
            <div className="flex items-center gap-3">
              {order.price && (
                <span className="font-mono text-xs text-gray-400">
                  {order.order_type} ${formatPrice(order.price)}
                </span>
              )}
              {!order.price && (
                <span className="text-xs text-gray-400">{order.order_type}</span>
              )}
              <span className={`text-xs font-medium ${statusColor(order.status)}`}>
                {order.status} {order.status === 'Filled' ? '✓' : ''}
              </span>
            </div>
          </div>
        ))}
        {data.length === 0 && (
          <div className="py-4 text-center text-sm text-gray-600">활성 주문 없음</div>
        )}
      </div>
    </div>
  );
}

'use client';

interface TradeHistoryProps {
  symbol: string;
}

const demoTrades = [
  { price: 67235, qty: 0.12, side: 'Buy' as const, time: '09:32:14' },
  { price: 67234, qty: 0.05, side: 'Sell' as const, time: '09:32:12' },
  { price: 67235, qty: 0.3, side: 'Buy' as const, time: '09:32:10' },
  { price: 67233, qty: 0.1, side: 'Sell' as const, time: '09:32:08' },
  { price: 67234, qty: 0.5, side: 'Buy' as const, time: '09:32:05' },
  { price: 67232, qty: 0.8, side: 'Sell' as const, time: '09:32:02' },
];

export default function TradeHistory({ symbol }: TradeHistoryProps) {
  return (
    <div className="rounded-lg border border-gray-800 bg-bg-secondary p-4">
      <div className="mb-3 text-xs text-gray-500">최근 체결</div>
      <div className="space-y-0.5">
        <div className="mb-1 flex justify-between text-xs text-gray-600">
          <span>가격</span>
          <span>수량</span>
          <span>시간</span>
        </div>
        {demoTrades.map((trade, i) => (
          <div key={i} className="flex justify-between py-0.5 text-xs">
            <span className={`font-mono ${trade.side === 'Buy' ? 'text-profit' : 'text-loss'}`}>
              ${trade.price.toLocaleString()}
            </span>
            <span className="font-mono text-gray-400">{trade.qty.toFixed(2)}</span>
            <span className="font-mono text-gray-500">{trade.time}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

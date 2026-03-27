'use client';

interface OrderBookProps {
  symbol: string;
}

const demoAsks = [
  { price: 67240, qty: 1.2 },
  { price: 67238, qty: 0.85 },
  { price: 67236, qty: 2.5 },
  { price: 67235, qty: 0.3 },
];

const demoBids = [
  { price: 67234, qty: 2.1 },
  { price: 67233, qty: 0.6 },
  { price: 67230, qty: 1.8 },
  { price: 67228, qty: 3.2 },
];

export default function OrderBook({ symbol }: OrderBookProps) {
  const maxQty = Math.max(...[...demoAsks, ...demoBids].map((l) => l.qty));

  return (
    <div className="rounded-lg border border-gray-800 bg-bg-secondary p-4">
      <div className="mb-3 text-xs text-gray-500">호가창</div>
      <div className="space-y-0.5">
        <div className="mb-1 flex justify-between text-xs text-gray-600">
          <span>가격</span>
          <span>수량</span>
        </div>
        {/* Ask (매도) - 역순 */}
        {[...demoAsks].reverse().map((level) => (
          <div key={level.price} className="relative flex justify-between py-0.5 text-xs">
            <div
              className="absolute inset-y-0 right-0 bg-red-500/10"
              style={{ width: `${(level.qty / maxQty) * 100}%` }}
            />
            <span className="relative font-mono text-loss">${level.price.toLocaleString()}</span>
            <span className="relative font-mono text-gray-400">{level.qty.toFixed(2)}</span>
          </div>
        ))}
        {/* Mid */}
        <div className="border-y border-gray-700 py-1 text-center text-xs text-gray-500">
          spread: ${(demoAsks[demoAsks.length - 1].price - demoBids[0].price).toFixed(0)}
        </div>
        {/* Bid (매수) */}
        {demoBids.map((level) => (
          <div key={level.price} className="relative flex justify-between py-0.5 text-xs">
            <div
              className="absolute inset-y-0 right-0 bg-green-500/10"
              style={{ width: `${(level.qty / maxQty) * 100}%` }}
            />
            <span className="relative font-mono text-profit">${level.price.toLocaleString()}</span>
            <span className="relative font-mono text-gray-400">{level.qty.toFixed(2)}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

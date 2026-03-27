'use client';

import { useTradingStore } from '@/stores/tradingStore';
import { statusDotClass } from '@/lib/formatters';
import { Plus } from 'lucide-react';

const demoPairs = [
  { id: '1', leg_a: 'BTC', leg_b: 'ETH', hedge_ratio: 0.052, status: 'active' as const, kappa: 0.045, mu: -0.12, z_score: -1.82 },
  { id: '2', leg_a: 'SOL', leg_b: 'AVAX', hedge_ratio: 0.83, status: 'active' as const, kappa: 0.021, mu: 0.05, z_score: 0.31 },
  { id: '3', leg_a: 'BNB', leg_b: 'SOL', hedge_ratio: 1.24, status: 'weak' as const, kappa: 0.008, mu: 0.22, z_score: 1.05 },
];

export default function PairTable() {
  const pairs = useTradingStore((s) => s.pairs);
  const data = pairs.length > 0 ? pairs : demoPairs;

  return (
    <div className="rounded-lg border border-gray-800 bg-bg-secondary p-4">
      <div className="mb-3 flex items-center justify-between">
        <span className="text-sm font-medium text-white">등록된 페어</span>
        <button className="flex items-center gap-1 rounded bg-bg-tertiary px-3 py-1 text-xs text-gray-300 hover:bg-gray-600">
          <Plus size={12} />
          페어 추가
        </button>
      </div>
      <table className="w-full text-sm">
        <thead>
          <tr className="border-b border-gray-700 text-xs text-gray-500">
            <th className="py-2 text-left">Pair</th>
            <th className="py-2 text-right">Hedge</th>
            <th className="py-2 text-right">κ</th>
            <th className="py-2 text-right">µ</th>
            <th className="py-2 text-right">z-score</th>
            <th className="py-2 text-center">Status</th>
          </tr>
        </thead>
        <tbody>
          {data.map((pair: any) => (
            <tr key={pair.id} className="border-b border-gray-800 hover:bg-bg-tertiary">
              <td className="py-2 font-medium text-white">{pair.leg_a}-{pair.leg_b}</td>
              <td className="py-2 text-right font-mono text-gray-400">{pair.hedge_ratio?.toFixed(3)}</td>
              <td className="py-2 text-right font-mono text-gray-400">{pair.kappa?.toFixed(3) ?? '-'}</td>
              <td className="py-2 text-right font-mono text-gray-400">{pair.mu?.toFixed(2) ?? '-'}</td>
              <td className={`py-2 text-right font-mono ${(pair.z_score ?? 0) < -1 ? 'text-profit' : (pair.z_score ?? 0) > 1 ? 'text-loss' : 'text-gray-400'}`}>
                {pair.z_score?.toFixed(2) ?? '-'}
              </td>
              <td className="py-2 text-center">
                <span className={`inline-block h-2 w-2 rounded-full ${statusDotClass(pair.status)}`} />
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

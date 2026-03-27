'use client';

import { useTradingStore } from '@/stores/tradingStore';
import { formatLatency, statusDotClass } from '@/lib/formatters';
import { Activity } from 'lucide-react';

export default function SystemHealth() {
  const systemStatus = useTradingStore((s) => s.systemStatus);

  const layers = [
    { label: 'Feed', status: systemStatus?.feed ?? 'connected' },
    { label: 'Strategy', status: systemStatus?.strategy ?? 'running' },
    { label: 'Execution', status: systemStatus?.execution ?? 'running' },
  ];

  return (
    <div className="rounded-lg border border-gray-800 bg-bg-secondary p-4">
      <div className="mb-3 flex items-center gap-2 text-xs text-gray-500">
        <Activity size={12} />
        시스템 상태
      </div>
      <div className="space-y-2">
        {layers.map(({ label, status }) => (
          <div key={label} className="flex items-center justify-between">
            <span className="text-sm text-gray-300">{label}</span>
            <div className="flex items-center gap-2">
              <span className={`h-2 w-2 rounded-full ${statusDotClass(status)}`} />
              <span className="text-xs text-gray-400">{status === 'connected' || status === 'running' ? '정상' : status}</span>
            </div>
          </div>
        ))}
        <div className="mt-2 flex items-center justify-between border-t border-gray-800 pt-2">
          <span className="text-sm text-gray-300">Latency</span>
          <span className="font-mono text-xs text-accent-cyan">
            {formatLatency(systemStatus?.latency_us ?? 42)}
          </span>
        </div>
      </div>
    </div>
  );
}

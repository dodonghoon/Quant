'use client';

import SignalConfigPanel from '@/components/strategy/SignalConfigPanel';
import PairTable from '@/components/strategy/PairTable';
import ModelConfigPanel from '@/components/strategy/ModelConfigPanel';

export default function StrategyPage() {
  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-lg font-semibold text-white">전략 엔진</h1>
        <div className="flex items-center gap-2">
          <span className="h-2 w-2 rounded-full bg-green-500" />
          <span className="text-sm text-gray-400">ON</span>
        </div>
      </div>

      <SignalConfigPanel />
      <PairTable />
      <ModelConfigPanel />
    </div>
  );
}

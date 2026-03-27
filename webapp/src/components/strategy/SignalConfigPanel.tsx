'use client';

import { useState } from 'react';
import { toast } from 'sonner';
import SliderInput from '@/components/common/SliderInput';
import { useTradingStore } from '@/stores/tradingStore';
import * as api from '@/lib/api';

export default function SignalConfigPanel() {
  const [entryThreshold, setEntryThreshold] = useState(1.5);
  const [strongEntry, setStrongEntry] = useState(2.5);
  const [exitThreshold, setExitThreshold] = useState(0.5);
  const [ouWeight, setOuWeight] = useState(0.7);
  const [minConfidence, setMinConfidence] = useState(0.3);

  const handleApply = async () => {
    try {
      await api.config.putSignal({
        entry_threshold: entryThreshold,
        strong_entry_threshold: strongEntry,
        exit_threshold: exitThreshold,
        ou_weight: ouWeight,
        kalman_weight: 1 - ouWeight,
        min_confidence: minConfidence,
      });
      toast.success('시그널 설정이 적용되었습니다');
    } catch {
      toast.error('설정 적용에 실패했습니다');
    }
  };

  const handleReset = () => {
    setEntryThreshold(1.5);
    setStrongEntry(2.5);
    setExitThreshold(0.5);
    setOuWeight(0.7);
    setMinConfidence(0.3);
  };

  return (
    <div className="rounded-lg border border-gray-800 bg-bg-secondary p-4">
      <div className="mb-4 text-sm font-medium text-white">시그널 설정 (라이브 조정)</div>
      <div className="space-y-4">
        <SliderInput label="Entry Threshold" value={entryThreshold} min={0.5} max={3.0} step={0.1} unit="σ" onChange={setEntryThreshold} />
        <SliderInput label="Strong Entry" value={strongEntry} min={1.5} max={4.0} step={0.1} unit="σ" onChange={setStrongEntry} />
        <SliderInput label="Exit Threshold" value={exitThreshold} min={0.1} max={1.5} step={0.1} unit="σ" onChange={setExitThreshold} />
        <SliderInput label="OU Weight" value={ouWeight} min={0} max={1} step={0.05} onChange={setOuWeight} />
        <div className="text-xs text-gray-600">Kalman Weight: {(1 - ouWeight).toFixed(2)} (auto = 1 - OU)</div>
        <SliderInput label="Min Confidence" value={minConfidence} min={0} max={1} step={0.05} onChange={setMinConfidence} />
      </div>
      <div className="mt-4 flex gap-2">
        <button onClick={handleApply} className="rounded bg-accent-blue px-4 py-1.5 text-sm text-white hover:bg-blue-600">적용</button>
        <button onClick={handleReset} className="rounded bg-bg-tertiary px-4 py-1.5 text-sm text-gray-300 hover:bg-gray-600">기본값 복원</button>
      </div>
    </div>
  );
}

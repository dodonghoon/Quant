'use client';

import { useState } from 'react';
import { toast } from 'sonner';
import * as api from '@/lib/api';

export default function ModelConfigPanel() {
  const [processNoise, setProcessNoise] = useState('1e-5');
  const [measurementNoise, setMeasurementNoise] = useState('1e-3');
  const [garchAlpha, setGarchAlpha] = useState('0.06');
  const [garchBeta, setGarchBeta] = useState('0.90');

  const handleApply = async () => {
    try {
      await api.config.putKalman({
        process_noise: parseFloat(processNoise),
        measurement_noise: parseFloat(measurementNoise),
      });
      await api.config.putGarch({
        p: 1,
        q: 1,
        omega: 0.00001,
        alpha: [parseFloat(garchAlpha)],
        beta: [parseFloat(garchBeta)],
      });
      toast.success('모델 설정이 적용되었습니다');
    } catch {
      toast.error('설정 적용에 실패했습니다');
    }
  };

  return (
    <div className="rounded-lg border border-gray-800 bg-bg-secondary p-4">
      <div className="mb-4 text-sm font-medium text-white">Kalman / GARCH 설정</div>
      <div className="grid grid-cols-2 gap-4">
        <div>
          <label className="mb-1 block text-xs text-gray-500">Process Noise (Q)</label>
          <input value={processNoise} onChange={(e) => setProcessNoise(e.target.value)} className="w-full rounded border border-gray-700 bg-bg-tertiary px-3 py-1.5 font-mono text-sm text-white outline-none focus:border-accent-blue" />
        </div>
        <div>
          <label className="mb-1 block text-xs text-gray-500">Measurement Noise (R)</label>
          <input value={measurementNoise} onChange={(e) => setMeasurementNoise(e.target.value)} className="w-full rounded border border-gray-700 bg-bg-tertiary px-3 py-1.5 font-mono text-sm text-white outline-none focus:border-accent-blue" />
        </div>
        <div>
          <label className="mb-1 block text-xs text-gray-500">GARCH α</label>
          <input value={garchAlpha} onChange={(e) => setGarchAlpha(e.target.value)} className="w-full rounded border border-gray-700 bg-bg-tertiary px-3 py-1.5 font-mono text-sm text-white outline-none focus:border-accent-blue" />
        </div>
        <div>
          <label className="mb-1 block text-xs text-gray-500">GARCH β</label>
          <input value={garchBeta} onChange={(e) => setGarchBeta(e.target.value)} className="w-full rounded border border-gray-700 bg-bg-tertiary px-3 py-1.5 font-mono text-sm text-white outline-none focus:border-accent-blue" />
        </div>
      </div>
      <div className="mt-4">
        <button onClick={handleApply} className="rounded bg-accent-blue px-4 py-1.5 text-sm text-white hover:bg-blue-600">적용</button>
      </div>
    </div>
  );
}

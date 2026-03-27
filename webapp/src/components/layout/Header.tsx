'use client';

import { useEffect, useState } from 'react';
import { Power, Wifi, WifiOff, Clock } from 'lucide-react';
import { useTradingStore } from '@/stores/tradingStore';
import ConnectionStatus from './ConnectionStatus';

export default function Header() {
  const [time, setTime] = useState('');
  const killSwitch = useTradingStore((s) => s.killSwitch);

  useEffect(() => {
    const tick = () => {
      setTime(
        new Date().toLocaleTimeString('ko-KR', {
          hour: '2-digit',
          minute: '2-digit',
          second: '2-digit',
          hour12: false,
        })
      );
    };
    tick();
    const interval = setInterval(tick, 1000);
    return () => clearInterval(interval);
  }, []);

  return (
    <header className="flex h-14 items-center justify-between border-b border-gray-800 bg-bg-secondary px-6">
      {/* Kill Switch 버튼 */}
      <div className="flex items-center gap-4">
        <button
          className={`flex items-center gap-2 rounded-md px-4 py-1.5 text-sm font-bold transition-colors ${
            killSwitch.active
              ? 'animate-pulse bg-red-600 text-white'
              : 'bg-red-600/20 text-red-400 hover:bg-red-600/40'
          }`}
        >
          <Power size={16} />
          {killSwitch.active ? 'KILL SWITCH ON' : 'KILL SWITCH'}
        </button>
        {killSwitch.active && killSwitch.reason && (
          <span className="text-xs text-red-400">
            사유: {killSwitch.reason}
          </span>
        )}
      </div>

      {/* 우측: 시계 + 연결 상태 */}
      <div className="flex items-center gap-6">
        <ConnectionStatus />
        <div className="flex items-center gap-2 text-sm text-gray-400">
          <Clock size={14} />
          <span className="font-mono font-tabular">{time}</span>
          <span className="text-xs">KST</span>
        </div>
      </div>
    </header>
  );
}

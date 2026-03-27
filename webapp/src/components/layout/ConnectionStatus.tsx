'use client';

import { Wifi, WifiOff, Loader2 } from 'lucide-react';
import { useTradingStore } from '@/stores/tradingStore';

export default function ConnectionStatus() {
  const connectionState = useTradingStore((s) => s.connectionState);

  const config = {
    connected: {
      icon: Wifi,
      color: 'text-green-400',
      label: '연결됨',
      dot: 'bg-green-500',
    },
    connecting: {
      icon: Loader2,
      color: 'text-yellow-400',
      label: '연결 중...',
      dot: 'bg-yellow-500',
    },
    disconnected: {
      icon: WifiOff,
      color: 'text-red-400',
      label: '연결 끊김',
      dot: 'bg-red-500',
    },
  }[connectionState];

  const Icon = config.icon;

  return (
    <div className={`flex items-center gap-2 text-sm ${config.color}`}>
      <span className={`h-2 w-2 rounded-full ${config.dot}`} />
      <Icon size={14} className={connectionState === 'connecting' ? 'animate-spin' : ''} />
      <span>{config.label}</span>
    </div>
  );
}

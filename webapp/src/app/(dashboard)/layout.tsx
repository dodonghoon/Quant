'use client';

import Sidebar from '@/components/layout/Sidebar';
import Header from '@/components/layout/Header';
import { useTradingStore } from '@/stores/tradingStore';

export default function DashboardLayout({ children }: { children: React.ReactNode }) {
  const killSwitch = useTradingStore((s) => s.killSwitch);

  return (
    <div className={`flex h-screen ${killSwitch.active ? 'kill-switch-active' : ''}`}>
      <Sidebar />
      <div className="flex flex-1 flex-col overflow-hidden">
        <Header />
        <main className="flex-1 overflow-y-auto p-6">{children}</main>
      </div>
    </div>
  );
}

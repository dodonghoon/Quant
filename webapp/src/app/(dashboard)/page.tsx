'use client';

import PnlCard from '@/components/dashboard/PnlCard';
import PositionSummary from '@/components/dashboard/PositionSummary';
import SystemHealth from '@/components/dashboard/SystemHealth';
import SignalFeed from '@/components/dashboard/SignalFeed';
import ActiveOrders from '@/components/dashboard/ActiveOrders';
import RiskSummary from '@/components/dashboard/RiskSummary';

export default function DashboardPage() {
  return (
    <div className="space-y-6">
      <h1 className="text-lg font-semibold text-white">대시보드</h1>

      {/* 상단 카드 3개 */}
      <div className="grid grid-cols-1 gap-4 md:grid-cols-3">
        <PnlCard />
        <PositionSummary />
        <SystemHealth />
      </div>

      {/* 시그널 피드 */}
      <SignalFeed />

      {/* 하단 2열 */}
      <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
        <ActiveOrders />
        <RiskSummary />
      </div>
    </div>
  );
}

'use client';

import { useEffect, useRef } from 'react';

interface PriceChartProps {
  symbol: string;
}

export default function PriceChart({ symbol }: PriceChartProps) {
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    // TradingView Lightweight Charts 초기화는 실제 라이브러리 설치 후 구현
    // 현재는 플레이스홀더
  }, [symbol]);

  return (
    <div className="rounded-lg border border-gray-800 bg-bg-secondary p-4">
      <div className="mb-2 text-xs text-gray-500">가격 차트 — {symbol}-USDT</div>
      <div
        ref={containerRef}
        className="flex h-80 items-center justify-center rounded bg-bg-primary text-gray-600"
      >
        <div className="text-center">
          <p className="text-sm">TradingView Lightweight Charts</p>
          <p className="mt-1 text-xs text-gray-700">
            Kalman 필터 오버레이 + OU Z-score 서브차트 + GARCH 변동성 서브차트
          </p>
          <p className="mt-2 text-xs text-gray-700">
            npm install lightweight-charts 후 구현
          </p>
        </div>
      </div>
    </div>
  );
}

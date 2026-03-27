'use client';

import { useState, useEffect } from 'react';
import { useTradingStore } from '@/stores/tradingStore';
import { formatTimestamp, formatZScore, signalColorClass } from '@/lib/formatters';
import { Radio } from 'lucide-react';
import type { TradingSignal } from '@/types/trading';

// ─────────────────────────────────────────────────────────────────────────────
// Demo signal TEMPLATES — no Date.now() here.
// Dynamic timestamps are only computed client-side (after mount) to prevent
// the SSR/hydration mismatch:
//   Server renders at T1  →  formatTimestamp gives "05:02:18"
//   Client hydrates at T2 →  formatTimestamp gives "05:02:20"  ← mismatch!
// ─────────────────────────────────────────────────────────────────────────────
const DEMO_TEMPLATES = [
  {
    symbol: 'BTC-ETH',
    direction: 'StrongBuy',
    composite_z: -2.7,
    confidence: 0.89,
    raw_position_frac: 0.62,
    offsetMs: 0,
    alpha: { ou_z: -2.8, ou_weight: 0.7, kalman_innovation: -1.2, kalman_weight: 0.3 },
  },
  {
    symbol: 'SOL-AVAX',
    direction: 'Neutral',
    composite_z: 0.3,
    confidence: 0.45,
    raw_position_frac: 0.05,
    offsetMs: 4_000,
    alpha: { ou_z: 0.3, ou_weight: 0.7, kalman_innovation: 0.1, kalman_weight: 0.3 },
  },
  {
    symbol: 'BTC-ETH',
    direction: 'Buy',
    composite_z: -1.8,
    confidence: 0.72,
    raw_position_frac: 0.42,
    offsetMs: 16_000,
    alpha: { ou_z: -1.9, ou_weight: 0.7, kalman_innovation: -0.8, kalman_weight: 0.3 },
  },
] as const;

export default function SignalFeed() {
  const signals = useTradingStore((s) => s.signals);

  // ── Two-pass rendering guard ────────────────────────────────────────────────
  // `mounted` is false during SSR and the first synchronous render on the client.
  // After the first useEffect fires (client-only), it becomes true.
  // This ensures Date.now() is never called on the server.
  const [mounted, setMounted] = useState(false);

  useEffect(() => {
    setMounted(true);
  }, []);

  // Compute demo timestamps only after mount (client-side).
  // Server render uses ts_ns = 0 → formatTimestamp renders "" → no mismatch.
  const demoSignals: TradingSignal[] = DEMO_TEMPLATES.map((t) => ({
    ...t,
    ts_ns: mounted ? (Date.now() - t.offsetMs) * 1e6 : 0,
  }));

  const data = signals.length > 0 ? signals.slice(0, 10) : demoSignals;

  return (
    <div className="rounded-lg border border-gray-800 bg-bg-secondary p-4">
      <div className="mb-3 flex items-center gap-2 text-xs text-gray-500">
        <Radio size={12} className="text-accent-cyan" />
        최근 시그널
      </div>
      <div className="space-y-1.5">
        {data.map((sig, i) => (
          <div
            key={`${sig.symbol}-${i}`}
            className="flex items-center gap-4 rounded px-2 py-1.5 text-sm hover:bg-bg-tertiary"
          >
            {/*
              suppressHydrationWarning is added as defence-in-depth for live
              signals (which arrive via WebSocket and also carry dynamic ts_ns).
              The mounted guard above handles the demoSignals case structurally.
            */}
            <span
              className="w-16 font-mono text-xs text-gray-500"
              suppressHydrationWarning
            >
              {mounted ? formatTimestamp(sig.ts_ns) : ''}
            </span>
            <span className="w-24 font-medium text-white">{sig.symbol}</span>
            <span className={`w-20 ${signalColorClass(sig.direction)}`}>
              {sig.direction}
            </span>
            <span className="w-16 font-mono text-xs text-gray-400">
              z={formatZScore(sig.composite_z)}
            </span>
            <span className="w-20 font-mono text-xs text-gray-400">
              conf={sig.confidence.toFixed(2)}
            </span>
            {(sig.direction === 'StrongBuy' || sig.direction === 'Buy') && (
              <span className="text-xs text-profit">▶ 매수</span>
            )}
            {(sig.direction === 'StrongSell' || sig.direction === 'Sell') && (
              <span className="text-xs text-loss">▶ 매도</span>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}

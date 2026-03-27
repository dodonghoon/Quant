'use client';

import { useMemo, useState } from 'react';
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ReferenceLine,
  ResponsiveContainer,
} from 'recharts';
import { useTradingStore } from '@/stores/tradingStore';
import { formatTimestamp, formatZScore } from '@/lib/formatters';

export default function SignalsPage() {
  const { signals } = useTradingStore();
  const [selectedSymbol, setSelectedSymbol] = useState<string | null>(null);

  const filteredSignals = useMemo(() => {
    if (!selectedSymbol) return signals;
    return signals.filter((s: any) => s.symbol === selectedSymbol);
  }, [signals, selectedSymbol]);

  const chartData = useMemo(() => {
    return filteredSignals
      .sort((a: any, b: any) => new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime())
      .map((signal: any) => ({
        timestamp: signal.timestamp,
        z_score: parseFloat(signal.z_score),
        displayTime: formatTimestamp(signal.timestamp),
      }));
  }, [filteredSignals]);

  const symbols = useMemo(() => {
    return Array.from(new Set(signals.map((s: any) => s.symbol))) as string[];
  }, [signals]);

  const getSignalColor = (signalType: string) => {
    switch (signalType) {
      case 'ENTRY_LONG':
        return 'text-green-400';
      case 'ENTRY_SHORT':
        return 'text-red-500';
      case 'EXIT':
        return 'text-yellow-400';
      default:
        return 'text-gray-400';
    }
  };

  const getSignalBgColor = (signalType: string) => {
    switch (signalType) {
      case 'ENTRY_LONG':
        return 'bg-green-500/10 border-green-500/30';
      case 'ENTRY_SHORT':
        return 'bg-red-500/10 border-red-500/30';
      case 'EXIT':
        return 'bg-yellow-500/10 border-yellow-500/30';
      default:
        return 'bg-gray-500/10 border-gray-500/30';
    }
  };

  return (
    <div className="min-h-screen bg-primary p-6">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="mb-8">
          <h1 className="text-3xl font-bold text-primary mb-2">Trading Signals</h1>
          <p className="text-secondary text-sm">Z-score analysis and signal history</p>
        </div>

        {/* Chart Section */}
        <div className="bg-secondary rounded-lg border border-gray-700 p-6 mb-8">
          <h2 className="text-lg font-semibold text-primary mb-4">Z-Score Over Time</h2>
          
          {chartData.length > 0 ? (
            <ResponsiveContainer width="100%" height={400}>
              <LineChart data={chartData}>
                <CartesianGrid strokeDasharray="3 3" stroke="#333" />
                <XAxis
                  dataKey="displayTime"
                  stroke="#666"
                  tick={{ fill: '#999', fontSize: 12 }}
                />
                <YAxis
                  stroke="#666"
                  tick={{ fill: '#999', fontSize: 12 }}
                />
                <Tooltip
                  contentStyle={{
                    backgroundColor: '#1a1d29',
                    border: '1px solid #333',
                    borderRadius: '8px',
                  }}
                  labelStyle={{ color: '#999' }}
                  formatter={(value: number) => formatZScore(value)}
                />
                
                {/* Reference Lines */}
                <ReferenceLine
                  y={0}
                  stroke="white"
                  strokeDasharray="5 5"
                  label={{ value: '0', position: 'right', fill: '#fff', fontSize: 12 }}
                />
                <ReferenceLine
                  y={2}
                  stroke="#ef4444"
                  strokeDasharray="5 5"
                  label={{ value: '+2 (Entry)', position: 'right', fill: '#ef4444', fontSize: 11 }}
                />
                <ReferenceLine
                  y={-2}
                  stroke="#ef4444"
                  strokeDasharray="5 5"
                  label={{ value: '-2 (Entry)', position: 'right', fill: '#ef4444', fontSize: 11 }}
                />
                <ReferenceLine
                  y={0.5}
                  stroke="#22c55e"
                  strokeDasharray="5 5"
                  label={{ value: '+0.5 (Exit)', position: 'right', fill: '#22c55e', fontSize: 11 }}
                />
                <ReferenceLine
                  y={-0.5}
                  stroke="#22c55e"
                  strokeDasharray="5 5"
                  label={{ value: '-0.5 (Exit)', position: 'right', fill: '#22c55e', fontSize: 11 }}
                />
                
                <Line
                  type="monotone"
                  dataKey="z_score"
                  stroke="#06b6d4"
                  dot={false}
                  strokeWidth={2}
                  isAnimationActive={false}
                />
              </LineChart>
            </ResponsiveContainer>
          ) : (
            <div className="h-96 flex items-center justify-center text-secondary">
              No data available
            </div>
          )}
        </div>

        {/* Signals List Section */}
        <div className="bg-secondary rounded-lg border border-gray-700 p-6">
          <div className="flex items-center justify-between mb-6">
            <h2 className="text-lg font-semibold text-primary">Recent Signals</h2>
            <select
              value={selectedSymbol || ''}
              onChange={(e) => setSelectedSymbol(e.target.value || null)}
              className="bg-primary border border-gray-700 text-primary px-3 py-2 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-accent-cyan"
            >
              <option value="">All Symbols</option>
              {symbols.map((symbol) => (
                <option key={symbol} value={symbol}>
                  {symbol}
                </option>
              ))}
            </select>
          </div>

          {filteredSignals.length > 0 ? (
            <div className="space-y-3 max-h-96 overflow-y-auto">
              {filteredSignals
                .sort((a: any, b: any) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime())
                .map((signal: any, idx: number) => (
                  <div
                    key={idx}
                    className={`rounded-lg border p-4 ${getSignalBgColor(signal.signal_type)}`}
                  >
                    <div className="flex items-start justify-between">
                      <div className="flex-1">
                        <div className="flex items-center gap-3 mb-2">
                          <span className="text-sm text-secondary">
                            {formatTimestamp(signal.timestamp)}
                          </span>
                          <span className="font-mono font-semibold text-primary">
                            {signal.symbol}
                          </span>
                          <span className={`font-semibold text-sm ${getSignalColor(signal.signal_type)}`}>
                            {signal.signal_type}
                          </span>
                        </div>
                        <div className="flex items-center gap-4 text-sm">
                          <div>
                            <span className="text-secondary">Z-Score: </span>
                            <span className="text-primary font-mono">
                              {formatZScore(signal.z_score)}
                            </span>
                          </div>
                          <div>
                            <span className="text-secondary">Confidence: </span>
                            <span className="text-accent-cyan font-mono">
                              {(signal.confidence * 100).toFixed(1)}%
                            </span>
                          </div>
                        </div>
                      </div>
                    </div>
                  </div>
                ))}
            </div>
          ) : (
            <div className="h-48 flex items-center justify-center text-secondary">
              No signals available
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

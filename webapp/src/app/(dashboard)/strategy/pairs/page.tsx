'use client';

import { useEffect, useState } from 'react';
import PairTable from '@/components/strategy/PairTable';
import { useTradingStore } from '@/stores/tradingStore';
import { pairs as pairsApi } from '@/lib/api';

export default function PairsPage() {
  const { pairs, setPairs } = useTradingStore();
  const [isLoading, setIsLoading] = useState(false);

  const handleRefresh = async () => {
    setIsLoading(true);
    try {
      const pairsData = await pairsApi.getAll();
      setPairs(pairsData);
    } catch (error) {
      console.error('Failed to refresh pairs:', error);
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    if (pairs.length === 0) {
      handleRefresh();
    }
  }, []);

  const stats = pairs.length > 0 ? {
    totalPairs: pairs.length,
    activePairs: pairs.filter((p: any) => p.active).length,
    avgSpread: (pairs.reduce((sum: number, p: any) => sum + (p.spread || 0), 0) / pairs.length).toFixed(4),
    avgHalfLife: (pairs.reduce((sum: number, p: any) => sum + (p.half_life || 0), 0) / pairs.length).toFixed(2),
  } : {
    totalPairs: 0,
    activePairs: 0,
    avgSpread: '0.0000',
    avgHalfLife: '0.00',
  };

  return (
    <div className="min-h-screen bg-primary p-6">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="mb-8">
          <h1 className="text-3xl font-bold text-primary mb-2">Trading Pairs</h1>
          <p className="text-secondary text-sm">Monitor and manage cointegrated trading pairs</p>
        </div>

        {/* Refresh Button */}
        <div className="mb-6 flex justify-end">
          <button
            onClick={handleRefresh}
            disabled={isLoading}
            className="px-4 py-2 bg-accent-blue hover:bg-accent-blue/90 disabled:opacity-50 text-white rounded-lg font-medium transition-colors"
          >
            {isLoading ? 'Refreshing...' : 'Refresh Pairs'}
          </button>
        </div>

        {/* Summary Stats */}
        <div className="grid grid-cols-4 gap-4 mb-8">
          <div className="bg-secondary rounded-lg p-4 border border-gray-700">
            <p className="text-secondary text-sm mb-1">Total Pairs</p>
            <p className="text-2xl font-bold text-primary">{stats.totalPairs}</p>
          </div>
          <div className="bg-secondary rounded-lg p-4 border border-gray-700">
            <p className="text-secondary text-sm mb-1">Active Pairs</p>
            <p className="text-2xl font-bold text-accent-cyan">{stats.activePairs}</p>
          </div>
          <div className="bg-secondary rounded-lg p-4 border border-gray-700">
            <p className="text-secondary text-sm mb-1">Avg Spread</p>
            <p className="text-2xl font-bold text-primary">{stats.avgSpread}</p>
          </div>
          <div className="bg-secondary rounded-lg p-4 border border-gray-700">
            <p className="text-secondary text-sm mb-1">Avg Half-Life</p>
            <p className="text-2xl font-bold text-primary">{stats.avgHalfLife}</p>
          </div>
        </div>

        {/* Pair Table */}
        <div className="bg-secondary rounded-lg border border-gray-700 overflow-hidden">
          <PairTable />
        </div>
      </div>
    </div>
  );
}

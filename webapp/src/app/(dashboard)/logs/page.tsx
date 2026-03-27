'use client';

import { useState, useEffect } from 'react';
import DataTable from '@/components/common/DataTable';
import Badge from '@/components/ui/badge';
import Input from '@/components/ui/input';
import Button from '@/components/ui/button';
import Card from '@/components/ui/card';
import { RefreshCw, AlertCircle } from 'lucide-react';
import { auditLog } from '@/lib/api';

interface AuditLog {
  id: string;
  timestamp: string;
  level: 'INFO' | 'WARN' | 'ERROR';
  source: string;
  message: string;
}

export default function LogsPage() {
  const [logs, setLogs] = useState<AuditLog[]>([
    {
      id: '1',
      timestamp: '2024-01-20 14:35:22',
      level: 'INFO',
      source: 'BacktestEngine',
      message: 'Backtest BT-001 completed successfully',
    },
    {
      id: '2',
      timestamp: '2024-01-20 14:32:15',
      level: 'WARN',
      source: 'DataSync',
      message: 'Polygon.io sync took longer than expected (2.3s)',
    },
    {
      id: '3',
      timestamp: '2024-01-20 14:30:44',
      level: 'INFO',
      source: 'TradingEngine',
      message: 'Signal generated for pair AAPL/MSFT',
    },
    {
      id: '4',
      timestamp: '2024-01-20 14:28:11',
      level: 'ERROR',
      source: 'APIClient',
      message: 'Failed to fetch data from IQFeed - Connection timeout',
    },
    {
      id: '5',
      timestamp: '2024-01-20 14:25:33',
      level: 'INFO',
      source: 'PortfolioManager',
      message: 'Position rebalancing completed, 3 trades executed',
    },
  ]);

  const [filterLevel, setFilterLevel] = useState<'All' | 'INFO' | 'WARN' | 'ERROR'>('All');
  const [filterSource, setFilterSource] = useState('');
  const [autoRefresh, setAutoRefresh] = useState(false);

  // Auto-refresh logic
  useEffect(() => {
    if (!autoRefresh) return;

    const interval = setInterval(() => {
      // Simulate fetching new logs
      console.log('Refreshing logs...');
    }, 5000);

    return () => clearInterval(interval);
  }, [autoRefresh]);

  const filteredLogs = logs.filter((log) => {
    const levelMatch = filterLevel === 'All' || log.level === filterLevel;
    const sourceMatch = log.source.toLowerCase().includes(filterSource.toLowerCase());
    return levelMatch && sourceMatch;
  });

  const getLevelBadge = (row: AuditLog) => {
    const colors: Record<string, string> = {
      INFO: 'bg-accent-blue text-white',
      WARN: 'bg-yellow-500 text-white',
      ERROR: 'bg-loss text-white',
    };
    return <Badge className={colors[row.level] || ''}>{row.level}</Badge>;
  };

  const handleRefresh = async () => {
    try {
      const data = await auditLog.query();
      setLogs(data);
    } catch (error) {
      console.error('Failed to refresh logs:', error);
    }
  };

  return (
    <div className="min-h-screen bg-primary p-8">
      <div className="max-w-7xl mx-auto">
        <h1 className="text-4xl font-bold text-primary mb-8">Audit Logs</h1>

        {/* Filter and Controls */}
        <Card className="bg-secondary border border-border p-6 mb-8">
          <div className="grid grid-cols-1 md:grid-cols-4 gap-4 mb-4">
            {/* Level Filter */}
            <div>
              <label className="block text-sm text-secondary mb-2">Log Level</label>
              <select
                value={filterLevel}
                onChange={(e) => setFilterLevel(e.target.value as any)}
                className="w-full bg-primary border border-border rounded px-3 py-2 text-primary"
              >
                <option value="All">All Levels</option>
                <option value="INFO">INFO</option>
                <option value="WARN">WARN</option>
                <option value="ERROR">ERROR</option>
              </select>
            </div>

            {/* Source Filter */}
            <div className="md:col-span-2">
              <label className="block text-sm text-secondary mb-2">Source</label>
              <Input
                type="text"
                placeholder="Filter by source (e.g., BacktestEngine)"
                value={filterSource}
                onChange={(e) => setFilterSource(e.target.value)}
                className="bg-primary border-border text-primary"
              />
            </div>

            {/* Auto Refresh Toggle */}
            <div className="flex flex-col justify-end">
              <Button
                onClick={() => setAutoRefresh(!autoRefresh)}
                className={`${
                  autoRefresh
                    ? 'bg-accent-blue hover:bg-accent-blue/80 text-white'
                    : 'bg-secondary border border-border text-secondary hover:text-primary'
                }`}
              >
                <RefreshCw size={16} className={`mr-2 ${autoRefresh ? 'animate-spin' : ''}`} />
                {autoRefresh ? 'Refreshing (5s)' : 'Auto Refresh'}
              </Button>
            </div>
          </div>

          {/* Manual Refresh Button */}
          <div className="flex justify-end">
            <Button
              onClick={handleRefresh}
              variant="outline"
              className="border-border text-secondary hover:text-primary"
            >
              <RefreshCw size={16} className="mr-2" /> Manual Refresh
            </Button>
          </div>
        </Card>

        {/* Logs Table */}
        <Card className="bg-secondary border border-border p-6">
          {filteredLogs.length > 0 ? (
            <DataTable
              columns={[
                { key: 'timestamp', label: 'Timestamp' },
                { key: 'level', label: 'Level', render: (row: AuditLog) => getLevelBadge(row) },
                { key: 'source', label: 'Source' },
                { key: 'message', label: 'Message' },
              ]}
              data={filteredLogs}
              keyField="id"
            />
          ) : (
            <div className="flex flex-col items-center justify-center py-12 text-center">
              <AlertCircle size={40} className="text-secondary mb-4" />
              <p className="text-secondary text-lg">No logs matching your filters</p>
              <p className="text-secondary text-sm mt-2">Try adjusting your filter criteria</p>
            </div>
          )}
        </Card>

        {/* Log Stats */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-6 mt-8">
          <Card className="bg-secondary border border-border p-6">
            <p className="text-secondary text-sm mb-2">Total Logs</p>
            <p className="text-3xl font-bold text-primary">{logs.length}</p>
          </Card>

          <Card className="bg-secondary border border-border p-6">
            <p className="text-secondary text-sm mb-2">Errors (24h)</p>
            <p className="text-3xl font-bold text-loss">{logs.filter((l) => l.level === 'ERROR').length}</p>
          </Card>

          <Card className="bg-secondary border border-border p-6">
            <p className="text-secondary text-sm mb-2">Warnings (24h)</p>
            <p className="text-3xl font-bold text-yellow-500">
              {logs.filter((l) => l.level === 'WARN').length}
            </p>
          </Card>
        </div>
      </div>
    </div>
  );
}

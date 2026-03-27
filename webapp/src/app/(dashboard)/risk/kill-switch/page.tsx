'use client';

import { useState, useEffect } from 'react';
import { killSwitch as killSwitchApi, auditLog } from '@/lib/api';
import DataTable from '@/components/common/DataTable';
import { formatTimestamp } from '@/lib/formatters';

interface KillSwitchEvent {
  id: string;
  timestamp: string;
  action: 'activated' | 'deactivated';
  triggered_by: string;
  reason: string;
}

export default function KillSwitchAuditPage() {
  const [events, setEvents] = useState<KillSwitchEvent[]>([]);
  const [isCurrentlyActive, setIsCurrentlyActive] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [isOperating, setIsOperating] = useState(false);

  useEffect(() => {
    fetchKillSwitchHistory();
    const interval = setInterval(fetchKillSwitchHistory, 3000);
    return () => clearInterval(interval);
  }, []);

  const fetchKillSwitchHistory = async () => {
    try {
      setIsLoading(true);
      const [historyRes, statusRes] = await Promise.all([
        auditLog.query({ action: 'kill_switch' }),
        killSwitchApi.getStatus(),
      ]);
      setEvents(historyRes);
      setIsCurrentlyActive(statusRes.active);
    } catch (error) {
      console.error('Failed to fetch kill switch data:', error);
    } finally {
      setIsLoading(false);
    }
  };

  const handleActivate = async () => {
    try {
      setIsOperating(true);
      await killSwitchApi.activate('Manual activation from UI');
      setIsCurrentlyActive(true);
      await fetchKillSwitchHistory();
    } catch (error) {
      console.error('Failed to activate kill switch:', error);
    } finally {
      setIsOperating(false);
    }
  };

  const handleDeactivate = async () => {
    try {
      setIsOperating(true);
      await killSwitchApi.reset();
      setIsCurrentlyActive(false);
      await fetchKillSwitchHistory();
    } catch (error) {
      console.error('Failed to deactivate kill switch:', error);
    } finally {
      setIsOperating(false);
    }
  };

  const columns = [
    {
      key: 'timestamp',
      label: 'Timestamp',
      width: '25%',
      render: (row: KillSwitchEvent) => formatTimestamp(row.timestamp),
    },
    {
      key: 'action',
      label: 'Action',
      width: '15%',
      render: (row: KillSwitchEvent) => (
        <span className={`inline-block rounded px-2 py-1 text-xs font-semibold ${
          row.action === 'activated'
            ? 'bg-loss/20 text-loss'
            : 'bg-profit/20 text-profit'
        }`}>
          {row.action.charAt(0).toUpperCase() + row.action.slice(1)}
        </span>
      ),
    },
    {
      key: 'triggered_by',
      label: 'Triggered By',
      width: '20%',
      render: (row: KillSwitchEvent) => (
        <span className="font-mono text-sm text-primary">{row.triggered_by}</span>
      ),
    },
    {
      key: 'reason',
      label: 'Reason',
      width: '40%',
      render: (row: KillSwitchEvent) => <span className="text-secondary">{row.reason}</span>,
    },
  ];

  return (
    <div className="space-y-8 p-6">
      {/* Status Indicator */}
      <section className="rounded-lg border border-border bg-secondary p-6">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-4">
            <div
              className={`h-4 w-4 rounded-full ${
                isCurrentlyActive ? 'bg-loss' : 'bg-profit'
              } animate-pulse`}
            />
            <div>
              <h2 className="text-2xl font-bold text-primary">Kill Switch Status</h2>
              <p className={`mt-1 text-lg font-semibold ${
                isCurrentlyActive ? 'text-loss' : 'text-profit'
              }`}>
                {isCurrentlyActive ? 'ACTIVE - Trading Halted' : 'SAFE - Trading Enabled'}
              </p>
            </div>
          </div>

          <div className="flex gap-3">
            <button
              onClick={handleActivate}
              disabled={isOperating || isCurrentlyActive}
              className="rounded-lg bg-loss px-6 py-2 font-semibold text-white transition-all hover:bg-red-600 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {isOperating ? 'Processing...' : 'Activate'}
            </button>
            <button
              onClick={handleDeactivate}
              disabled={isOperating || !isCurrentlyActive}
              className="rounded-lg bg-accent-cyan px-6 py-2 font-semibold text-white transition-all hover:bg-cyan-600 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {isOperating ? 'Processing...' : 'Deactivate'}
            </button>
          </div>
        </div>
      </section>

      {/* Kill Switch Events Table */}
      <section className="rounded-lg border border-border bg-secondary p-6">
        <h3 className="mb-6 text-xl font-bold text-primary">Kill Switch Audit History</h3>
        {isLoading ? (
          <div className="flex items-center justify-center py-12">
            <p className="text-secondary">Loading kill switch history...</p>
          </div>
        ) : events.length === 0 ? (
          <div className="flex items-center justify-center py-12">
            <p className="text-secondary">No kill switch events recorded</p>
          </div>
        ) : (
          <DataTable
            columns={columns}
            data={events}
            keyField="id"
          />
        )}
      </section>

      {/* Statistics */}
      <section className="rounded-lg border border-border bg-secondary p-6">
        <h3 className="mb-6 text-xl font-bold text-primary">Statistics</h3>
        <div className="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-4">
          <div className="rounded-lg bg-primary p-4">
            <p className="text-sm text-secondary">Total Activations</p>
            <p className="mt-2 text-3xl font-bold text-loss">
              {events.filter((e) => e.action === 'activated').length}
            </p>
          </div>
          <div className="rounded-lg bg-primary p-4">
            <p className="text-sm text-secondary">Total Deactivations</p>
            <p className="mt-2 text-3xl font-bold text-profit">
              {events.filter((e) => e.action === 'deactivated').length}
            </p>
          </div>
          <div className="rounded-lg bg-primary p-4">
            <p className="text-sm text-secondary">Manual Triggers</p>
            <p className="mt-2 text-3xl font-bold text-accent-blue">
              {events.filter((e) => e.triggered_by === 'manual').length}
            </p>
          </div>
          <div className="rounded-lg bg-primary p-4">
            <p className="text-sm text-secondary">Automatic Triggers</p>
            <p className="mt-2 text-3xl font-bold text-accent-cyan">
              {events.filter((e) => e.triggered_by === 'automatic').length}
            </p>
          </div>
        </div>
      </section>
    </div>
  );
}

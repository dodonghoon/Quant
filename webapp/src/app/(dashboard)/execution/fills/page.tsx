'use client';

import { useEffect, useState } from 'react';
import { orders as ordersApi } from '@/lib/api';
import DataTable from '@/components/common/DataTable';
import { formatPrice, formatPnl, formatTimestamp } from '@/lib/formatters';

interface Fill {
  id: string;
  symbol: string;
  side: 'BUY' | 'SELL';
  price: number;
  quantity: number;
  fee: number;
  pnl: number;
  timestamp: string;
}

export default function FillsPage() {
  const [fills, setFills] = useState<Fill[]>([]);
  const [filteredFills, setFilteredFills] = useState<Fill[]>([]);
  const [loading, setLoading] = useState(false);
  const [fromDate, setFromDate] = useState('');
  const [toDate, setToDate] = useState('');

  const loadFills = async () => {
    setLoading(true);
    try {
      const data = await ordersApi.getFills();
      setFills(data);
      setFilteredFills(data);
    } catch (error) {
      console.error('Failed to load fills:', error);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadFills();
  }, []);

  useEffect(() => {
    let filtered = fills;

    if (fromDate) {
      const fromTime = new Date(fromDate).getTime();
      filtered = filtered.filter(
        (f) => new Date(f.timestamp).getTime() >= fromTime
      );
    }

    if (toDate) {
      const toTime = new Date(toDate).getTime();
      filtered = filtered.filter(
        (f) => new Date(f.timestamp).getTime() <= toTime
      );
    }

    setFilteredFills(filtered);
  }, [fills, fromDate, toDate]);

  const exportToCSV = () => {
    if (filteredFills.length === 0) {
      console.warn('No fills to export');
      return;
    }

    const headers = ['ID', 'Symbol', 'Side', 'Price', 'Quantity', 'Fee', 'PnL', 'Timestamp'];
    const rows = filteredFills.map((fill) => [
      fill.id,
      fill.symbol,
      fill.side,
      fill.price,
      fill.quantity,
      fill.fee,
      fill.pnl,
      fill.timestamp,
    ]);

    const csvContent = [
      headers.join(','),
      ...rows.map((row) => row.map((cell) => `"${cell}"`).join(',')),
    ].join('\n');

    const blob = new Blob([csvContent], { type: 'text/csv' });
    const url = window.URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.download = `fills_${new Date().toISOString().split('T')[0]}.csv`;
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    window.URL.revokeObjectURL(url);
  };

  const columns = [
    {
      key: 'id',
      label: 'Fill ID',
    },
    {
      key: 'symbol',
      label: 'Symbol',
    },
    {
      key: 'side',
      label: 'Side',
      render: (row: Fill) => (
        <span
          className={`font-semibold ${
            row.side === 'BUY' ? 'text-profit' : 'text-loss'
          }`}
        >
          {row.side}
        </span>
      ),
    },
    {
      key: 'price',
      label: 'Price',
      render: (row: Fill) => formatPrice(row.price),
    },
    {
      key: 'quantity',
      label: 'Quantity',
    },
    {
      key: 'fee',
      label: 'Fee',
      render: (row: Fill) => formatPrice(row.fee),
    },
    {
      key: 'pnl',
      label: 'PnL',
      render: (row: Fill) => (
        <span
          className={`font-semibold ${
            row.pnl >= 0 ? 'text-profit' : 'text-loss'
          }`}
        >
          {formatPnl(row.pnl)}
        </span>
      ),
    },
    {
      key: 'timestamp',
      label: 'Timestamp',
      render: (row: Fill) => formatTimestamp(row.timestamp),
    },
  ];

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-3xl font-bold text-primary">Fill History</h1>
        <button
          onClick={exportToCSV}
          disabled={filteredFills.length === 0}
          className="px-4 py-2 rounded bg-accent-cyan text-primary font-medium hover:bg-accent-cyan/90 transition-colors disabled:opacity-50"
        >
          Export CSV
        </button>
      </div>

      <div className="grid grid-cols-2 gap-4">
        <div>
          <label className="block text-sm font-medium text-secondary mb-2">
            From Date
          </label>
          <input
            type="datetime-local"
            value={fromDate}
            onChange={(e) => setFromDate(e.target.value)}
            className="w-full px-3 py-2 rounded bg-secondary border border-border text-primary placeholder-text-secondary focus:outline-none focus:ring-2 focus:ring-accent-blue"
          />
        </div>
        <div>
          <label className="block text-sm font-medium text-secondary mb-2">
            To Date
          </label>
          <input
            type="datetime-local"
            value={toDate}
            onChange={(e) => setToDate(e.target.value)}
            className="w-full px-3 py-2 rounded bg-secondary border border-border text-primary placeholder-text-secondary focus:outline-none focus:ring-2 focus:ring-accent-blue"
          />
        </div>
      </div>

      <div className="rounded-lg border border-border bg-primary overflow-hidden">
        <DataTable columns={columns} data={filteredFills} keyField="id" />
      </div>
    </div>
  );
}

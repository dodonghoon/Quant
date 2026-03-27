'use client';

import { useEffect, useState } from 'react';
import { orders } from '@/lib/api';
import DataTable from '@/components/common/DataTable';
import ConfirmDialog from '@/components/common/ConfirmDialog';
import { formatPrice, formatTimestamp } from '@/lib/formatters';

interface Order {
  id: string;
  symbol: string;
  side: 'BUY' | 'SELL';
  type: string;
  price: number;
  quantity: number;
  filled_qty: number;
  status: string;
  created_at: string;
}

export default function OrdersPage() {
  const [activeOrders, setActiveOrders] = useState<Order[]>([]);
  const [loading, setLoading] = useState(false);
  const [confirmDialog, setConfirmDialog] = useState<{
    open: boolean;
    orderId?: string;
    onConfirm?: () => void;
  }>({ open: false });

  const loadOrders = async () => {
    setLoading(true);
    try {
      const data = await orders.getAll();
      setActiveOrders(Array.isArray(data) ? data : []);
    } catch (error) {
      console.error('Failed to load orders:', error);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadOrders();
  }, []);

  const handleCancelOrder = (orderId: string) => {
    setConfirmDialog({
      open: true,
      orderId,
      onConfirm: async () => {
        try {
          await orders.cancel(orderId);
          await loadOrders();
          setConfirmDialog({ open: false });
        } catch (error) {
          console.error('Failed to cancel order:', error);
        }
      },
    });
  };

  const handleCancelAll = () => {
    setConfirmDialog({
      open: true,
      onConfirm: async () => {
        try {
          await Promise.all(
            activeOrders
              .filter((o) => o.status === 'OPEN' || o.status === 'PARTIALLY_FILLED')
              .map((o) => orders.cancel(o.id))
          );
          await loadOrders();
          setConfirmDialog({ open: false });
        } catch (error) {
          console.error('Failed to cancel all orders:', error);
        }
      },
    });
  };

  const columns = [
    {
      key: 'id',
      label: 'Order ID',
      render: (row: Order) => row.id.substring(0, 8) + '...',
    },
    {
      key: 'symbol',
      label: 'Symbol',
      sortable: true,
    },
    {
      key: 'side',
      label: 'Side',
      render: (row: Order) => (
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
      key: 'type',
      label: 'Type',
    },
    {
      key: 'price',
      label: 'Price',
      align: 'right' as const,
      render: (row: Order) => formatPrice(row.price),
    },
    {
      key: 'quantity',
      label: 'Quantity',
      align: 'right' as const,
    },
    {
      key: 'filled_qty',
      label: 'Filled',
      align: 'right' as const,
    },
    {
      key: 'status',
      label: 'Status',
      render: (row: Order) => (
        <span className="px-2 py-0.5 rounded bg-bg-tertiary text-xs font-medium">
          {row.status}
        </span>
      ),
    },
    {
      key: 'created_at',
      label: 'Created',
      render: (row: Order) => formatTimestamp(row.created_at),
    },
    {
      key: 'actions',
      label: '',
      render: (row: Order) => (
        <button
          onClick={(e) => { e.stopPropagation(); handleCancelOrder(row.id); }}
          className="px-3 py-1 text-xs font-medium rounded bg-loss/20 text-loss hover:bg-loss/30 transition-colors"
        >
          Cancel
        </button>
      ),
    },
  ];

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-3xl font-bold text-primary">Active Orders</h1>
        <div className="flex gap-2">
          <button
            onClick={loadOrders}
            disabled={loading}
            className="px-4 py-2 rounded bg-accent-blue text-primary font-medium hover:bg-accent-blue/90 transition-colors disabled:opacity-50"
          >
            {loading ? 'Refreshing...' : 'Refresh'}
          </button>
          {activeOrders.filter((o) => o.status === 'OPEN' || o.status === 'PARTIALLY_FILLED')
            .length > 0 && (
            <button
              onClick={handleCancelAll}
              className="px-4 py-2 rounded bg-loss/20 text-loss font-medium hover:bg-loss/30 transition-colors"
            >
              Cancel All
            </button>
          )}
        </div>
      </div>

      <div className="rounded-lg border border-border bg-primary overflow-hidden">
        <DataTable columns={columns} data={activeOrders} keyField="id" />
      </div>

      <ConfirmDialog
        open={confirmDialog.open}
        title="Confirm Cancel"
        message={
          confirmDialog.orderId
            ? `Are you sure you want to cancel order ${confirmDialog.orderId.substring(0, 8)}?`
            : 'Are you sure you want to cancel all active orders?'
        }
        onConfirm={() => {
          confirmDialog.onConfirm?.();
        }}
        onCancel={() => setConfirmDialog({ open: false })}
      />
    </div>
  );
}

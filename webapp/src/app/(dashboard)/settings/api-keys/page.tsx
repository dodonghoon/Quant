'use client';

import { useState } from 'react';
import Link from 'next/link';
import Button from '@/components/ui/button';
import DataTable from '@/components/common/DataTable';
import Badge from '@/components/ui/badge';
import Input from '@/components/ui/input';
import Card from '@/components/ui/card';
import { Eye, EyeOff, Trash2, Plus, ChevronLeft } from 'lucide-react';

interface ApiKey {
  id: string;
  name: string;
  exchange: string;
  key: string;
  created_at: string;
  status: 'active' | 'revoked';
}

export default function ApiKeysPage() {
  const [showForm, setShowForm] = useState(false);
  const [revealedKeys, setRevealedKeys] = useState<Set<string>>(new Set());
  const [formData, setFormData] = useState({ name: '', exchange: '', api_key: '', api_secret: '' });

  const apiKeys: ApiKey[] = [
    { id: '1', name: 'Binance Live', exchange: 'Binance', key: '****...3x2kL', created_at: '2024-01-10', status: 'active' },
    { id: '2', name: 'Kraken Paper', exchange: 'Kraken', key: '****...9mP2Q', created_at: '2024-01-05', status: 'active' },
    { id: '3', name: 'Old API (Revoked)', exchange: 'Coinbase', key: '****...5nR8w', created_at: '2023-12-20', status: 'revoked' },
  ];

  const toggleReveal = (id: string) => {
    setRevealedKeys((prev) => {
      const newSet = new Set(prev);
      if (newSet.has(id)) {
        newSet.delete(id);
      } else {
        newSet.add(id);
      }
      return newSet;
    });
  };

  const handleDelete = (id: string) => {
    if (confirm('Are you sure you want to delete this API key? This action cannot be undone.')) {
      console.log('Deleting key:', id);
    }
  };

  const handleSubmit = () => {
    console.log('Adding new API key:', formData);
    setFormData({ name: '', exchange: '', api_key: '', api_secret: '' });
    setShowForm(false);
  };

  const maskKey = (key: string, revealed: boolean) => {
    if (revealed) return key;
    return `****...${key.slice(-6)}`;
  };

  return (
    <div className="min-h-screen bg-primary p-8">
      <div className="max-w-6xl mx-auto">
        {/* Header */}
        <div className="flex items-center gap-4 mb-8">
          <Link href="/settings">
            <Button variant="ghost" size="icon" className="text-secondary hover:text-primary">
              <ChevronLeft size={20} />
            </Button>
          </Link>
          <h1 className="text-4xl font-bold text-primary">API Keys</h1>
        </div>

        {/* Manage Keys Section */}
        <div className="space-y-6 mb-8">
          <div className="flex justify-between items-center">
            <p className="text-secondary">Manage your exchange API credentials securely</p>
            <Button
              onClick={() => setShowForm(!showForm)}
              className="bg-accent-blue hover:bg-accent-blue/80 text-white"
            >
              <Plus size={16} className="mr-2" /> Add Key
            </Button>
          </div>

          {/* API Keys Table */}
          <DataTable
            columns={[
              { key: 'name', label: 'Name' },
              { key: 'exchange', label: 'Exchange' },
              {
                key: 'key',
                label: 'API Key',
                render: (row: ApiKey) => (
                  <div className="flex items-center gap-2">
                    <span className="font-mono text-sm">{maskKey(row.key, revealedKeys.has(row.id))}</span>
                    <button
                      onClick={() => toggleReveal(row.id)}
                      className="text-secondary hover:text-primary"
                    >
                      {revealedKeys.has(row.id) ? <EyeOff size={16} /> : <Eye size={16} />}
                    </button>
                  </div>
                ),
              },
              { key: 'created_at', label: 'Created' },
              {
                key: 'status',
                label: 'Status',
                render: (row: ApiKey) => (
                  <Badge className={row.status === 'active' ? 'bg-profit text-white' : 'bg-loss text-white'}>
                    {row.status}
                  </Badge>
                ),
              },
              {
                key: 'id',
                label: 'Actions',
                render: (row: ApiKey) => (
                  <button
                    onClick={() => handleDelete(row.id)}
                    className="text-loss hover:text-loss/80"
                  >
                    <Trash2 size={16} />
                  </button>
                ),
              },
            ]}
            data={apiKeys}
            keyField="id"
          />
        </div>

        {/* Add Key Form */}
        {showForm && (
          <Card className="bg-secondary border border-border p-6">
            <h2 className="text-xl font-semibold text-primary mb-6">Add New API Key</h2>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-6">
              <div>
                <label className="block text-sm text-secondary mb-2">Name</label>
                <Input
                  type="text"
                  placeholder="e.g., Binance Live Trading"
                  value={formData.name}
                  onChange={(e) => setFormData({ ...formData, name: e.target.value })}
                  className="bg-primary border-border text-primary"
                />
              </div>

              <div>
                <label className="block text-sm text-secondary mb-2">Exchange</label>
                <select
                  value={formData.exchange}
                  onChange={(e) => setFormData({ ...formData, exchange: e.target.value })}
                  className="w-full bg-primary border border-border rounded px-3 py-2 text-primary"
                >
                  <option value="">Select Exchange</option>
                  <option value="binance">Binance</option>
                  <option value="kraken">Kraken</option>
                  <option value="coinbase">Coinbase</option>
                  <option value="bybit">Bybit</option>
                </select>
              </div>

              <div className="md:col-span-2">
                <label className="block text-sm text-secondary mb-2">API Key</label>
                <Input
                  type="password"
                  placeholder="Enter API key"
                  value={formData.api_key}
                  onChange={(e) => setFormData({ ...formData, api_key: e.target.value })}
                  className="bg-primary border-border text-primary"
                />
              </div>

              <div className="md:col-span-2">
                <label className="block text-sm text-secondary mb-2">API Secret</label>
                <Input
                  type="password"
                  placeholder="Enter API secret"
                  value={formData.api_secret}
                  onChange={(e) => setFormData({ ...formData, api_secret: e.target.value })}
                  className="bg-primary border-border text-primary"
                />
              </div>
            </div>

            <div className="flex gap-3">
              <Button
                onClick={handleSubmit}
                className="bg-accent-blue hover:bg-accent-blue/80 text-white"
              >
                Save Key
              </Button>
              <Button
                onClick={() => setShowForm(false)}
                variant="outline"
                className="border-border text-secondary hover:text-primary"
              >
                Cancel
              </Button>
            </div>
          </Card>
        )}
      </div>
    </div>
  );
}

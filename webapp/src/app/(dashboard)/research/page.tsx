'use client';

import { useState } from 'react';
import Button from '@/components/ui/button';
import DataTable from '@/components/common/DataTable';
import Badge from '@/components/ui/badge';
import { Play, Zap, Brain, Database } from 'lucide-react';

interface BacktestRun {
  id: string;
  strategy: string;
  period: string;
  sharpe: number;
  total_return: number;
  max_dd: number;
  status: 'completed' | 'running' | 'failed';
}

interface CointegrationResult {
  pair: string;
  test_stat: number;
  p_value: number;
  half_life: number;
  is_cointegrated: boolean;
}

interface OnnxModel {
  name: string;
  version: string;
  accuracy: number;
  last_trained: string;
  status: 'active' | 'inactive';
}

interface DataSource {
  source: string;
  symbols_count: number;
  last_sync: string;
  status: 'healthy' | 'degraded' | 'error';
}

export default function ResearchPage() {
  const [activeTab, setActiveTab] = useState<'backtest' | 'cointegration' | 'onnx' | 'datalake'>('backtest');

  // Mock data
  const backtestRuns: BacktestRun[] = [
    { id: 'BT-001', strategy: 'Mean Reversion', period: '2024-01-01 to 2024-12-31', sharpe: 1.85, total_return: 23.4, max_dd: -8.2, status: 'completed' },
    { id: 'BT-002', strategy: 'Momentum', period: '2024-01-01 to 2024-12-31', sharpe: 1.42, total_return: 18.7, max_dd: -12.1, status: 'completed' },
  ];

  const cointegrationResults: CointegrationResult[] = [
    { pair: 'GOOGL/MSFT', test_stat: -3.85, p_value: 0.012, half_life: 5.2, is_cointegrated: true },
    { pair: 'AAPL/TSLA', test_stat: -2.15, p_value: 0.18, half_life: 12.4, is_cointegrated: false },
  ];

  const onnxModels: OnnxModel[] = [
    { name: 'price_predictor_v1', version: '1.0.0', accuracy: 0.847, last_trained: '2024-01-15', status: 'active' },
    { name: 'volatility_model', version: '2.1.0', accuracy: 0.923, last_trained: '2024-01-10', status: 'active' },
  ];

  const dataSources: DataSource[] = [
    { source: 'IQFeed', symbols_count: 45000, last_sync: '2024-01-20 14:32:10', status: 'healthy' },
    { source: 'Polygon.io', symbols_count: 12000, last_sync: '2024-01-20 14:31:45', status: 'healthy' },
  ];

  const getStatusBadge = (status: string) => {
    const colors: Record<string, string> = {
      completed: 'bg-profit text-white',
      running: 'bg-accent-blue text-white',
      failed: 'bg-loss text-white',
      active: 'bg-profit text-white',
      inactive: 'bg-secondary text-secondary',
      healthy: 'bg-profit text-white',
      degraded: 'bg-yellow-500 text-white',
      error: 'bg-loss text-white',
    };
    return <Badge className={colors[status] || ''}>{status}</Badge>;
  };

  return (
    <div className="min-h-screen bg-primary p-8">
      <div className="max-w-7xl mx-auto">
        <h1 className="text-4xl font-bold text-primary mb-8">Research Lab</h1>

        {/* Tab Navigation */}
        <div className="flex gap-4 mb-8 border-b border-border">
          <button
            onClick={() => setActiveTab('backtest')}
            className={`pb-3 px-4 font-medium flex items-center gap-2 ${
              activeTab === 'backtest' ? 'text-accent-blue border-b-2 border-accent-blue' : 'text-secondary'
            }`}
          >
            <Play size={18} /> Backtest
          </button>
          <button
            onClick={() => setActiveTab('cointegration')}
            className={`pb-3 px-4 font-medium flex items-center gap-2 ${
              activeTab === 'cointegration' ? 'text-accent-blue border-b-2 border-accent-blue' : 'text-secondary'
            }`}
          >
            <Zap size={18} /> Cointegration
          </button>
          <button
            onClick={() => setActiveTab('onnx')}
            className={`pb-3 px-4 font-medium flex items-center gap-2 ${
              activeTab === 'onnx' ? 'text-accent-blue border-b-2 border-accent-blue' : 'text-secondary'
            }`}
          >
            <Brain size={18} /> ONNX Models
          </button>
          <button
            onClick={() => setActiveTab('datalake')}
            className={`pb-3 px-4 font-medium flex items-center gap-2 ${
              activeTab === 'datalake' ? 'text-accent-blue border-b-2 border-accent-blue' : 'text-secondary'
            }`}
          >
            <Database size={18} /> Data Lake
          </button>
        </div>

        {/* Backtest Tab */}
        {activeTab === 'backtest' && (
          <div className="space-y-6">
            <div className="flex justify-between items-center">
              <h2 className="text-2xl font-semibold text-primary">Backtest Runs</h2>
              <Button className="bg-accent-blue hover:bg-accent-blue/80 text-white">
                <Play size={16} className="mr-2" /> Run Backtest
              </Button>
            </div>
            <DataTable
              columns={[
                { key: 'id', label: 'ID' },
                { key: 'strategy', label: 'Strategy' },
                { key: 'period', label: 'Period' },
                { key: 'sharpe', label: 'Sharpe Ratio', render: (row: BacktestRun) => row.sharpe.toFixed(2) },
                { key: 'total_return', label: 'Total Return (%)', render: (row: BacktestRun) => `${row.total_return.toFixed(2)}%` },
                { key: 'max_dd', label: 'Max Drawdown (%)', render: (row: BacktestRun) => `${row.max_dd.toFixed(2)}%` },
                { key: 'status', label: 'Status', render: (row: BacktestRun) => getStatusBadge(row.status) },
              ]}
              data={backtestRuns}
              keyField="id"
            />
          </div>
        )}

        {/* Cointegration Tab */}
        {activeTab === 'cointegration' && (
          <div className="space-y-6">
            <h2 className="text-2xl font-semibold text-primary">Pair Cointegration Results</h2>
            <DataTable
              columns={[
                { key: 'pair', label: 'Pair' },
                { key: 'test_stat', label: 'Test Statistic', render: (row: CointegrationResult) => row.test_stat.toFixed(4) },
                { key: 'p_value', label: 'P-Value', render: (row: CointegrationResult) => row.p_value.toFixed(4) },
                { key: 'half_life', label: 'Half-Life (days)', render: (row: CointegrationResult) => row.half_life.toFixed(2) },
                { key: 'is_cointegrated', label: 'Cointegrated', render: (row: CointegrationResult) => (
                  <Badge className={row.is_cointegrated ? 'bg-profit text-white' : 'bg-loss text-white'}>
                    {row.is_cointegrated ? 'Yes' : 'No'}
                  </Badge>
                )},
              ]}
              data={cointegrationResults}
              keyField="pair"
            />
          </div>
        )}

        {/* ONNX Models Tab */}
        {activeTab === 'onnx' && (
          <div className="space-y-6">
            <div className="flex justify-between items-center">
              <h2 className="text-2xl font-semibold text-primary">ONNX Model Registry</h2>
              <Button className="bg-accent-cyan hover:bg-accent-cyan/80 text-white">
                <Brain size={16} className="mr-2" /> Retrain
              </Button>
            </div>
            <DataTable
              columns={[
                { key: 'name', label: 'Model Name' },
                { key: 'version', label: 'Version' },
                { key: 'accuracy', label: 'Accuracy', render: (row: OnnxModel) => `${(row.accuracy * 100).toFixed(2)}%` },
                { key: 'last_trained', label: 'Last Trained' },
                { key: 'status', label: 'Status', render: (row: OnnxModel) => getStatusBadge(row.status) },
              ]}
              data={onnxModels}
              keyField="name"
            />
          </div>
        )}

        {/* Data Lake Tab */}
        {activeTab === 'datalake' && (
          <div className="space-y-6">
            <div className="flex justify-between items-center">
              <h2 className="text-2xl font-semibold text-primary">Data Sources</h2>
              <Button className="bg-accent-cyan hover:bg-accent-cyan/80 text-white">
                <Database size={16} className="mr-2" /> Sync All
              </Button>
            </div>
            <DataTable
              columns={[
                { key: 'source', label: 'Source' },
                { key: 'symbols_count', label: 'Symbols Count', render: (row: DataSource) => row.symbols_count.toLocaleString() },
                { key: 'last_sync', label: 'Last Sync' },
                { key: 'status', label: 'Status', render: (row: DataSource) => getStatusBadge(row.status) },
              ]}
              data={dataSources}
              keyField="source"
            />
          </div>
        )}
      </div>
    </div>
  );
}

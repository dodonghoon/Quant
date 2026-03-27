'use client';

import { useState } from 'react';
import Link from 'next/link';
import Button from '@/components/ui/button';
import Card from '@/components/ui/card';
import { ChevronLeft, Check, AlertCircle } from 'lucide-react';
import { config } from '@/lib/api';

type ConfigType = 'signal' | 'kalman' | 'garch' | 'kelly' | 'almgren';

interface ConfigTab {
  id: ConfigType;
  label: string;
  apiKey: string;
}

export default function ParametersPage() {
  const [activeTab, setActiveTab] = useState<ConfigType>('signal');
  const [configs, setConfigs] = useState<Record<ConfigType, any>>({
    signal: {
      window_size: 20,
      threshold: 2.0,
      lookback: 252,
    },
    kalman: {
      process_variance: 0.01,
      measurement_variance: 0.1,
      initial_state: 0,
    },
    garch: {
      p: 1,
      q: 1,
      alpha: 0.1,
      beta: 0.8,
    },
    kelly: {
      fraction: 0.25,
      min_probability: 0.51,
      max_bet_size: 0.1,
    },
    almgren: {
      lambda: 1e-6,
      eta: 0.5,
      time_horizon: 1.0,
      trading_rate: 0.5,
    },
  });

  const [editedConfig, setEditedConfig] = useState<string>(JSON.stringify(configs[activeTab], null, 2));
  const [validationStatus, setValidationStatus] = useState<Record<ConfigType, boolean>>({
    signal: true,
    kalman: true,
    garch: true,
    kelly: true,
    almgren: true,
  });

  const configTabs: ConfigTab[] = [
    { id: 'signal', label: 'Signal', apiKey: 'getSignal' },
    { id: 'kalman', label: 'Kalman', apiKey: 'getKalman' },
    { id: 'garch', label: 'GARCH', apiKey: 'getGarch' },
    { id: 'kelly', label: 'Kelly', apiKey: 'getKelly' },
    { id: 'almgren', label: 'Almgren-Chriss', apiKey: 'getAc' },
  ];

  const handleTabChange = (tabId: ConfigType) => {
    setActiveTab(tabId);
    setEditedConfig(JSON.stringify(configs[tabId], null, 2));
  };

  const handleConfigChange = (value: string) => {
    setEditedConfig(value);

    // Validate JSON
    try {
      JSON.parse(value);
      setValidationStatus({ ...validationStatus, [activeTab]: true });
    } catch {
      setValidationStatus({ ...validationStatus, [activeTab]: false });
    }
  };

  const handleSave = async () => {
    try {
      const parsed = JSON.parse(editedConfig);
      setConfigs({ ...configs, [activeTab]: parsed });

      // Call appropriate API method based on activeTab
      switch (activeTab) {
        case 'signal':
          await config.putSignal(parsed);
          break;
        case 'kalman':
          await config.putKalman(parsed);
          break;
        case 'garch':
          await config.putGarch(parsed);
          break;
        case 'kelly':
          await config.putKelly(parsed);
          break;
        case 'almgren':
          await config.putAc(parsed);
          break;
      }

      alert(`${activeTab} configuration saved successfully!`);
    } catch (error) {
      alert('Invalid JSON format');
    }
  };

  const isValidJson = validationStatus[activeTab];

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
          <h1 className="text-4xl font-bold text-primary">Trading Parameters</h1>
        </div>

        <p className="text-secondary mb-8">Configure JSON parameters for trading strategies and algorithms</p>

        {/* Config Tabs */}
        <Card className="bg-secondary border border-border p-6">
          {/* Tab Navigation */}
          <div className="flex gap-3 mb-8 pb-4 border-b border-border overflow-x-auto">
            {configTabs.map((tab) => (
              <button
                key={tab.id}
                onClick={() => handleTabChange(tab.id)}
                className={`px-4 py-2 rounded font-medium whitespace-nowrap transition-colors ${
                  activeTab === tab.id
                    ? 'bg-accent-blue text-white'
                    : 'bg-primary text-secondary hover:text-primary border border-border'
                }`}
              >
                {tab.label}
              </button>
            ))}
          </div>

          {/* JSON Editor */}
          <div className="space-y-4">
            <div>
              <div className="flex justify-between items-center mb-3">
                <label className="block text-sm text-secondary font-medium">Configuration JSON</label>
                <div className="flex items-center gap-2">
                  {isValidJson ? (
                    <div className="flex items-center gap-1 text-profit text-sm">
                      <Check size={16} /> Valid JSON
                    </div>
                  ) : (
                    <div className="flex items-center gap-1 text-loss text-sm">
                      <AlertCircle size={16} /> Invalid JSON
                    </div>
                  )}
                </div>
              </div>

              <textarea
                value={editedConfig}
                onChange={(e) => handleConfigChange(e.target.value)}
                className={`w-full h-96 bg-primary border-2 rounded p-4 font-mono text-sm text-primary resize-none focus:outline-none focus:ring-2 ${
                  isValidJson ? 'border-border focus:ring-profit' : 'border-loss focus:ring-loss'
                }`}
              />
            </div>

            {/* Config Info */}
            <div className="bg-primary p-4 rounded border border-border">
              <p className="text-secondary text-sm">
                <span className="font-medium">Current Config Key:</span> {configTabs.find((t) => t.id === activeTab)?.apiKey}
              </p>
            </div>

            {/* Actions */}
            <div className="flex gap-3">
              <Button
                onClick={handleSave}
                disabled={!isValidJson}
                className={`${
                  isValidJson
                    ? 'bg-accent-blue hover:bg-accent-blue/80 text-white'
                    : 'bg-secondary text-secondary cursor-not-allowed'
                }`}
              >
                <Check size={16} className="mr-2" /> Save Configuration
              </Button>

              <Button
                onClick={() => handleTabChange(activeTab)}
                variant="outline"
                className="border-border text-secondary hover:text-primary"
              >
                Reset to Last Saved
              </Button>
            </div>
          </div>

          {/* Configuration Schema Reference */}
          <div className="mt-8 pt-8 border-t border-border">
            <h3 className="text-lg font-semibold text-primary mb-4">Schema Reference</h3>
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4 text-sm">
              <div className="bg-primary p-3 rounded border border-border">
                <p className="text-accent-blue font-mono mb-2">Signal Config:</p>
                <p className="text-secondary">window_size, threshold, lookback</p>
              </div>
              <div className="bg-primary p-3 rounded border border-border">
                <p className="text-accent-cyan font-mono mb-2">Kalman Config:</p>
                <p className="text-secondary">process_variance, measurement_variance, initial_state</p>
              </div>
              <div className="bg-primary p-3 rounded border border-border">
                <p className="text-accent-blue font-mono mb-2">GARCH Config:</p>
                <p className="text-secondary">p, q, alpha, beta</p>
              </div>
              <div className="bg-primary p-3 rounded border border-border">
                <p className="text-accent-cyan font-mono mb-2">Kelly Config:</p>
                <p className="text-secondary">fraction, min_probability, max_bet_size</p>
              </div>
            </div>
          </div>
        </Card>
      </div>
    </div>
  );
}

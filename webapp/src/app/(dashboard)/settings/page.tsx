'use client';

import Link from 'next/link';
import { Key, Sliders, Info } from 'lucide-react';
import Card from '@/components/ui/card';

export default function SettingsPage() {
  return (
    <div className="min-h-screen bg-primary p-8">
      <div className="max-w-4xl mx-auto">
        <h1 className="text-4xl font-bold text-primary mb-2">Settings</h1>
        <p className="text-secondary mb-8">Manage your trading dashboard configuration</p>

        {/* Settings Cards */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6 mb-8">
          {/* API Keys Card */}
          <Link href="/settings/api-keys">
            <Card className="bg-secondary border border-border hover:border-accent-blue transition-colors cursor-pointer h-full">
              <div className="p-6">
                <div className="flex items-center gap-3 mb-3">
                  <Key size={24} className="text-accent-blue" />
                  <h2 className="text-xl font-semibold text-primary">API Keys</h2>
                </div>
                <p className="text-secondary text-sm">
                  Manage exchange API credentials and authentication tokens
                </p>
              </div>
            </Card>
          </Link>

          {/* Parameters Card */}
          <Link href="/settings/parameters">
            <Card className="bg-secondary border border-border hover:border-accent-cyan transition-colors cursor-pointer h-full">
              <div className="p-6">
                <div className="flex items-center gap-3 mb-3">
                  <Sliders size={24} className="text-accent-cyan" />
                  <h2 className="text-xl font-semibold text-primary">Parameters</h2>
                </div>
                <p className="text-secondary text-sm">
                  Configure trading strategies, algorithms, and risk parameters
                </p>
              </div>
            </Card>
          </Link>
        </div>

        {/* System Info Section */}
        <Card className="bg-secondary border border-border p-6">
          <div className="flex items-center gap-3 mb-6">
            <Info size={24} className="text-accent-blue" />
            <h2 className="text-xl font-semibold text-primary">System Information</h2>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
            <div>
              <p className="text-secondary text-sm mb-2">Version</p>
              <p className="text-primary font-mono text-lg">v1.2.4</p>
            </div>

            <div>
              <p className="text-secondary text-sm mb-2">Uptime</p>
              <p className="text-primary font-mono text-lg">28 days, 14h</p>
            </div>

            <div>
              <p className="text-secondary text-sm mb-2">Environment</p>
              <p className="text-primary font-mono text-lg">production</p>
            </div>
          </div>

          <div className="mt-6 pt-6 border-t border-border">
            <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
              <div>
                <p className="text-secondary text-sm mb-2">API Status</p>
                <div className="flex items-center gap-2">
                  <div className="w-2 h-2 bg-profit rounded-full"></div>
                  <p className="text-primary">Operational</p>
                </div>
              </div>

              <div>
                <p className="text-secondary text-sm mb-2">Database Status</p>
                <div className="flex items-center gap-2">
                  <div className="w-2 h-2 bg-profit rounded-full"></div>
                  <p className="text-primary">Connected</p>
                </div>
              </div>

              <div>
                <p className="text-secondary text-sm mb-2">Cache Status</p>
                <div className="flex items-center gap-2">
                  <div className="w-2 h-2 bg-profit rounded-full"></div>
                  <p className="text-primary">Healthy</p>
                </div>
              </div>
            </div>
          </div>
        </Card>
      </div>
    </div>
  );
}

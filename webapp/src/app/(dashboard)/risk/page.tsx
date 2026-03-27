'use client';

import { useState, useEffect } from 'react';
import { useTradingStore } from '@/stores/tradingStore';
import { killSwitch as killSwitchApi, config, positions as positionsApi } from '@/lib/api';
import ConfirmDialog from '@/components/common/ConfirmDialog';
import SliderInput from '@/components/common/SliderInput';
import { formatPnl, formatPercent } from '@/lib/formatters';

interface RiskConfig {
  max_position_pct: number;
  max_drawdown_pct: number;
  max_order_notional: number;
  max_daily_loss: number;
}

interface RiskMetrics {
  drawdown_pct: number;
  var_pct: number;
  exposure_pct: number;
}

interface PositionRisk {
  symbol: string;
  position_size: number;
  current_pnl: number;
  risk_contribution_pct: number;
  delta: number;
}

export default function RiskPage() {
  const { killSwitch, activateKillSwitch, deactivateKillSwitch } = useTradingStore();
  const [riskConfig, setRiskConfig] = useState<RiskConfig>({
    max_position_pct: 5,
    max_drawdown_pct: 10,
    max_order_notional: 500000,
    max_daily_loss: 50000,
  });
  const [riskMetrics, setRiskMetrics] = useState<RiskMetrics>({
    drawdown_pct: 0,
    var_pct: 0,
    exposure_pct: 0,
  });
  const [positionRisks, setPositionRisks] = useState<PositionRisk[]>([]);
  const [isKillSwitchDialogOpen, setIsKillSwitchDialogOpen] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [isSaving, setIsSaving] = useState(false);

  useEffect(() => {
    fetchRiskData();
    const interval = setInterval(fetchRiskData, 5000);
    return () => clearInterval(interval);
  }, []);

  const fetchRiskData = async () => {
    try {
      setIsLoading(true);
      const [configRes, positionsRes] = await Promise.all([
        config.getRisk(),
        positionsApi.getAll(),
      ]);
      setRiskConfig(configRes);
      setRiskMetrics(riskMetrics); // Use current metrics from state
      setPositionRisks(positionsRes);
    } catch (error) {
      console.error('Failed to fetch risk data:', error);
    } finally {
      setIsLoading(false);
    }
  };

  const handleKillSwitch = async () => {
    try {
      setIsLoading(true);
      await killSwitchApi.activate('Manual activation from UI');
      activateKillSwitch();
      setIsKillSwitchDialogOpen(false);
    } catch (error) {
      console.error('Failed to activate kill switch:', error);
    } finally {
      setIsLoading(false);
    }
  };

  const handleSaveRiskConfig = async () => {
    try {
      setIsSaving(true);
      await config.putRisk(riskConfig);
      console.log('Risk config saved successfully');
    } catch (error) {
      console.error('Failed to save risk config:', error);
    } finally {
      setIsSaving(false);
    }
  };

  const handleConfigChange = (key: keyof RiskConfig, value: number) => {
    setRiskConfig((prev) => ({
      ...prev,
      [key]: value,
    }));
  };

  return (
    <div className="space-y-8 p-6">
      {/* Kill Switch Section */}
      <section className="rounded-lg border border-border bg-secondary p-6">
        <div className="flex items-center justify-between">
          <div>
            <h2 className="text-2xl font-bold text-primary">Emergency Kill Switch</h2>
            <p className="mt-1 text-sm text-secondary">
              Stop all trading immediately and liquidate positions
            </p>
          </div>
          <div className="text-right">
            <p className="text-sm font-medium text-secondary">Status</p>
            <p className={`mt-1 text-lg font-bold ${killSwitch.active ? 'text-loss' : 'text-profit'}`}>
              {killSwitch.active ? 'ACTIVE' : 'INACTIVE'}
            </p>
            {killSwitch.timestamp && (
              <p className="mt-2 text-xs text-secondary">
                {new Date(killSwitch.timestamp).toLocaleString()}
              </p>
            )}
          </div>
        </div>

        <button
          onClick={() => setIsKillSwitchDialogOpen(true)}
          disabled={isLoading || killSwitch.active}
          className="mt-6 rounded-lg bg-loss px-6 py-3 font-semibold text-white transition-all hover:bg-red-600 disabled:opacity-50 disabled:cursor-not-allowed"
        >
          Activate Kill Switch
        </button>
      </section>

      {/* Risk Metrics Section */}
      <section className="rounded-lg border border-border bg-secondary p-6">
        <h3 className="text-xl font-bold text-primary">Current Risk Metrics</h3>
        <div className="mt-6 grid grid-cols-1 gap-6 md:grid-cols-3">
          {/* Drawdown Bar */}
          <div>
            <div className="flex items-center justify-between">
              <span className="text-sm font-medium text-secondary">Drawdown</span>
              <span className="font-semibold text-primary">
                {formatPercent(riskMetrics.drawdown_pct)}
              </span>
            </div>
            <div className="mt-2 h-2 w-full rounded-full bg-primary">
              <div
                className="h-full rounded-full bg-accent-cyan transition-all"
                style={{
                  width: `${Math.min(riskMetrics.drawdown_pct, 100)}%`,
                }}
              />
            </div>
            <p className="mt-2 text-xs text-secondary">Max: {formatPercent(riskConfig.max_drawdown_pct)}</p>
          </div>

          {/* VaR Bar */}
          <div>
            <div className="flex items-center justify-between">
              <span className="text-sm font-medium text-secondary">Value at Risk (99%)</span>
              <span className="font-semibold text-primary">
                {formatPercent(riskMetrics.var_pct)}
              </span>
            </div>
            <div className="mt-2 h-2 w-full rounded-full bg-primary">
              <div
                className="h-full rounded-full bg-accent-blue transition-all"
                style={{
                  width: `${Math.min(riskMetrics.var_pct, 100)}%`,
                }}
              />
            </div>
          </div>

          {/* Exposure Bar */}
          <div>
            <div className="flex items-center justify-between">
              <span className="text-sm font-medium text-secondary">Portfolio Exposure</span>
              <span className="font-semibold text-primary">
                {formatPercent(riskMetrics.exposure_pct)}
              </span>
            </div>
            <div className="mt-2 h-2 w-full rounded-full bg-primary">
              <div
                className="h-full rounded-full bg-profit transition-all"
                style={{
                  width: `${Math.min(riskMetrics.exposure_pct, 100)}%`,
                }}
              />
            </div>
          </div>
        </div>
      </section>

      {/* Risk Configuration Section */}
      <section className="rounded-lg border border-border bg-secondary p-6">
        <h3 className="text-xl font-bold text-primary">Risk Configuration</h3>
        <div className="mt-6 grid grid-cols-1 gap-8 md:grid-cols-2 lg:grid-cols-4">
          <SliderInput
            label="Max Position Size"
            value={riskConfig.max_position_pct}
            min={0}
            max={100}
            step={0.5}
            onChange={(value) => handleConfigChange('max_position_pct', value)}
            unit="%"
          />
          <SliderInput
            label="Max Drawdown"
            value={riskConfig.max_drawdown_pct}
            min={0}
            max={50}
            step={0.5}
            onChange={(value) => handleConfigChange('max_drawdown_pct', value)}
            unit="%"
          />
          <SliderInput
            label="Max Order Notional"
            value={riskConfig.max_order_notional}
            min={0}
            max={1000000}
            step={10000}
            onChange={(value) => handleConfigChange('max_order_notional', value)}
            unit="$"
          />
          <SliderInput
            label="Max Daily Loss"
            value={riskConfig.max_daily_loss}
            min={0}
            max={100000}
            step={1000}
            onChange={(value) => handleConfigChange('max_daily_loss', value)}
            unit="$"
          />
        </div>

        <button
          onClick={handleSaveRiskConfig}
          disabled={isSaving}
          className="mt-6 rounded-lg bg-accent-blue px-6 py-2 font-semibold text-white transition-all hover:bg-blue-600 disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {isSaving ? 'Saving...' : 'Save Configuration'}
        </button>
      </section>

      {/* Position Risk Table */}
      <section className="rounded-lg border border-border bg-secondary p-6">
        <h3 className="text-xl font-bold text-primary">Position Risk Contribution</h3>
        <div className="mt-6 overflow-x-auto">
          <table className="w-full">
            <thead>
              <tr className="border-b border-border">
                <th className="px-4 py-3 text-left text-sm font-semibold text-secondary">Symbol</th>
                <th className="px-4 py-3 text-right text-sm font-semibold text-secondary">Position Size</th>
                <th className="px-4 py-3 text-right text-sm font-semibold text-secondary">Current PnL</th>
                <th className="px-4 py-3 text-right text-sm font-semibold text-secondary">Risk Contribution</th>
                <th className="px-4 py-3 text-right text-sm font-semibold text-secondary">Delta</th>
              </tr>
            </thead>
            <tbody>
              {positionRisks.map((position, idx) => (
                <tr key={idx} className="border-b border-border hover:bg-primary">
                  <td className="px-4 py-3 font-medium text-primary">{position.symbol}</td>
                  <td className="px-4 py-3 text-right text-primary">{position.position_size.toFixed(2)}</td>
                  <td className={`px-4 py-3 text-right font-semibold ${position.current_pnl >= 0 ? 'text-profit' : 'text-loss'}`}>
                    {formatPnl(position.current_pnl)}
                  </td>
                  <td className="px-4 py-3 text-right text-accent-cyan font-semibold">
                    {formatPercent(position.risk_contribution_pct)}
                  </td>
                  <td className="px-4 py-3 text-right text-accent-blue">{position.delta.toFixed(2)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>

      <ConfirmDialog
        open={isKillSwitchDialogOpen}
        title="Activate Emergency Kill Switch?"
        message="This will immediately stop all trading and liquidate all positions. This action cannot be undone. Are you absolutely sure?"
        confirmLabel="Yes, Activate Kill Switch"
        cancelLabel="Cancel"
        onConfirm={handleKillSwitch}
        onCancel={() => setIsKillSwitchDialogOpen(false)}
        danger={true}
      />
    </div>
  );
}

import { create } from 'zustand';
import type { Position, TradingSignal, Order, RiskMetrics, SystemStatus, DailyPnl } from '@/types/trading';

interface KillSwitch {
  active: boolean;
  reason?: string;
}

interface AuthState {
  token: string;
  role: string;
}

type ConnectionState = 'connecting' | 'connected' | 'disconnected';

interface TradingState {
  // Data
  dailyPnl: DailyPnl | null;
  positions: Position[];
  signals: TradingSignal[];
  activeOrders: Record<number, Order>;
  riskMetrics: RiskMetrics | null;
  systemStatus: SystemStatus | null;
  killSwitch: KillSwitch;
  auth: AuthState | null;
  pairs: any[];
  connectionState: ConnectionState;

  // Actions
  setDailyPnl: (pnl: DailyPnl) => void;
  setPositions: (positions: Position[]) => void;
  addSignal: (signal: TradingSignal) => void;
  setActiveOrders: (orders: Record<number, Order>) => void;
  setRiskMetrics: (metrics: RiskMetrics) => void;
  setSystemStatus: (status: SystemStatus) => void;
  activateKillSwitch: (reason?: string) => void;
  deactivateKillSwitch: () => void;
  setAuth: (auth: AuthState | null) => void;
  clearAuth: () => void;
  setPairs: (pairs: any[]) => void;
  setConnectionState: (state: ConnectionState) => void;
}

export const useTradingStore = create<TradingState>((set) => ({
  // Initial state
  dailyPnl: null,
  positions: [],
  signals: [],
  activeOrders: {},
  riskMetrics: null,
  systemStatus: null,
  killSwitch: { active: false },
  auth: null,
  pairs: [],
  connectionState: 'disconnected',

  // Actions
  setDailyPnl: (pnl) => set({ dailyPnl: pnl }),
  setPositions: (positions) => set({ positions }),
  addSignal: (signal) =>
    set((state) => ({
      signals: [signal, ...state.signals].slice(0, 100),
    })),
  setActiveOrders: (orders) => set({ activeOrders: orders }),
  setRiskMetrics: (metrics) => set({ riskMetrics: metrics }),
  setSystemStatus: (status) => set({ systemStatus: status }),
  activateKillSwitch: (reason) =>
    set({ killSwitch: { active: true, reason } }),
  deactivateKillSwitch: () => set({ killSwitch: { active: false } }),
  setAuth: (auth) => set({ auth }),
  clearAuth: () => set({ auth: null }),
  setPairs: (pairs) => set({ pairs }),
  setConnectionState: (connectionState) => set({ connectionState }),
}));

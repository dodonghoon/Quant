// Core types for the trading system

export interface DailyPnl {
  total: number;
  realized?: number;
  unrealized?: number;
  timestamp?: number;
}

export interface Position {
  symbol: string;
  quantity: number;
  unrealized_pnl: number;
  entry_price?: number;
  current_price?: number;
}

export interface TradingSignal {
  symbol: string;
  direction: 'StrongBuy' | 'Buy' | 'Neutral' | 'Sell' | 'StrongSell';
  composite_z: number;
  confidence: number;
  raw_position_frac: number;
  ts_ns: number;
  alpha: {
    ou_z: number;
    ou_weight: number;
    kalman_innovation: number;
    kalman_weight: number;
  };
}

export type OrderSide = 'Buy' | 'Sell';
export type OrderType = 'Market' | 'Limit';
export type OrderStatus = 'Pending' | 'Sent' | 'Filled' | 'Cancelled' | 'Rejected';

export interface Order {
  internal_id: number;
  symbol: string;
  side: OrderSide;
  order_type: OrderType;
  quantity: number;
  price: number | null;
  status: OrderStatus;
  timestamp?: number;
}

export interface RiskMetrics {
  daily_pnl: number;
  max_daily_loss: number;
  total_exposure: number;
  max_exposure: number;
  max_position_used: number;
  max_position_limit: number;
  order_rate: number;
  max_order_rate: number;
}

export interface SystemStatus {
  feed: 'connected' | 'disconnected' | 'error';
  strategy: 'running' | 'paused' | 'stopped' | 'error';
  execution: 'running' | 'paused' | 'stopped' | 'error';
  latency_us: number;
}

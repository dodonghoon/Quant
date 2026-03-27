import type { TradingSignal, Order, FillReport, RiskMetrics, SystemStatus, ModelState } from './trading';

// WebSocket 채널 타입
export type WsChannel = 'market-data' | 'signals' | 'orders' | 'risk' | 'system' | 'models';

// 구독 메시지
export interface SubscribeMessage {
  action: 'subscribe' | 'unsubscribe';
  channels: WsChannel[];
  symbols?: string[];
}

// 서버에서 오는 메시지
export type WsMessage =
  | { channel: 'market-data'; data: MarketDataEvent }
  | { channel: 'signals'; data: TradingSignal }
  | { channel: 'orders'; data: OrderEvent }
  | { channel: 'risk'; data: RiskMetrics }
  | { channel: 'system'; data: SystemStatus }
  | { channel: 'models'; data: ModelUpdateEvent };

// 시장 데이터 이벤트
export interface BboSnapshot {
  symbol: string;
  bid_price: number;
  bid_qty: number;
  ask_price: number;
  ask_qty: number;
  ts_ns: number;
}

export interface TradeEvent {
  symbol: string;
  price: number;
  quantity: number;
  side: 'Buy' | 'Sell';
  ts_ns: number;
}

export type MarketDataEvent =
  | { type: 'bbo'; data: BboSnapshot }
  | { type: 'trade'; data: TradeEvent };

// 주문 이벤트
export interface OrderEvent {
  event: 'new' | 'fill' | 'cancel' | 'reject';
  order: Order;
  fill?: FillReport;
}

// 모델 업데이트
export interface ModelUpdateEvent {
  symbol: string;
  models: ModelState;
}

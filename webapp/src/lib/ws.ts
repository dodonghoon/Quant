/**
 * WebSocket 클라이언트
 *
 * 자동 재연결, 채널 구독 관리
 */

type WsEventHandler = (data: any) => void;

interface WsOptions {
  url: string;
  onOpen?: () => void;
  onClose?: () => void;
  onError?: (error: Event) => void;
  reconnectInterval?: number;
  maxReconnectAttempts?: number;
}

export class WsClient {
  private ws: WebSocket | null = null;
  private handlers: Map<string, Set<WsEventHandler>> = new Map();
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private reconnectAttempts = 0;
  private intentionalClose = false;
  private options: Required<WsOptions>;

  constructor(options: WsOptions) {
    this.options = {
      reconnectInterval: 3000,
      maxReconnectAttempts: 20,
      onOpen: () => {},
      onClose: () => {},
      onError: () => {},
      ...options,
    };
  }

  connect() {
    this.intentionalClose = false;
    this.reconnectAttempts = 0;
    this._connect();
  }

  private _connect() {
    try {
      this.ws = new WebSocket(this.options.url);

      this.ws.onopen = () => {
        this.reconnectAttempts = 0;
        this.options.onOpen();
      };

      this.ws.onmessage = (event) => {
        try {
          const message = JSON.parse(event.data);
          const channel = message.channel as string;
          if (channel && this.handlers.has(channel)) {
            this.handlers.get(channel)!.forEach((handler) => handler(message.data));
          }
          // 글로벌 핸들러
          if (this.handlers.has('*')) {
            this.handlers.get('*')!.forEach((handler) => handler(message));
          }
        } catch (err) {
          console.error('[WS] Failed to parse message:', err);
        }
      };

      this.ws.onclose = () => {
        this.options.onClose();
        if (!this.intentionalClose) {
          this._scheduleReconnect();
        }
      };

      this.ws.onerror = (error) => {
        this.options.onError(error);
      };
    } catch (err) {
      console.error('[WS] Connection failed:', err);
      this._scheduleReconnect();
    }
  }

  private _scheduleReconnect() {
    if (this.reconnectAttempts >= this.options.maxReconnectAttempts) {
      console.error('[WS] Max reconnect attempts reached');
      return;
    }

    this.reconnectAttempts++;
    const delay = this.options.reconnectInterval * Math.min(this.reconnectAttempts, 5);

    this.reconnectTimer = setTimeout(() => {
      console.log(`[WS] Reconnecting... (attempt ${this.reconnectAttempts})`);
      this._connect();
    }, delay);
  }

  disconnect() {
    this.intentionalClose = true;
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
  }

  subscribe(channel: string, handler: WsEventHandler) {
    if (!this.handlers.has(channel)) {
      this.handlers.set(channel, new Set());
    }
    this.handlers.get(channel)!.add(handler);

    // 서버에 구독 요청 전송
    this.send({
      action: 'subscribe',
      channels: [channel],
    });

    // unsubscribe 함수 반환
    return () => {
      this.handlers.get(channel)?.delete(handler);
      if (this.handlers.get(channel)?.size === 0) {
        this.handlers.delete(channel);
        this.send({ action: 'unsubscribe', channels: [channel] });
      }
    };
  }

  send(data: unknown) {
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(data));
    }
  }

  get isConnected(): boolean {
    return this.ws?.readyState === WebSocket.OPEN;
  }

  get connectionState(): 'connecting' | 'connected' | 'disconnected' {
    if (!this.ws) return 'disconnected';
    switch (this.ws.readyState) {
      case WebSocket.CONNECTING: return 'connecting';
      case WebSocket.OPEN: return 'connected';
      default: return 'disconnected';
    }
  }
}

// 싱글톤 인스턴스들
const WS_BASE = typeof window !== 'undefined'
  ? `ws://${window.location.hostname}:8080`
  : 'ws://127.0.0.1:8080';

export function createMarketDataWs() {
  return new WsClient({ url: `${WS_BASE}/ws/market-data` });
}

export function createSignalsWs() {
  return new WsClient({ url: `${WS_BASE}/ws/signals` });
}

export function createOrdersWs() {
  return new WsClient({ url: `${WS_BASE}/ws/orders` });
}

export function createRiskWs() {
  return new WsClient({ url: `${WS_BASE}/ws/risk` });
}

export function createSystemWs() {
  return new WsClient({ url: `${WS_BASE}/ws/system` });
}

export function createModelsWs() {
  return new WsClient({ url: `${WS_BASE}/ws/models` });
}

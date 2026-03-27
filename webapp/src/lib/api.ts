/**
 * REST API 클라이언트
 *
 * JWT 자동 첨부, 에러 핸들링, 토큰 자동 갱신
 */

const API_BASE = '/api/v1';

let accessToken: string | null = null;
let refreshToken: string | null = null;

export function setTokens(access: string, refresh: string) {
  accessToken = access;
  refreshToken = refresh;
}

export function clearTokens() {
  accessToken = null;
  refreshToken = null;
}

export function getAccessToken(): string | null {
  return accessToken;
}

async function refreshAccessToken(): Promise<boolean> {
  if (!refreshToken) return false;

  try {
    const res = await fetch(`${API_BASE}/auth/refresh`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ refresh_token: refreshToken }),
    });

    if (!res.ok) return false;

    const data = await res.json();
    accessToken = data.access_token;
    return true;
  } catch {
    return false;
  }
}

export class ApiError extends Error {
  constructor(
    public status: number,
    public statusText: string,
    public body?: unknown
  ) {
    super(`API Error ${status}: ${statusText}`);
    this.name = 'ApiError';
  }
}

async function request<T>(
  path: string,
  options: RequestInit = {}
): Promise<T> {
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    ...(options.headers as Record<string, string> || {}),
  };

  if (accessToken) {
    headers['Authorization'] = `Bearer ${accessToken}`;
  }

  let res = await fetch(`${API_BASE}${path}`, {
    ...options,
    headers,
  });

  // 401이면 토큰 갱신 시도
  if (res.status === 401 && refreshToken) {
    const refreshed = await refreshAccessToken();
    if (refreshed) {
      headers['Authorization'] = `Bearer ${accessToken}`;
      res = await fetch(`${API_BASE}${path}`, { ...options, headers });
    }
  }

  if (!res.ok) {
    const body = await res.json().catch(() => null);
    throw new ApiError(res.status, res.statusText, body);
  }

  return res.json();
}

// === API 메서드 ===

// 인증
export const auth = {
  login: (username: string, password: string) =>
    request<{ access_token: string; refresh_token: string; expires_in: number; role: string }>(
      '/auth/login',
      { method: 'POST', body: JSON.stringify({ username, password }) }
    ),
  refresh: () =>
    request<{ access_token: string; expires_in: number }>(
      '/auth/refresh',
      { method: 'POST', body: JSON.stringify({ refresh_token: refreshToken }) }
    ),
};

// 시스템 상태
export const system = {
  getStatus: () => request<any>('/status'),
  healthCheck: () => request<any>('/health'),
};

// 포지션 & PnL
export const positions = {
  getAll: () => request<any>('/positions'),
  getDailyPnl: () => request<any>('/pnl/daily'),
  getPnlHistory: (from?: string, to?: string) => {
    const params = new URLSearchParams();
    if (from) params.set('from', from);
    if (to) params.set('to', to);
    const qs = params.toString();
    return request<any>(`/pnl/history${qs ? `?${qs}` : ''}`);
  },
};

// 주문
export const orders = {
  getAll: (query?: { status?: string; symbol?: string; limit?: number }) => {
    const params = new URLSearchParams();
    if (query?.status) params.set('status', query.status);
    if (query?.symbol) params.set('symbol', query.symbol);
    if (query?.limit) params.set('limit', String(query.limit));
    const qs = params.toString();
    return request<any>(`/orders${qs ? `?${qs}` : ''}`);
  },
  getById: (id: string) => request<any>(`/orders/${id}`),
  cancel: (id: string) => request<any>(`/orders/${id}`, { method: 'DELETE' }),
  getFills: (query?: { symbol?: string; limit?: number }) => {
    const params = new URLSearchParams();
    if (query?.symbol) params.set('symbol', query.symbol);
    if (query?.limit) params.set('limit', String(query.limit));
    const qs = params.toString();
    return request<any>(`/fills${qs ? `?${qs}` : ''}`);
  },
};

// 시그널
export const signals = {
  getLatest: () => request<any>('/signals/latest'),
  getHistory: (pair?: string, limit?: number) => {
    const params = new URLSearchParams();
    if (pair) params.set('pair', pair);
    if (limit) params.set('limit', String(limit));
    const qs = params.toString();
    return request<any>(`/signals/history${qs ? `?${qs}` : ''}`);
  },
};

// 모델
export const models = {
  getKalman: (symbol: string) => request<any>(`/models/kalman/${symbol}`),
  getOu: (pair: string) => request<any>(`/models/ou/${pair}`),
  getGarch: (symbol: string) => request<any>(`/models/garch/${symbol}`),
};

// Kill Switch
export const killSwitch = {
  getStatus: () => request<any>('/kill-switch'),
  activate: (reason: string) =>
    request<any>('/kill-switch/activate', {
      method: 'POST',
      body: JSON.stringify({ reason }),
    }),
  reset: () => request<any>('/kill-switch/reset', { method: 'POST' }),
};

// 설정
export const config = {
  getSignal: () => request<any>('/config/signal'),
  putSignal: (cfg: any) => request<any>('/config/signal', { method: 'PUT', body: JSON.stringify(cfg) }),
  getRisk: () => request<any>('/config/risk'),
  putRisk: (cfg: any) => request<any>('/config/risk', { method: 'PUT', body: JSON.stringify(cfg) }),
  getKelly: () => request<any>('/config/kelly'),
  putKelly: (cfg: any) => request<any>('/config/kelly', { method: 'PUT', body: JSON.stringify(cfg) }),
  getKalman: () => request<any>('/config/kalman'),
  putKalman: (cfg: any) => request<any>('/config/kalman', { method: 'PUT', body: JSON.stringify(cfg) }),
  getGarch: () => request<any>('/config/garch'),
  putGarch: (cfg: any) => request<any>('/config/garch', { method: 'PUT', body: JSON.stringify(cfg) }),
  getAc: () => request<any>('/config/almgren-chriss'),
  putAc: (cfg: any) => request<any>('/config/almgren-chriss', { method: 'PUT', body: JSON.stringify(cfg) }),
};

// 페어
export const pairs = {
  getAll: () => request<any>('/pairs'),
  add: (pair: { leg_a: string; leg_b: string; hedge_ratio: number }) =>
    request<any>('/pairs', { method: 'POST', body: JSON.stringify(pair) }),
  remove: (id: string) => request<any>(`/pairs/${id}`, { method: 'DELETE' }),
};

// 감사 로그
export const auditLog = {
  query: (params?: { level?: string; action?: string; limit?: number; offset?: number }) => {
    const qs = new URLSearchParams();
    if (params?.level) qs.set('level', params.level);
    if (params?.action) qs.set('action', params.action);
    if (params?.limit) qs.set('limit', String(params.limit));
    if (params?.offset) qs.set('offset', String(params.offset));
    const q = qs.toString();
    return request<any>(`/audit-log${q ? `?${q}` : ''}`);
  },
};

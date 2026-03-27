// API 응답 래퍼 타입
export interface ApiResponse<T> {
  status: 'ok' | 'error';
  data?: T;
  message?: string;
}

export interface PaginatedResponse<T> {
  status: 'ok';
  data: T[];
  total_count: number;
  has_more: boolean;
}

// 주문 조회 쿼리
export interface OrderQuery {
  status?: string;
  symbol?: string;
  side?: string;
  limit?: number;
  offset?: number;
}

// 감사 로그 조회 쿼리
export interface AuditLogQuery {
  level?: string;
  action?: string;
  limit?: number;
  offset?: number;
}

// 인증
export interface LoginRequest {
  username: string;
  password: string;
}

export interface LoginResponse {
  access_token: string;
  refresh_token: string;
  expires_in: number;
  role: 'Viewer' | 'Operator' | 'Admin';
}

export interface RefreshResponse {
  access_token: string;
  expires_in: number;
}

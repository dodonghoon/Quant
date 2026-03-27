/**
 * 포맷팅 유틸리티
 *
 * PnL, 가격, 시간, 퍼센트 등의 포맷팅
 */

/**
 * 가격을 포맷합니다 (소수점 2자리, 콤마 구분)
 */
export function formatPrice(price: number, decimals = 2): string {
  return price.toLocaleString('en-US', {
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals,
  });
}

/**
 * PnL을 +/- 부호와 함께 포맷합니다
 */
export function formatPnl(pnl: number, decimals = 2): string {
  const sign = pnl >= 0 ? '+' : '';
  return `${sign}$${formatPrice(Math.abs(pnl), decimals)}`;
}

/**
 * 퍼센트를 포맷합니다
 */
export function formatPercent(value: number, decimals = 1): string {
  const sign = value >= 0 ? '+' : '';
  return `${sign}${(value * 100).toFixed(decimals)}%`;
}

/**
 * 수량을 포맷합니다
 */
export function formatQuantity(qty: number, decimals = 4): string {
  return qty.toLocaleString('en-US', {
    minimumFractionDigits: 0,
    maximumFractionDigits: decimals,
  });
}

/**
 * 나노초 타임스탬프를 시간 문자열로 변환
 */
export function formatTimestamp(tsNs: number): string {
  const ms = tsNs / 1_000_000;
  const date = new Date(ms);
  return date.toLocaleTimeString('ko-KR', {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false,
  });
}

/**
 * 날짜를 포맷합니다
 */
export function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleDateString('ko-KR', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
  });
}

/**
 * 지연시간을 포맷합니다 (µs / ms)
 */
export function formatLatency(us: number): string {
  if (us < 1000) return `${us}µs`;
  return `${(us / 1000).toFixed(1)}ms`;
}

/**
 * 업타임을 사람이 읽을 수 있는 형식으로 변환
 */
export function formatUptime(secs: number): string {
  const hours = Math.floor(secs / 3600);
  const minutes = Math.floor((secs % 3600) / 60);
  const seconds = secs % 60;

  if (hours > 0) return `${hours}h ${minutes}m`;
  if (minutes > 0) return `${minutes}m ${seconds}s`;
  return `${seconds}s`;
}

/**
 * Z-score를 포맷합니다
 */
export function formatZScore(z: number): string {
  const sign = z >= 0 ? '+' : '';
  return `${sign}${z.toFixed(2)}σ`;
}

/**
 * 큰 수를 약어로 표시 (1.2K, 3.5M 등)
 */
export function formatCompact(value: number): string {
  const absValue = Math.abs(value);
  if (absValue >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}M`;
  if (absValue >= 1_000) return `${(value / 1_000).toFixed(1)}K`;
  return value.toFixed(0);
}

/**
 * PnL 색상 CSS 클래스 반환
 */
export function pnlColorClass(value: number): string {
  if (value > 0) return 'text-profit';
  if (value < 0) return 'text-loss';
  return 'text-neutral';
}

/**
 * 시그널 방향에 따른 색상 CSS 클래스
 */
export function signalColorClass(direction: string): string {
  switch (direction) {
    case 'StrongBuy': return 'text-profit font-bold';
    case 'Buy': return 'text-profit';
    case 'Sell': return 'text-loss';
    case 'StrongSell': return 'text-loss font-bold';
    default: return 'text-neutral';
  }
}

/**
 * 상태에 따른 도트 색상
 */
export function statusDotClass(status: string): string {
  switch (status) {
    case 'connected':
    case 'running':
    case 'active':
      return 'bg-green-500';
    case 'degraded':
    case 'weak':
      return 'bg-yellow-500';
    case 'disconnected':
    case 'stopped':
    case 'error':
    case 'inactive':
      return 'bg-red-500';
    default:
      return 'bg-gray-500';
  }
}

/**
 * cn 유틸리티 - tailwind merge + clsx
 */
export function cn(...classes: (string | undefined | null | false)[]): string {
  return classes.filter(Boolean).join(' ');
}

# 퀀트 트레이딩 대시보드 — 웹앱 기술문서 (v1.0)

> Quant Trading System의 모니터링, 제어, 분석을 위한 웹 애플리케이션 설계 명세

---

## 1. 개요

### 1.1 목적

기존 Rust/Python 퀀트 트레이딩 시스템의 전체 파이프라인을 실시간으로 모니터링하고 제어하는 웹 대시보드입니다. 터미널이나 로그 파일이 아닌 시각적 인터페이스를 통해 트레이딩 시스템의 상태를 한눈에 파악하고, 파라미터 조정과 긴급 제어를 즉시 수행할 수 있도록 합니다.

### 1.2 핵심 요구사항

1. **실시간성**: 시장 데이터, 시그널, 주문 상태를 밀리초 단위로 반영
2. **제어 가능성**: Kill Switch, 파라미터 조정, 주문 취소 등 즉각적 시스템 제어
3. **분석 기능**: 히스토리컬 PnL, 전략 성과, 리스크 메트릭 시각화
4. **안전성**: 위험한 조작에 대한 확인 절차, 권한 관리, 감사 로그

### 1.3 설계 원칙

- **단일 진실 원천(Single Source of Truth)**: 모든 상태는 Rust 엔진이 관리하고, 웹앱은 이를 조회/제어하는 클라이언트
- **비침투적 통합**: 기존 Rust 크레이트 코드를 최소한으로 변경하여 웹 API 계층 추가
- **장애 격리**: 웹앱 장애가 트레이딩 엔진에 영향을 주지 않음
- **모바일 대응**: 긴급 Kill Switch 제어를 위한 반응형 디자인

---

## 2. 시스템 아키텍처

### 2.1 전체 토폴로지

```
┌────────────────────────────────────────────────────────────────┐
│                     Web Browser (Client)                       │
│                                                                │
│  ┌─ Dashboard ──┐  ┌─ Market ──┐  ┌─ Strategy ─┐  ┌─ Risk ─┐ │
│  │ PnL, 포지션  │  │ 호가, 차트│  │ 시그널,    │  │ Kill   │ │
│  │ 시스템 상태  │  │ 체결 내역 │  │ 파라미터   │  │ Switch │ │
│  └──────────────┘  └──────────┘  └────────────┘  └────────┘ │
│                                                                │
│  React 18 + TypeScript + TailwindCSS + Zustand                │
│  TradingView Lightweight Charts + Recharts                     │
└──────────────────────┬─────────────────────────────────────────┘
                       │  HTTP (REST) + WebSocket
                       ▼
┌──────────────────────────────────────────────────────────────────┐
│                   API Gateway (Rust — Axum)                      │
│                                                                  │
│  ┌─ REST API ───────────────┐  ┌─ WebSocket Server ───────────┐ │
│  │ GET  /api/v1/status      │  │ ws://host/ws/market-data     │ │
│  │ GET  /api/v1/positions   │  │ ws://host/ws/signals         │ │
│  │ POST /api/v1/kill-switch │  │ ws://host/ws/orders          │ │
│  │ PUT  /api/v1/config/...  │  │ ws://host/ws/risk            │ │
│  │ GET  /api/v1/orders      │  │ ws://host/ws/system          │ │
│  └──────────────────────────┘  └──────────────────────────────┘ │
│                                                                  │
│  Auth (JWT) │ Rate Limit │ Audit Log │ CORS                     │
└──────┬───────────────┬───────────────┬───────────────────────────┘
       │               │               │
       ▼               ▼               ▼
┌─────────────┐ ┌─────────────┐ ┌──────────────┐
│ data-       │ │ strategy-   │ │ execution-   │
│ ingestion   │ │ engine      │ │ engine       │
│ (Layer 1)   │ │ (Layer 2)   │ │ (Layer 3)    │
└─────────────┘ └─────────────┘ └──────────────┘
```

### 2.2 기술 스택

#### Frontend

| 기술 | 버전 | 용도 |
|------|------|------|
| React | 18+ | UI 프레임워크 |
| TypeScript | 5.x | 타입 안전성 |
| Next.js | 14+ | SSR/라우팅/번들링 |
| TailwindCSS | 3.x | 유틸리티 스타일링 |
| Zustand | 4.x | 경량 상태 관리 |
| TradingView Lightweight Charts | 4.x | 캔들/라인 차트 |
| Recharts | 2.x | 대시보드 차트 (PnL, 파이 등) |
| React Query (TanStack) | 5.x | 서버 상태 캐싱 & 동기화 |
| Lucide React | — | 아이콘 |
| Sonner | — | 토스트 알림 |

#### Backend (API Gateway)

| 기술 | 버전 | 용도 |
|------|------|------|
| Axum | 0.7+ | HTTP/WebSocket 서버 |
| tokio | 1.x | 비동기 런타임 (기존 공유) |
| tower | 0.4+ | 미들웨어 (rate limit, CORS, auth) |
| serde / serde_json | 1.x | JSON 직렬화 |
| jsonwebtoken | 9.x | JWT 인증 토큰 |
| tracing | 0.1 | 구조화 로깅 (기존 공유) |
| tokio-tungstenite | 0.21 | WebSocket 서버 |
| sqlx | 0.7+ | SQLite/PostgreSQL (감사 로그, 설정 저장) |

---

## 3. 페이지 구성 및 UI 설계

### 3.1 페이지 맵

```
/                          → 대시보드 (메인)
/market                    → 실시간 시장 데이터
/market/:symbol            → 심볼별 상세 (차트 + 호가창)
/strategy                  → 전략 모니터링 & 파라미터
/strategy/pairs            → 페어 관리
/strategy/signals          → 시그널 히스토리
/execution                 → 주문 집행 현황
/execution/orders          → 주문 내역 (필터/검색)
/execution/fills           → 체결 내역
/risk                      → 리스크 대시보드
/risk/kill-switch          → Kill Switch 제어 (전용 페이지)
/research                  → 백테스트 & 모델 성과
/settings                  → 시스템 설정
/settings/api-keys         → 거래소 API 키 관리
/settings/parameters       → 전략/리스크 파라미터 일괄 설정
/logs                      → 감사 로그 & 시스템 로그
```

### 3.2 메인 대시보드 (/)

한눈에 시스템 전체 상태를 파악하는 화면입니다.

```
┌─────────────────────────────────────────────────────────────────┐
│ [🔴 KILL SWITCH]  Quant Dashboard          ⏱ 09:32:15 KST     │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌── 오늘의 PnL ──┐  ┌── 포지션 ──────┐  ┌── 시스템 상태 ──┐  │
│  │                 │  │                │  │                  │  │
│  │  +$342.50       │  │  BTC: +0.15    │  │  Feed: ● 정상   │  │
│  │  ▲ +1.2%        │  │  ETH: -2.30    │  │  Strategy: ● 정상│  │
│  │  [PnL 곡선]     │  │  SOL: +45.0    │  │  Exec: ● 정상   │  │
│  │                 │  │                │  │  Latency: 42µs   │  │
│  └─────────────────┘  └────────────────┘  └──────────────────┘  │
│                                                                 │
│  ┌── 최근 시그널 ─────────────────────────────────────────────┐ │
│  │ 09:32:14  BTC-ETH  StrongBuy  z=-2.7  conf=0.89  ▶ 매수   │ │
│  │ 09:32:10  SOL-AVAX Neutral    z=+0.3  conf=0.45           │ │
│  │ 09:31:58  BTC-ETH  Buy       z=-1.8  conf=0.72  ▶ 매수   │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                 │
│  ┌── 활성 주문 ────────────────┐  ┌── 리스크 요약 ──────────┐  │
│  │ #1042  BTC  Buy   0.15      │  │ 일일 손실: $342 / $1000 │  │
│  │        Limit $67,200        │  │ 총 노출: 85% / 200%     │  │
│  │        Status: Sent         │  │ 최대 포지션: 12% / 10%  │  │
│  │ #1041  ETH  Sell  2.30      │  │ 주문률: 12/s / 50/s     │  │
│  │        Market               │  │                         │  │
│  │        Status: Filled ✓     │  │ [██████░░░░] 34% 한도   │  │
│  └─────────────────────────────┘  └─────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

**구성 요소:**
- **Kill Switch 버튼**: 상단 고정. 빨간색 토글. 활성화 시 전체 화면 빨간 테두리로 경고
- **PnL 카드**: 오늘의 실현/미실현 PnL, 미니 라인 차트
- **포지션 요약**: 심볼별 현재 수량 + 미실현 PnL
- **시스템 상태**: 각 Layer 연결 상태 (녹색/노랑/빨강), 최근 지연 시간
- **최근 시그널 피드**: 실시간 스트리밍. 방향/Z-score/신뢰도 표시
- **활성 주문**: 미체결 주문 목록, 상태별 색상 구분
- **리스크 게이지**: 일일 손실 한도, 총 노출, 주문률을 프로그레스바로 표시

### 3.3 시장 데이터 페이지 (/market/:symbol)

```
┌─────────────────────────────────────────────────────────────────┐
│ BTC-USDT (Binance)          Last: 67,234.50  ▲ +2.3%          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌── 캔들차트 (TradingView) ──────────────────────────────────┐│
│  │                                                             ││
│  │  [가격 캔들]                                                ││
│  │  [Kalman Filtered Price 오버레이 (파란 선)]                 ││
│  │  [볼린저 밴드 / Z-score 밴드]                               ││
│  │                                                             ││
│  │  서브차트 1: [OU Z-Score 라인]  Entry/Exit 임계값 수평선    ││
│  │  서브차트 2: [GARCH 변동성 곡선]                            ││
│  │  서브차트 3: [거래량 막대]                                  ││
│  └─────────────────────────────────────────────────────────────┘│
│                                                                 │
│  ┌── 호가창 ──────┐  ┌── 최근 체결 ──────┐  ┌── 모델 상태 ──┐ │
│  │ Ask            │  │ 67,235  0.12 Buy  │  │ Kalman:       │ │
│  │ 67,240  1.20   │  │ 67,234  0.05 Sell │  │  gain: 0.032  │ │
│  │ 67,238  0.85   │  │ 67,235  0.30 Buy  │  │  innov: -1.2  │ │
│  │ ─── mid ────── │  │ 67,233  0.10 Sell │  │ OU:           │ │
│  │ 67,235  2.10   │  │ 67,234  0.50 Buy  │  │  κ: 0.045     │ │
│  │ 67,233  0.60   │  │                   │  │  half: 15.4s  │ │
│  │ Bid            │  │                   │  │  z: -1.82     │ │
│  └────────────────┘  └───────────────────┘  └───────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

### 3.4 전략 모니터링 페이지 (/strategy)

```
┌─────────────────────────────────────────────────────────────────┐
│ Strategy Engine                                   Status: ● ON  │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌── 시그널 설정 (라이브 조정) ──────────────────────────────┐  │
│  │                                                           │  │
│  │  Entry Threshold     [━━━━━━━○━━━] 1.5σ     (0.5 ~ 3.0) │  │
│  │  Strong Entry        [━━━━━━━━━○━] 2.5σ     (1.5 ~ 4.0) │  │
│  │  Exit Threshold      [━━○━━━━━━━━] 0.5σ     (0.1 ~ 1.5) │  │
│  │  OU Weight           [━━━━━━━○━━━] 0.70     (0.0 ~ 1.0) │  │
│  │  Kalman Weight       [━━━○━━━━━━━] 0.30     (auto = 1-OU)│  │
│  │  Min Confidence      [━━━○━━━━━━━] 0.30     (0.0 ~ 1.0) │  │
│  │                                                           │  │
│  │  [적용]  [기본값 복원]  [프리셋: 공격적 / 보수적 / 기본]  │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                 │
│  ┌── 등록된 페어 ─────────────────────────────────────────────┐ │
│  │ Pair         Hedge   κ       µ       z-score  Status       │ │
│  │ BTC-ETH      0.052   0.045   -0.12   -1.82    Active ●    │ │
│  │ SOL-AVAX     0.830   0.021   +0.05   +0.31    Active ●    │ │
│  │ BNB-SOL      1.240   0.008   +0.22   +1.05    Weak ●      │ │
│  │                                                            │ │
│  │ [+ 페어 추가]  [공적분 스캔]                               │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                 │
│  ┌── Kalman / GARCH 설정 ────────────────────────────────────┐  │
│  │  Process Noise (Q)     [1e-5]   Measurement Noise (R) [1e-3]│ │
│  │  GARCH α               [0.06]   GARCH β              [0.90]│ │
│  │  Initial Variance      [0.0004]                             │ │
│  │  [적용]                                                     │ │
│  └─────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
```

### 3.5 리스크 & Kill Switch 페이지 (/risk)

```
┌─────────────────────────────────────────────────────────────────┐
│ Risk Management                Kill Switch: [🟢 비활성]         │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌── Kill Switch 제어 ───────────────────────────────────────┐  │
│  │                                                           │  │
│  │        ┌──────────────────────────────┐                   │  │
│  │        │     ⚠ 비상 정지 (KILL)       │                   │  │
│  │        │                              │                   │  │
│  │        │  [클릭하여 활성화]            │                   │  │
│  │        │  활성화 시 모든 신규 주문 차단│                   │  │
│  │        └──────────────────────────────┘                   │  │
│  │                                                           │  │
│  │  사유 선택: ○ 수동  ○ 일일 손실  ○ 포지션 한도           │  │
│  │           ○ 피드 끊김  ○ 연속 실패  ○ 리스크 이상        │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                 │
│  ┌── 리스크 한도 설정 ───────────────────────────────────────┐  │
│  │  일일 최대 손실       [$1,000]   현재: -$342 (34%)       │  │
│  │  심볼당 최대 포지션   [100.0]    최대 사용: 45.0 (45%)   │  │
│  │  총 노출 한도         [2.0x]     현재: 0.85x (43%)       │  │
│  │  최대 주문 크기       [10.0]                              │  │
│  │  초당 최대 주문       [50]       현재: 12/s (24%)         │  │
│  │  연속 실패 허용       [5]        현재: 0                  │  │
│  │  총 자본             [$100,000]                           │  │
│  │  [적용]  [기본값 복원]                                    │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                 │
│  ┌── 포지션 상세 ────────────────────────────────────────────┐  │
│  │ Symbol   Qty      Avg Entry    Unrealized  Realized  Total│ │
│  │ BTC      +0.150   $67,100      +$20.18    +$312.00  +$332│ │
│  │ ETH      -2.300   $3,520       -$4.60     +$28.50   +$24 │ │
│  │ SOL      +45.00   $142.30      +$11.25    -$25.00   -$14 │ │
│  │ ─────────────────────────────────────────────────────────│ │
│  │ 합계                           +$26.83    +$315.50  +$342│ │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                 │
│  ┌── Kelly Criterion 설정 ───────────────────────────────────┐  │
│  │  Kelly Fraction      [0.25]     (풀 Kelly의 25%)          │  │
│  │  Max Position %      [0.10]     (자본의 10%)              │  │
│  │  Min Position %      [0.001]                              │  │
│  │  Risk-free Rate      [0.05]     (연 5%)                   │  │
│  │  Min Win Rate        [0.50]     (50% 미만 시 미매매)      │  │
│  │  [적용]                                                    │  │
│  └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

### 3.6 주문 집행 페이지 (/execution)

주요 구성 요소:
- **활성 주문 테이블**: 실시간 상태 업데이트, 개별 주문 취소 버튼
- **체결 히스토리**: 시간 역순, 필터(심볼/사이드/상태), CSV 내보내기
- **Almgren-Chriss 시뮬레이터**: 슬라이더로 파라미터 조정 → 예상 집행 경로 시각화
- **집행 통계**: 시그널 수신 수, 주문 전송/거절/체결 수, 평균 체결 시간

### 3.7 리서치 페이지 (/research)

Python 연구 계층과의 연동 화면:
- **백테스트 결과 뷰어**: vectorbt 결과를 차트로 표시 (Sharpe, MDD, 수익곡선)
- **공적분 스캐너**: 등록된 심볼 조합의 p-value 히트맵
- **모델 성과 비교**: ONNX 모델 버전별 정확도, 방향 적중률
- **데이터 레이크 상태**: Zarr/Parquet 저장 용량, 심볼별 데이터 기간

---

## 4. API 설계

### 4.1 REST API 엔드포인트

#### 시스템 상태

```
GET  /api/v1/status
  → { feed: "connected", strategy: "running", execution: "running",
      kill_switch: false, uptime_secs: 3600, latency_us: 42 }

GET  /api/v1/health
  → { status: "ok", layers: { ingestion: true, strategy: true, execution: true } }
```

#### 포지션 & PnL

```
GET  /api/v1/positions
  → [{ symbol: "BTC", quantity: 0.15, avg_entry: 67100.0,
       unrealized_pnl: 20.18, realized_pnl: 312.0 }, ...]

GET  /api/v1/pnl/daily
  → { date: "2026-02-08", realized: 315.50, unrealized: 26.83,
      total: 342.33, drawdown: -45.20 }

GET  /api/v1/pnl/history?from=2026-01-01&to=2026-02-08
  → [{ date: "...", total: ... }, ...]
```

#### 주문

```
GET  /api/v1/orders?status=active&symbol=BTC&limit=50
  → [{ internal_id: 1042, exchange_id: "binance_1042",
       symbol: "BTC", side: "Buy", order_type: "Limit",
       quantity: 0.15, price: 67200.0, status: "Sent",
       filled_qty: 0.0, created_at_ns: ... }, ...]

GET  /api/v1/orders/:id
  → { ... 단일 주문 상세 ... }

DELETE /api/v1/orders/:id
  → { status: "cancelled" }

GET  /api/v1/fills?symbol=BTC&limit=100
  → [{ internal_id: 1041, exchange_id: "...", filled_qty: 2.30,
       fill_price: 3520.0, ts_ns: ... }, ...]
```

#### 시그널

```
GET  /api/v1/signals/latest
  → [{ symbol: "BTC-ETH", direction: "StrongBuy",
       composite_z: -2.7, confidence: 0.89,
       raw_position_frac: 0.62, ts_ns: ...,
       alpha: { ou_z: -2.8, ou_weight: 0.7,
                kalman_innovation: -1.2, kalman_weight: 0.3 } }, ...]

GET  /api/v1/signals/history?pair=BTC-ETH&limit=500
  → [{ ... 시간순 시그널 이력 ... }]
```

#### 전략 모델 상태

```
GET  /api/v1/models/kalman/:symbol
  → { estimated_price: 67234.5, gain: 0.032, innovation: -1.2,
      estimation_error: 0.0012, tick_count: 45230 }

GET  /api/v1/models/ou/:pair
  → { z_score: -1.82, spread: -45.2, is_mean_reverting: true,
      params: { kappa: 0.045, mu: -0.12, sigma: 0.034,
                half_life: 15.4, r_squared: 0.87 } }

GET  /api/v1/models/garch/:symbol
  → { variance: 0.00042, volatility: 0.0205, persistence: 0.96,
      long_run_volatility: 0.018, sample_count: 12000 }
```

#### Kill Switch

```
GET  /api/v1/kill-switch
  → { active: false, reason: null, activated_at_ns: null }

POST /api/v1/kill-switch/activate
  Body: { reason: "ManualIntervention" }
  → { active: true, reason: "ManualIntervention", activated_at_ns: ... }

POST /api/v1/kill-switch/reset
  → { active: false }
```

#### 설정 (Config)

```
GET  /api/v1/config/signal
  → { entry_threshold: 1.5, strong_entry_threshold: 2.5,
      exit_threshold: 0.5, ou_weight: 0.7, kalman_weight: 0.3,
      min_confidence: 0.3 }

PUT  /api/v1/config/signal
  Body: { entry_threshold: 1.8, ou_weight: 0.6, kalman_weight: 0.4 }
  → { status: "applied", previous: { ... }, current: { ... } }

GET  /api/v1/config/risk
PUT  /api/v1/config/risk

GET  /api/v1/config/kelly
PUT  /api/v1/config/kelly

GET  /api/v1/config/kalman
PUT  /api/v1/config/kalman

GET  /api/v1/config/garch
PUT  /api/v1/config/garch

GET  /api/v1/config/almgren-chriss
PUT  /api/v1/config/almgren-chriss
```

#### 페어 관리

```
GET    /api/v1/pairs
  → [{ leg_a: "BTC", leg_b: "ETH", hedge_ratio: 0.052, status: "active" }]

POST   /api/v1/pairs
  Body: { leg_a: "SOL", leg_b: "AVAX", hedge_ratio: 0.83 }

DELETE /api/v1/pairs/:pair_id
```

#### 감사 로그

```
GET  /api/v1/audit-log?limit=100
  → [{ ts: "...", user: "admin", action: "config.signal.update",
       detail: { entry_threshold: { from: 1.5, to: 1.8 } } }, ...]
```

### 4.2 WebSocket 채널

| 채널 | 경로 | 메시지 형식 | 빈도 |
|------|------|------------|------|
| 시장 데이터 | `ws://host/ws/market-data?symbols=BTC,ETH` | `{ type: "bbo"\|"trade", data: {...} }` | 매 틱 |
| 시그널 | `ws://host/ws/signals` | `{ symbol: "BTC-ETH", direction: "...", z: -2.7, ... }` | 시그널 변경 시 |
| 주문 | `ws://host/ws/orders` | `{ event: "new"\|"fill"\|"cancel", order: {...} }` | 주문 이벤트 시 |
| 리스크 | `ws://host/ws/risk` | `{ daily_pnl: ..., exposure: ..., kill_switch: false }` | 1초 |
| 시스템 | `ws://host/ws/system` | `{ cpu: ..., memory: ..., latency: ..., feed_status: ... }` | 5초 |
| 모델 | `ws://host/ws/models?symbols=BTC,ETH` | `{ kalman: {...}, garch: {...}, ou: {...} }` | 1초 |

WebSocket 연결은 구독(subscribe) 모델입니다. 연결 후 관심 채널과 심볼을 등록합니다:

```json
// 클라이언트 → 서버
{ "action": "subscribe", "channels": ["signals", "orders", "risk"] }
{ "action": "subscribe", "channels": ["market-data"], "symbols": ["BTC", "ETH"] }

// 서버 → 클라이언트
{ "channel": "signals", "data": { "symbol": "BTC-ETH", ... } }
{ "channel": "orders", "data": { "event": "fill", "order": { ... } } }
```

---

## 5. 인증 및 보안

### 5.1 인증 방식

- **JWT (JSON Web Token)**: 로그인 시 Access Token (15분) + Refresh Token (7일) 발급
- **환경**: 로컬 네트워크 전용 (0.0.0.0 바인딩 금지, 127.0.0.1 또는 VPN 내부만 허용)
- **HTTPS**: Production 환경에서 필수 (Let's Encrypt 또는 self-signed)

### 5.2 권한 레벨

| 레벨 | 설명 | 가능한 작업 |
|------|------|------------|
| Viewer | 읽기 전용 | 대시보드 조회, 차트, 주문/시그널 이력 |
| Operator | 제한적 제어 | Kill Switch 활성화, 주문 취소 |
| Admin | 전체 제어 | 파라미터 변경, 페어 추가/삭제, API 키 관리, Kill Switch 해제 |

### 5.3 감사 로그

모든 상태 변경 작업(Kill Switch, 파라미터 변경, 주문 취소 등)은 감사 로그에 기록됩니다:
- 타임스탬프, 사용자, 액션, 이전/이후 값, IP 주소

### 5.4 위험 조작 보호

- Kill Switch **활성화**: 단일 클릭 (긴급 상황이므로 빠르게)
- Kill Switch **해제**: Admin 권한 + 확인 다이얼로그 + 5초 대기
- 파라미터 변경: 변경 전/후 비교 화면 + 확인 필요
- API 키 표시: 마스킹 처리, 복사 시 감사 로그

---

## 6. 실시간 데이터 아키텍처

### 6.1 데이터 흐름 (Rust 엔진 → 웹앱)

```
[Rust 트레이딩 엔진]
    │
    ├─ broadcast::Sender<MarketEvent>     → 시장 데이터 팬아웃
    ├─ broadcast::Sender<TradingSignal>   → 시그널 팬아웃
    ├─ broadcast::Sender<OrderEvent>      → 주문 이벤트
    └─ watch::Sender<SystemMetrics>       → 시스템 메트릭 (최신값)
         │
         ▼
    [Axum WebSocket Handler]
         │
         ├─ 클라이언트별 구독 필터
         ├─ JSON 직렬화
         └─ 백프레셔 처리 (느린 클라이언트 감지 → 드롭)
              │
              ▼
    [브라우저 WebSocket]
         │
         ▼
    [Zustand Store] → React 컴포넌트 리렌더링
```

### 6.2 백프레셔 전략

웹 클라이언트가 처리하지 못하는 속도로 데이터가 유입될 경우:

1. **Throttle**: 시장 데이터는 최대 10Hz로 제한 (100ms 간격 최신값만 전송)
2. **Drop**: 버퍼 초과 시 오래된 메시지 폐기 (최신값 우선)
3. **Snapshot + Delta**: 초기 연결 시 전체 스냅샷 전송, 이후 변경분만 전송

### 6.3 프론트엔드 상태 관리

```typescript
// Zustand 스토어 구조
interface TradingStore {
  // 시장 데이터
  bbos: Map<string, BboSnapshot>;        // 심볼별 최신 BBO
  trades: Map<string, Trade[]>;           // 심볼별 최근 체결 (ring buffer)

  // 전략
  signals: TradingSignal[];               // 최근 시그널 목록
  models: Map<string, ModelState>;        // 심볼별 모델 상태 (Kalman, OU, GARCH)
  pairs: PairInfo[];                      // 등록된 페어

  // 집행
  activeOrders: Map<number, Order>;       // 활성 주문
  recentFills: FillReport[];              // 최근 체결

  // 리스크
  positions: Map<string, PositionTracker>;
  dailyPnl: number;
  killSwitch: { active: boolean; reason: string | null };
  riskMetrics: RiskMetrics;

  // 설정
  signalConfig: SignalConfig;
  riskConfig: RiskConfig;
  kellyConfig: KellyConfig;

  // 시스템
  systemStatus: SystemStatus;
  connectionState: 'connecting' | 'connected' | 'disconnected';

  // WebSocket 액션
  connect: () => void;
  disconnect: () => void;
}
```

---

## 7. 백엔드 크레이트 구조

기존 프로젝트에 `web-dashboard` 크레이트를 추가합니다.

```
crates/
└── web-dashboard/
    ├── Cargo.toml
    └── src/
        ├── main.rs              # Axum 서버 진입점
        ├── lib.rs               # 모듈 루트
        ├── config.rs            # 서버 설정 (포트, CORS, JWT 시크릿)
        ├── auth/
        │   ├── mod.rs
        │   ├── jwt.rs           # JWT 생성/검증
        │   └── middleware.rs    # 인증 미들웨어
        ├── routes/
        │   ├── mod.rs
        │   ├── status.rs        # GET /api/v1/status, /health
        │   ├── positions.rs     # GET /api/v1/positions, /pnl
        │   ├── orders.rs        # GET/DELETE /api/v1/orders
        │   ├── signals.rs       # GET /api/v1/signals
        │   ├── models.rs        # GET /api/v1/models/...
        │   ├── kill_switch.rs   # GET/POST /api/v1/kill-switch
        │   ├── config.rs        # GET/PUT /api/v1/config/...
        │   ├── pairs.rs         # CRUD /api/v1/pairs
        │   └── audit.rs         # GET /api/v1/audit-log
        ├── ws/
        │   ├── mod.rs
        │   ├── handler.rs       # WebSocket 연결 관리
        │   ├── channels.rs      # 채널 구독/발행
        │   └── throttle.rs      # 백프레셔 & 스로틀링
        ├── bridge/
        │   ├── mod.rs
        │   ├── engine_bridge.rs # StrategyEngine ↔ API 브릿지
        │   ├── exec_bridge.rs   # ExecutionEngine ↔ API 브릿지
        │   └── feed_bridge.rs   # FeedHandler ↔ API 브릿지
        └── audit/
            ├── mod.rs
            └── logger.rs        # 감사 로그 (SQLite)
```

### 7.1 Cargo.toml (web-dashboard)

```toml
[package]
name = "web-dashboard"
version = "0.1.0"
edition = "2021"

[dependencies]
data-ingestion = { path = "../data-ingestion" }
strategy-engine = { path = "../strategy-engine" }
execution-engine = { path = "../execution-engine" }

# Web 서버
axum = { version = "0.7", features = ["ws", "json", "macros"] }
tower = { version = "0.4", features = ["limit", "timeout"] }
tower-http = { version = "0.5", features = ["cors", "trace", "compression-gzip"] }
tokio = { version = "1", features = ["full"] }

# 직렬화
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# 인증
jsonwebtoken = "9"
argon2 = "0.5"          # 비밀번호 해싱

# 감사 로그 저장
sqlx = { version = "0.7", features = ["sqlite", "runtime-tokio"] }

# 유틸리티
chrono = "0.4"
tracing = "0.1"
uuid = { version = "1", features = ["v4"] }
```

---

## 8. 프론트엔드 디렉토리 구조

```
webapp/
├── package.json
├── next.config.ts
├── tailwind.config.ts
├── tsconfig.json
├── public/
│   └── favicon.ico
└── src/
    ├── app/                       # Next.js App Router
    │   ├── layout.tsx                 글로벌 레이아웃 (사이드바, 헤더)
    │   ├── page.tsx                   대시보드 (/)
    │   ├── market/
    │   │   ├── page.tsx               시장 개요
    │   │   └── [symbol]/page.tsx      심볼별 상세
    │   ├── strategy/
    │   │   ├── page.tsx               전략 모니터링
    │   │   ├── pairs/page.tsx         페어 관리
    │   │   └── signals/page.tsx       시그널 히스토리
    │   ├── execution/
    │   │   ├── page.tsx               집행 현황
    │   │   ├── orders/page.tsx        주문 내역
    │   │   └── fills/page.tsx         체결 내역
    │   ├── risk/
    │   │   ├── page.tsx               리스크 대시보드
    │   │   └── kill-switch/page.tsx   Kill Switch 전용
    │   ├── research/page.tsx          백테스트 결과
    │   ├── settings/
    │   │   ├── page.tsx               일반 설정
    │   │   ├── api-keys/page.tsx      API 키 관리
    │   │   └── parameters/page.tsx    전체 파라미터
    │   └── logs/page.tsx              감사 로그
    │
    ├── components/
    │   ├── layout/
    │   │   ├── Sidebar.tsx            사이드 네비게이션
    │   │   ├── Header.tsx             상단 바 (Kill Switch, 시계)
    │   │   └── ConnectionStatus.tsx   WebSocket 연결 상태
    │   ├── dashboard/
    │   │   ├── PnlCard.tsx            PnL 요약 카드
    │   │   ├── PositionSummary.tsx    포지션 요약
    │   │   ├── SystemHealth.tsx       시스템 상태 표시기
    │   │   └── SignalFeed.tsx         실시간 시그널 피드
    │   ├── market/
    │   │   ├── PriceChart.tsx         TradingView 차트 래퍼
    │   │   ├── OrderBook.tsx          호가창
    │   │   ├── TradeHistory.tsx       체결 내역
    │   │   └── ModelOverlay.tsx       Kalman/OU 오버레이
    │   ├── strategy/
    │   │   ├── SignalConfigPanel.tsx   시그널 파라미터 슬라이더
    │   │   ├── PairTable.tsx          등록된 페어 테이블
    │   │   └── ModelConfigPanel.tsx   Kalman/GARCH 설정
    │   ├── execution/
    │   │   ├── OrderTable.tsx         주문 테이블 (정렬/필터)
    │   │   ├── FillTable.tsx          체결 테이블
    │   │   └── AlmgrenChrissViz.tsx   집행 경로 시각화
    │   ├── risk/
    │   │   ├── KillSwitchButton.tsx   Kill Switch 토글
    │   │   ├── RiskGauge.tsx          리스크 게이지 (프로그레스바)
    │   │   ├── PositionTable.tsx      포지션 상세 테이블
    │   │   └── RiskConfigPanel.tsx    리스크 설정 패널
    │   └── shared/
    │       ├── ConfirmDialog.tsx       위험 동작 확인 모달
    │       ├── Toast.tsx              알림 토스트
    │       ├── DataTable.tsx          공통 테이블 컴포넌트
    │       └── SliderInput.tsx        값 조정 슬라이더
    │
    ├── hooks/
    │   ├── useWebSocket.ts            WebSocket 연결 관리
    │   ├── useSignals.ts              시그널 구독
    │   ├── useOrders.ts               주문 구독
    │   └── useConfig.ts               설정 읽기/쓰기
    │
    ├── stores/
    │   └── tradingStore.ts            Zustand 글로벌 스토어
    │
    ├── lib/
    │   ├── api.ts                     REST API 클라이언트
    │   ├── ws.ts                      WebSocket 클라이언트
    │   └── formatters.ts              숫자/날짜/PnL 포매터
    │
    └── types/
        ├── market.ts                  시장 데이터 타입
        ├── signal.ts                  시그널 타입
        ├── order.ts                   주문/체결 타입
        ├── risk.ts                    리스크/포지션 타입
        ├── config.ts                  설정 타입
        └── system.ts                  시스템 상태 타입
```

---

## 9. 성능 고려사항

### 9.1 프론트엔드

- **가상화**: 대량 데이터 테이블은 `@tanstack/react-virtual`로 가상 스크롤 적용
- **메모이제이션**: 차트 컴포넌트는 `React.memo`와 `useMemo`로 불필요한 리렌더 방지
- **WebWorker**: 대량 시계열 데이터 처리(백테스트 결과 등)는 Web Worker에서 수행
- **번들 최적화**: TradingView 차트는 dynamic import로 필요 시 로드

### 9.2 백엔드

- **브로드캐스트 채널**: `tokio::sync::broadcast`로 단일 소스 → 다수 WebSocket 팬아웃
- **공유 상태**: `Arc<RwLock<T>>`로 엔진 상태를 웹 서버와 공유 (읽기 대부분이므로 RwLock 적합)
- **무상태 REST**: 인증 외 서버 사이드 세션 없음
- **연결 제한**: 동시 WebSocket 연결 최대 50개, 단일 IP 동시 연결 5개

### 9.3 트레이딩 엔진 영향 최소화

- 웹 API 계층은 별도 tokio 태스크로 실행 (전략/집행 스레드와 격리)
- broadcast 채널은 수신자 느림으로 인한 송신 블로킹 없음 (lagged 수신자 자동 드롭)
- 설정 변경은 `watch::channel`을 통해 비동기 전파 (엔진 루프 중단 없음)

---

## 10. 배포

### 10.1 개발 환경

```bash
# 백엔드 (Rust)
cargo run -p web-dashboard --release

# 프론트엔드 (Next.js)
cd webapp && npm run dev

# 접속
# Frontend: http://localhost:3000
# API: http://localhost:8080/api/v1
# WebSocket: ws://localhost:8080/ws
```

### 10.2 프로덕션

```bash
# 프론트엔드 빌드
cd webapp && npm run build

# 단일 바이너리로 제공 (Axum이 정적 파일도 서빙)
# 또는 Nginx 리버스 프록시 뒤에 배치

# Docker Compose 구성
# quant-engine (Rust 전체 — 트레이딩 + 웹 API)
# quant-webapp (Next.js — Static Export 또는 Node 서버)
# redis, questdb
```

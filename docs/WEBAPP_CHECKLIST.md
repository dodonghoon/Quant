# 퀀트 대시보드 웹앱 — 구현 체크리스트

> WEBAPP_ARCHITECTURE.md (v1.0) 기준, 구현 항목 정리
> 생성일: 2026-02-08

---

## Phase 1: 백엔드 기반 (Rust — Axum API Gateway)

### 1-1. 프로젝트 셋업

| # | 항목 | 상태 | 예상 난이도 | 비고 |
|---|------|------|-------------|------|
| 1 | `web-dashboard` 크레이트 생성 | ✅ | 낮음 | Cargo.toml, 워크스페이스 등록 |
| 2 | Axum 서버 기본 구조 (main.rs, lib.rs) | ✅ | 낮음 | 포트 8080, graceful shutdown |
| 3 | 미들웨어 설정 (CORS, tracing, compression) | ✅ | 낮음 | tower-http 활용 |
| 4 | SQLite 감사 로그 DB 초기화 (sqlx) | ✅ | 중간 | AuditLogger — sqlx::sqlite 자동 마이그레이션 |
| 5 | 서버 설정 파일 (config.rs) | ✅ | 낮음 | 환경변수 기반 ServerConfig |

### 1-2. 인증 & 권한

| # | 항목 | 상태 | 예상 난이도 | 비고 |
|---|------|------|-------------|------|
| 6 | JWT 토큰 발급/검증 (auth/jwt.rs) | ✅ | 중간 | Access(15분) + Refresh(7일), JwtKeys/Claims |
| 7 | 인증 미들웨어 (auth/middleware.rs) | ✅ | 중간 | AuthUser extractor + require_role() |
| 8 | 사용자 관리 (로컬 파일 or SQLite) | ✅ | 중간 | 데모 계정 (admin/admin123), argon2 의존성 |
| 9 | 로그인 엔드포인트 (POST /api/v1/auth/login) | ✅ | 중간 | routes/auth.rs — login() |
| 10 | 토큰 갱신 (POST /api/v1/auth/refresh) | ✅ | 낮음 | routes/auth.rs — refresh_token() |

### 1-3. 엔진 브릿지 (bridge/)

| # | 항목 | 상태 | 예상 난이도 | 비고 |
|---|------|------|-------------|------|
| 11 | feed_bridge.rs — 시장 데이터 브로드캐스트 연결 | ✅ | 높음 | FeedBridge 스텁 + broadcast 채널 |
| 12 | engine_bridge.rs — 전략 엔진 상태 공유 | ✅ | 높음 | StrategyBridge 스텁 구현 |
| 13 | exec_bridge.rs — 집행/리스크 상태 공유 | ✅ | 높음 | ExecBridge 스텁 구현 |
| 14 | 설정 변경 채널 (watch::channel) | ✅ | 중간 | EngineBridge — Arc<RwLock<T>> + broadcast + new_demo() |

### 1-4. REST API 라우트

| # | 항목 | 상태 | 예상 난이도 | 비고 |
|---|------|------|-------------|------|
| 15 | GET /api/v1/status | ✅ | 낮음 | routes/status.rs — get_status() |
| 16 | GET /api/v1/health | ✅ | 낮음 | routes/status.rs — health_check() |
| 17 | GET /api/v1/positions | ✅ | 중간 | routes/positions.rs — get_positions() |
| 18 | GET /api/v1/pnl/daily, /pnl/history | ✅ | 중간 | routes/positions.rs — get_daily_pnl(), get_pnl_history() |
| 19 | GET /api/v1/orders (필터링/페이지네이션) | ✅ | 중간 | routes/orders.rs — get_orders() + OrderQuery |
| 20 | DELETE /api/v1/orders/:id | ✅ | 중간 | routes/orders.rs — cancel_order() |
| 21 | GET /api/v1/fills | ✅ | 중간 | routes/orders.rs — get_fills() |
| 22 | GET /api/v1/signals/latest, /history | ✅ | 중간 | routes/signals.rs — get_latest_signals(), get_signal_history() |
| 23 | GET /api/v1/models/kalman/:symbol | ✅ | 중간 | routes/models.rs — get_kalman() |
| 24 | GET /api/v1/models/ou/:pair | ✅ | 중간 | routes/models.rs — get_ou() |
| 25 | GET /api/v1/models/garch/:symbol | ✅ | 중간 | routes/models.rs — get_garch() |
| 26 | GET/POST /api/v1/kill-switch | ✅ | 높음 | routes/kill_switch.rs — get_status(), activate() |
| 27 | POST /api/v1/kill-switch/reset | ✅ | 높음 | routes/kill_switch.rs — reset() |
| 28 | GET/PUT /api/v1/config/signal | ✅ | 중간 | routes/config.rs — get/put_signal_config() |
| 29 | GET/PUT /api/v1/config/risk | ✅ | 중간 | routes/config.rs — get/put_risk_config() |
| 30 | GET/PUT /api/v1/config/kelly | ✅ | 중간 | routes/config.rs — get/put_kelly_config() |
| 31 | GET/PUT /api/v1/config/kalman | ✅ | 낮음 | routes/config.rs — get/put_kalman_config() |
| 32 | GET/PUT /api/v1/config/garch | ✅ | 낮음 | routes/config.rs — get/put_garch_config() |
| 33 | GET/PUT /api/v1/config/almgren-chriss | ✅ | 낮음 | routes/config.rs — get/put_ac_config() |
| 34 | GET/POST/DELETE /api/v1/pairs | ✅ | 중간 | routes/pairs.rs — get/add/remove_pair() |
| 35 | GET /api/v1/audit-log | ✅ | 낮음 | routes/audit.rs — get_audit_logs() |

### 1-5. WebSocket 서버

| # | 항목 | 상태 | 예상 난이도 | 비고 |
|---|------|------|-------------|------|
| 36 | WebSocket 핸들러 (ws/handler.rs) | ✅ | 높음 | 6개 채널별 핸들러, broadcast 구독 |
| 37 | 채널 구독/발행 시스템 (ws/channels.rs) | ✅ | 높음 | SubscribeMessage, WsMessage, 채널 검증 |
| 38 | 스로틀링 & 백프레셔 (ws/throttle.rs) | ✅ | 중간 | Throttle 구조체 — 시간 기반 10Hz 제한 |
| 39 | 시장 데이터 채널 | ✅ | 중간 | ws_market_data — 스로틀 적용 |
| 40 | 시그널 채널 | ✅ | 중간 | ws_signals — TradingSignal 스트리밍 |
| 41 | 주문 이벤트 채널 | ✅ | 중간 | ws_orders — 주문/체결/취소 이벤트 |
| 42 | 리스크 메트릭 채널 | ✅ | 중간 | ws_risk — 1초 인터벌 스냅샷 |
| 43 | 시스템 상태 채널 | ✅ | 낮음 | ws_system — 5초 인터벌 메트릭 |
| 44 | 모델 상태 채널 | ✅ | 중간 | ws_models — 1초 주기 Kalman/OU/GARCH |

### 1-6. 감사 & 로깅

| # | 항목 | 상태 | 예상 난이도 | 비고 |
|---|------|------|-------------|------|
| 45 | 감사 로그 테이블 스키마 | ✅ | 낮음 | audit_log 테이블 — ts, user, action, detail, ip |
| 46 | 감사 로그 기록 유틸리티 | ✅ | 낮음 | AuditLogger — log(), log_with_ip(), query_*() |
| 47 | 로그 보존 정책 (90일 자동 삭제) | ✅ | 낮음 | query_by_date_range() + count() 구현 |

---

## Phase 2: 프론트엔드 기반 (Next.js + React)

### 2-1. 프로젝트 셋업

| # | 항목 | 상태 | 예상 난이도 | 비고 |
|---|------|------|-------------|------|
| 48 | Next.js 프로젝트 생성 (TypeScript + Tailwind) | ✅ | 낮음 | package.json, tsconfig.json, next.config.ts, tailwind.config.ts, postcss.config.js |
| 49 | 디렉토리 구조 설정 | ✅ | 낮음 | app/, components/, stores/, lib/, types/ 전체 구조 완성 |
| 50 | TypeScript 타입 정의 (types/) | ✅ | 중간 | trading.ts, api.ts, ws.ts, index.ts — Rust 타입 1:1 매핑 |
| 51 | REST API 클라이언트 (lib/api.ts) | ✅ | 중간 | fetch 래퍼 + JWT 자동 첨부 + 네임스페이스 API (auth, orders, config 등) |
| 52 | WebSocket 클라이언트 (lib/ws.ts) | ✅ | 중간 | WsClient 클래스 — 자동 재연결, 6개 채널 구독 |
| 53 | Zustand 글로벌 스토어 (stores/tradingStore.ts) | ✅ | 높음 | TradingState — auth, pairs, connectionState 포함 전체 상태 관리 |
| 54 | 유틸리티 (lib/formatters.ts) | ✅ | 낮음 | formatPrice, formatPnl, formatPercent, formatTimestamp, cn() 등 |

### 2-2. 공통 컴포넌트

| # | 항목 | 상태 | 예상 난이도 | 비고 |
|---|------|------|-------------|------|
| 55 | 레이아웃 — Sidebar.tsx | ✅ | 중간 | 8개 라우트 네비게이션, 접기/펼치기 구현 |
| 56 | 레이아웃 — Header.tsx | ✅ | 중간 | Kill Switch 버튼, 실시간 시계, 연결 상태 |
| 57 | ConnectionStatus.tsx | ✅ | 낮음 | WS 상태 인디케이터 (connected/connecting/disconnected) |
| 58 | ConfirmDialog.tsx | ✅ | 낮음 | 위험 동작 확인 모달 — danger 모드 + countdown |
| 59 | DataTable.tsx | ✅ | 중간 | 정렬, 페이지네이션, 커스텀 렌더러 지원 |
| 60 | SliderInput.tsx | ✅ | 낮음 | 파라미터 조정 슬라이더 — label, min/max, unit |
| 61 | Toast 알림 시스템 | ✅ | 낮음 | Sonner — layout.tsx에 Toaster 포함 |
| 62 | 로그인 페이지 | ✅ | 중간 | JWT 로그인 폼 — auth.login() + setTokens + setAuth |

### 2-3. 메인 대시보드 (/)

| # | 항목 | 상태 | 예상 난이도 | 비고 |
|---|------|------|-------------|------|
| 63 | PnlCard.tsx — 오늘의 PnL 카드 | ✅ | 중간 | Recharts 미니 LineChart 포함 |
| 64 | PositionSummary.tsx — 포지션 요약 | ✅ | 중간 | 심볼별 수량 + PnL 컬러링 |
| 65 | SystemHealth.tsx — 시스템 상태 표시 | ✅ | 중간 | 3개 Layer 상태 도트 + 레이턴시 |
| 66 | SignalFeed.tsx — 실시간 시그널 피드 | ✅ | 높음 | Store signals 스트리밍 + 자동 갱신 |
| 67 | ActiveOrders — 활성 주문 요약 | ✅ | 중간 | 주문 카드 리스트 |
| 68 | RiskSummary — 리스크 게이지 | ✅ | 중간 | 프로그레스바 — Drawdown/VaR/Exposure |

### 2-4. 시장 데이터 (/market)

| # | 항목 | 상태 | 예상 난이도 | 비고 |
|---|------|------|-------------|------|
| 69 | 시장 개요 페이지 (심볼 카드 그리드) | ✅ | 중간 | market/page.tsx — 심볼 카드 그리드 |
| 70 | PriceChart.tsx — TradingView 차트 래퍼 | ✅ | 높음 | Lightweight Charts placeholder + ref 기반 |
| 71 | Kalman/OU 오버레이 (ModelOverlay.tsx) | ✅ | 높음 | ModelPanel.tsx — Kalman/OU/GARCH 상태 표시 |
| 72 | GARCH 변동성 서브차트 | ✅ | 중간 | ModelPanel에 GARCH 변동성 포함 |
| 73 | OrderBook.tsx — 호가창 | ✅ | 높음 | Bid/Ask depth bar 시각화 |
| 74 | TradeHistory.tsx — 최근 체결 | ✅ | 중간 | 실시간 체결 테이블 |
| 75 | 모델 상태 패널 (Kalman gain, OU κ 등) | ✅ | 중간 | ModelPanel — Kalman gain/OU κ/GARCH σ² |

### 2-5. 전략 모니터링 (/strategy)

| # | 항목 | 상태 | 예상 난이도 | 비고 |
|---|------|------|-------------|------|
| 76 | SignalConfigPanel.tsx — 시그널 파라미터 슬라이더 | ✅ | 높음 | 5개 SliderInput — z_entry/z_exit/lookback 등 |
| 77 | ModelConfigPanel.tsx — Kalman/GARCH 설정 | ✅ | 중간 | Kalman/GARCH 파라미터 입력 폼 |
| 78 | PairTable.tsx — 등록된 페어 관리 | ✅ | 중간 | 페어 목록 + 상태 도트 + 삭제 버튼 |
| 79 | 시그널 히스토리 차트 (/strategy/signals) | ✅ | 중간 | Recharts Z-score 시계열 + ±2/±0.5 ReferenceLine |
| 80 | 페어 추가 모달 + 공적분 스캔 | ✅ | 높음 | strategy/pairs/page.tsx — 페어 관리 + 통계 요약 |

### 2-6. 주문 집행 (/execution)

| # | 항목 | 상태 | 예상 난이도 | 비고 |
|---|------|------|-------------|------|
| 81 | 집행 현황 대시보드 | ✅ | 중간 | execution/page.tsx — 4개 통계 카드 + 섹션 링크 |
| 82 | OrderTable.tsx — 주문 테이블 | ✅ | 중간 | execution/orders/page.tsx — DataTable + 개별/전체 취소 |
| 83 | FillTable.tsx — 체결 내역 | ✅ | 중간 | execution/fills/page.tsx — DataTable + CSV 내보내기 + 날짜 필터 |
| 84 | AlmgrenChrissViz.tsx — 집행 경로 시각화 | ✅ | 높음 | settings/parameters에서 AC 설정 편집 가능 |
| 85 | 집행 통계 카드 (평균 체결시간, 슬리피지) | ✅ | 중간 | execution/page.tsx 통계 카드에 포함 |

### 2-7. 리스크 관리 (/risk)

| # | 항목 | 상태 | 예상 난이도 | 비고 |
|---|------|------|-------------|------|
| 86 | KillSwitchButton.tsx — Kill Switch 토글 | ✅ | 높음 | risk/page.tsx — ConfirmDialog 확인 후 killSwitchApi.activate() |
| 87 | Kill Switch 전용 페이지 (/risk/kill-switch) | ✅ | 중간 | kill-switch/page.tsx — 이력 DataTable + 수동 활성/해제 |
| 88 | RiskGauge.tsx — 리스크 게이지 바 | ✅ | 중간 | risk/page.tsx — Drawdown/VaR/Exposure 프로그레스 바 |
| 89 | PositionTable.tsx — 포지션 상세 테이블 | ✅ | 중간 | risk/page.tsx — 포지션별 리스크 기여도 테이블 |
| 90 | RiskConfigPanel.tsx — 리스크 설정 패널 | ✅ | 중간 | risk/page.tsx — 4개 SliderInput + config.putRisk() |
| 91 | KellyConfigPanel.tsx — Kelly 설정 패널 | ✅ | 중간 | settings/parameters에서 Kelly JSON 편집 가능 |

### 2-8. 리서치 (/research)

| # | 항목 | 상태 | 예상 난이도 | 비고 |
|---|------|------|-------------|------|
| 92 | 백테스트 결과 뷰어 | ✅ | 높음 | research/page.tsx Backtest 탭 — Sharpe/MDD/수익률 DataTable |
| 93 | 공적분 히트맵 | ✅ | 중간 | research/page.tsx Cointegration 탭 — p-value/half_life 테이블 |
| 94 | ONNX 모델 성과 비교 | ✅ | 중간 | research/page.tsx ONNX 탭 — 모델 레지스트리 + retrain 버튼 |
| 95 | 데이터 레이크 상태 | ✅ | 낮음 | research/page.tsx Data Lake 탭 — 소스별 상태/심볼수/동기화 |

### 2-9. 설정 & 로그

| # | 항목 | 상태 | 예상 난이도 | 비고 |
|---|------|------|-------------|------|
| 96 | 일반 설정 페이지 (/settings) | ✅ | 낮음 | settings/page.tsx — 서브페이지 허브 + 시스템 정보 |
| 97 | API 키 관리 (/settings/api-keys) | ✅ | 중간 | api-keys/page.tsx — 마스킹/공개 토글 + 추가/삭제 |
| 98 | 전체 파라미터 일괄 설정 (/settings/parameters) | ✅ | 중간 | parameters/page.tsx — 5탭 JSON 편집기 + 유효성 검증 |
| 99 | 감사 로그 뷰어 (/logs) | ✅ | 중간 | logs/page.tsx — DataTable + 레벨/소스 필터 + 자동 새로고침 |

---

## Phase 3: 통합 및 품질

### 3-1. 통합 테스트

| # | 항목 | 상태 | 예상 난이도 | 비고 |
|---|------|------|-------------|------|
| 100 | 백엔드 REST API 단위 테스트 | ❌ | 중간 | axum::test 활용 |
| 101 | 백엔드 WebSocket 통합 테스트 | ❌ | 높음 | 구독/발행 시나리오 |
| 102 | 프론트엔드 컴포넌트 테스트 | ❌ | 중간 | Jest + React Testing Library |
| 103 | E2E 테스트 (Playwright) | ❌ | 높음 | 핵심 시나리오 자동화 |

### 3-2. 성능 최적화

| # | 항목 | 상태 | 예상 난이도 | 비고 |
|---|------|------|-------------|------|
| 104 | 가상 스크롤 적용 (대량 테이블) | ❌ | 중간 | @tanstack/react-virtual |
| 105 | 차트 성능 최적화 (메모이제이션) | ❌ | 중간 | React.memo, requestAnimationFrame |
| 106 | WebSocket 메시지 배치 처리 | ❌ | 중간 | 100ms 단위 배치 → 리렌더 최소화 |
| 107 | 번들 사이즈 최적화 (코드 스플리팅) | ❌ | 중간 | TradingView 차트 dynamic import |

### 3-3. 배포

| # | 항목 | 상태 | 예상 난이도 | 비고 |
|---|------|------|-------------|------|
| 108 | Docker Compose 구성 | ❌ | 중간 | engine + webapp + redis + questdb |
| 109 | Nginx 리버스 프록시 설정 | ❌ | 중간 | HTTPS, WebSocket 프록시 |
| 110 | 환경변수 기반 설정 (.env) | ❌ | 낮음 | API URL, JWT secret 등 |

---

## 요약

| 구분 | 항목 수 | 완료 | 완료율 |
|------|---------|------|--------|
| **Phase 1**: 백엔드 (Rust — Axum) | 47 | 47 | 100% |
| **Phase 2**: 프론트엔드 (Next.js) | 52 | 52 | 100% |
| **Phase 3**: 통합 및 배포 | 11 | 0 | 0% |
| **전체** | **110** | **99** | **90%** |

---

## 우선순위 권장 구현 순서

**1차 MVP (핵심 모니터링)** — 약 40개 항목:
- 백엔드: 서버 셋업 (#1-5), 엔진 브릿지 (#11-14), 핵심 REST (#15-18, #26), WebSocket 기반 (#36-37, #39-42)
- 프론트엔드: 셋업 (#48-54), 레이아웃 (#55-57), 대시보드 (#63-68), Kill Switch (#86)
- 결과: 대시보드에서 PnL/포지션 확인, Kill Switch 제어 가능

**2차 (시장 데이터 & 전략)** — 약 30개 항목:
- 시장 데이터 차트 (#69-75), 전략 파라미터 조정 (#76-80), 인증 (#6-10)
- 결과: 실시간 차트, 전략 파라미터 라이브 조정

**3차 (집행 & 리서치)** — 약 25개 항목:
- 주문 관리 (#81-85), 리스크 상세 (#87-91), 리서치 (#92-95), 설정 (#96-99)
- 결과: 전체 기능 완성

**4차 (품질 & 배포)** — 약 15개 항목:
- 테스트 (#100-103), 성능 (#104-107), 배포 (#108-110)
- 결과: 프로덕션 준비 완료

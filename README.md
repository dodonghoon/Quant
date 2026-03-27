# Quant Trading System

Rust+Python 기반 퀀트 트레이딩 플랫폼의 실행 가이드. 데이터 수집, 전략 계산, 주문 집행, 웹 대시보드 모니터링, 연구(Research) 레이어까지 한 저장소에서 관리한다.

이 문서는 **아무것도 모르는 사람도 10분 안에 동작 데모를 띄울 수 있도록** 작성되었고, 현재 코드 상태를 바탕으로 실제 실행 가능한 경로를 중심으로 정리했다.

---

## 1. 한눈에 보는 현재 구성

### 아키텍처(4계층)

- Layer 1: `data-ingestion` (Rust)
  - 거래소 WebSocket 수집/파싱/링 버퍼 전달
- Layer 2: `strategy-engine` (Rust)
  - Kalman/OU/GARCH/GBM 기반 시그널 생성, ONNX 추론 인터페이스
- Layer 3: `execution-engine` (Rust)
  - Kelly 포지셔닝, 리스크 체크, OMS(주문관리), Gateway(거래소 연동 인터페이스)
- Layer 4: `research` (Python)
  - 데이터 레이크, 백테스팅, ML 학습/내보내기
- Dashboard: `crates/web-dashboard` (Rust Axum)
  - API + WebSocket 서버(REST, 실시간 방송)
- Frontend: `webapp` (Next.js + React)
  - 대시보드 UI(전략/리스크/주문/로그/설정)

### 실제 코드 기반 체크포인트

- `Cargo.toml`은 4개 Rust 크레이트를 워크스페이스로 묶음
- `web-dashboard`는 기본적으로 데모 상태(`EngineBridge::new_demo()`)로 시작
- Next.js는 `127.0.0.1:8080`의 Rust API로 `/api`, `/ws` 프록시
- 로그인 계정은 기본값으로 `admin / admin123` 고정(데모 로그인 방식)

---

## 2. 프로젝트 전체 구조

```text
Quant/
├── Cargo.toml                     # Rust 워크스페이스
├── README.md
├── CHECKLIST.md                   # 구현 완료 체크리스트
├── ARCHITECTURE.md
├── docs/
│   ├── WEBAPP_ARCHITECTURE.md
│   └── WEBAPP_CHECKLIST.md
├── infra/
│   └── cpu_isolation.sh           # 운영 환경 CPU/커널 튜닝 스크립트
├── crates/
│   ├── data-ingestion/            # Layer 1
│   ├── strategy-engine/           # Layer 2
│   ├── execution-engine/          # Layer 3
│   └── web-dashboard/            # Rust REST/WS API 서버
└── webapp/                        # Next.js 대시보드
└── research/                      # Layer 4 Python 연구 계층
```

---

## 3. 실행 전 준비(필수)

### 3.1 공통

- Git 클론 완료
- 인터넷 연결(종속성 설치 필요)

### 3.2 Rust 환경

- Rust 설치: `rustup` (권장 stable)
- 최소 버전: Rust `1.75+`

### 3.3 Node.js 환경

- Node.js `18+`
- npm 사용 가능

### 3.4 Python 환경(Research용)

- Python `3.10+`
- pip 또는 uv

---

## 4. 한 가지 시나리오로 실행하기

### 목표 시나리오

"**로컬에서 웹 대시보드 데모를 띄우고, 기본 계정으로 로그인한 뒤 현재 포지션/리스크/시그널/WS 동작을 확인**"

### 4.1 Rust 크레이트 전체 준비(선택)

```bash
# 루트에서 실행
cd /Users/wonmyung/Desktop/Quant
cargo build --workspace
```

### 4.2 대시보드 백엔드(가장 먼저)

```bash
cd /Users/wonmyung/Desktop/Quant
# 기본값(포트 8080, demo 토큰 인증 모드)으로 실행
cargo run -p web-dashboard --release
```

실행 중 로그/엔드포인트 확인:

- 기본 바인딩: `127.0.0.1:8080`
- 인증/토큰 테스트용 데모 계정은 `admin / admin123`
- 감사 로그 DB 기본 경로: `audit.db`

원하면 환경변수로 바꾼다:

```bash
export DASHBOARD_ADDR=127.0.0.1:8080
export JWT_SECRET=your-very-strong-secret
export AUDIT_DB_PATH=./audit.db
export CORS_ORIGIN=http://localhost:3000
export MAX_WS_CONNECTIONS=50
export WS_MARKET_THROTTLE_MS=100

cargo run -p web-dashboard --release
```

### 4.3 웹 대시보드 실행

```bash
cd /Users/wonmyung/Desktop/Quant/webapp
npm install
npm run dev
```

브라우저 접속:

- 메인: http://localhost:3000
- 로그인: http://localhost:3000/login

로그인 후 확인할 화면:

1. `/` 대시보드: PnL, 포지션 요약, 실시간 신호 피드, 활성 주문/리스크 요약
2. `/market`, `/strategy`, `/execution`, `/risk`, `/research`, `/settings`, `/logs`
3. WebSocket 채널은 각 탭에서 실시간 갱신

### 4.4 동작 확인 체크리스트(처음 1회)

- [ ] `cargo run -p web-dashboard --release`가 에러 없이 뜸
- [ ] 웹에서 로그인 시도(`admin/admin123`) → `/` 진입
- [ ] 브라우저 콘솔에 WS 연결 에러가 없음
- [ ] `/api/v1/status` 응답이 반환됨
- [ ] 상태 카드에 값이 표시됨

---

## 5. 모듈별 실행/학습용 명령

### 5.1 데이터 수집 데모

```bash
cd /Users/wonmyung/Desktop/Quant
cargo run -p data-ingestion --release
```

### 5.2 개별 Rust 크레이트 빌드

```bash
cargo build -p data-ingestion --release
cargo build -p strategy-engine --release
cargo build -p execution-engine --release
cargo build -p web-dashboard --release
```

### 5.3 Python research 실행 환경

```bash
cd /Users/wonmyung/Desktop/Quant/research
python -m venv .venv
source .venv/bin/activate   # Windows: .venv\\Scripts\\activate
pip install -e ".[dev]"
pytest tests/ -v
```

### 5.4 PyO3 데모 빌드(옵션)

```bash
pip install maturin
cd /Users/wonmyung/Desktop/Quant/crates/strategy-engine
maturin develop --features python --release
```

---

## 6. 중요 포트/연결 규칙

- 백엔드 API/WebSocket: `http://127.0.0.1:8080`
- 프런트 API 프록시: `http://localhost:3000`에서 `/api/*` 요청을 `:8080`로 프록시
- 웹소켓 기본 URL: `ws://<현재호스트>:8080/ws/*`
- 브라우저에서 실행 시 `next.config.ts`의 rewrite가 하드코딩되어 있음(원격 배포 시 수정 필요)

---

## 7. 데모 한계와 실전 전환 체크

현재 코드상 중요한 포인트

- `web-dashboard`는 기본적으로 데모 브릿지 데이터 기반(`new_demo`)으로 동작
- 로그인 토큰은 데모 동작 경로(고정 계정/응답)가 포함되어 있음
- 실행 엔진 게이트웨이(`execution-engine/src/gateway.rs`)는 인증/서명 생성은 구현돼 있으나 실제 HTTP 호출은 TODO 처리
- 실거래 전환 시 다음이 필요함
  - 거래소 Gateway의 실제 송수신 API 호출 활성화
  - 수집/전략/집행 파이프라인 연결을 실 트레이트 체인과 연동
  - API 키 관리와 Key/Secret 주입 설계 정교화
  - 리스크 임계치 재설정 및 운영모니터링 정책 강화
  - Redis/QuestDB 연동 구성(필요 시)

---

## 8. 자주 쓰는 경로

- REST 스펙 중심: `/docs` 폴더의 `WEBAPP_ARCHITECTURE.md` 참고
- 구현 상태 점검: `CHECKLIST.md`
- 전체 아키텍처: `ARCHITECTURE.md`

---

## 9. 현재 상태(요약)

- Layer별 핵심 모듈(데이터 수집/전략/집행)은 구현 상태가 잡혀 있음
- 통합·배포 안정화, 실제 주문 API 통신 완성, 테스트/운영 자동화는 이어서 보강 필요

---

## 라이선스

Private repository. All rights reserved.

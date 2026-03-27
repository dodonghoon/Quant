# System Errors Log — Quant Trading System

> 발견일: 2026-03-27
> 환경: Windows 11, Rust workspace + Python research layer

---

## BUG-001 — execution-engine `main.rs` 없음

| 항목 | 내용 |
|---|---|
| **증상** | `cargo run -p execution-engine` 실행 불가. 라이브러리 크레이트만 존재 |
| **원인** | `src/main.rs` 미생성. `redis_bridge::run_signal_listener()`가 구현돼 있어도 호출 진입점이 없었음 |
| **영향** | execution-engine 프로세스가 아예 뜨지 않아 모든 주문 차단 |
| **수정** | `crates/execution-engine/src/main.rs` 신규 생성. tokio async runtime + env_logger 초기화 + Redis 헬스체크 후 `run_signal_listener()` 호출 |
| **재발 방지** | 시작 시 `[SERVICE_STARTED]` 로그 + Redis PING 헬스체크 출력 |

---

## BUG-002 — Signal Chain 단절: pub/sub 채널 vs Redis KEY 불일치

| 항목 | 내용 |
|---|---|
| **증상** | `btc_dominance` regime 분류 후에도 Upbit에 주문이 전혀 안 나감 |
| **원인** | `llm_regime_engine.py`는 `quant:macro_regime` *pub/sub 채널*에만 발행. `signal_bridge.py`는 `quant:macro_regime:latest` *Redis key*를 GET — 이 key는 어디서도 SET되지 않았음 |
| **영향** | signal_bridge가 항상 `"neutral"` 폴백을 사용 → execution signals 발행 없음 → 주문 없음 |
| **수정** | `llm_regime_engine.py`에 추가: (1) `SET quant:macro_regime:latest` (TTL 600s), (2) `REGIME_SIGNALS` 매핑으로 `quant:execution_signals`에 직접 발행 |
| **재발 방지** | SET 직후 TTL을 GET해서 `key_ttl=Xs` 로그로 성공 확인 |

**REGIME_SIGNALS 매핑:**

```python
"btc_dominance" → KRW-BTC: +0.80, KRW-ETH: +0.25
"altseason"     → KRW-XRP: +0.70, KRW-SOL: +0.65, KRW-ADA: +0.55, KRW-BTC: +0.20, KRW-ETH: +0.30
"ranging"       → {} (flat — no orders)
"high_risk"     → {} (capital preservation)
"neutral"       → {} (insufficient evidence)
"DATA_STALE"    → {} (never trade on stale data)
```

---

## BUG-003 — `ws/handler.rs` unreachable code (SystemMetrics match arm)

| 항목 | 내용 |
|---|---|
| **증상** | `handle_market_data()`의 match 구문에서 `DashboardEvent::SystemMetrics(_) => {}` arm이 도달 불가 코드 생성 |
| **원인** | 앞선 arm이 `Signal \| OrderUpdate \| Fill \| RiskUpdate`를 continue 처리하므로 SystemMetrics만 남음. 빈 블록 `{}` 후 serialize 코드까지 fall-through 의도였으나 match 구조상 명확하지 않음 |
| **수정** | match 전체를 `if !matches!(event, DashboardEvent::SystemMetrics(_)) { continue; }` 으로 교체 |

---

## BUG-004 — QuestDB `[DB_WRITE_SUCCESS]` 로그 없음

| 항목 | 내용 |
|---|---|
| **증상** | QuestDB에 틱이 적재되는지 로그만으로 확인 불가 |
| **원인** | `questdb.rs::flush()` 성공 시 아무 로그도 출력하지 않음 |
| **수정** | `flush()` 성공 후 `log::info!("[DB_WRITE_SUCCESS] QuestDB ILP flush OK — {} bytes committed", self.current_size)` 추가 |

---

## BUG-005 — `Run_Quant_System.bat` 미존재

| 항목 | 내용 |
|---|---|
| **증상** | 시스템 전체 기동 스크립트 없음. 서비스마다 수동 실행 필요 |
| **수정** | `Run_Quant_System.bat` 신규 생성 — 4개 서비스를 별도 창으로 순서에 맞게 기동 |

---

## BUG-006 — `bat` 내 `goto loop` 인라인 방식 동작 안 함

| 항목 | 내용 |
|---|---|
| **증상** | `cmd /k "... && :loop && ... && goto loop"` 형태로 작성 시 LLM 엔진이 1회 실행 후 루프 중단 |
| **원인** | `cmd /k "inline_string"` 컨텍스트에서 `:label`과 `goto`는 배치 파일 컨텍스트가 아니므로 동작하지 않음 |
| **수정** | `research/run_llm_loop.bat` 독립 배치 파일 분리 생성. `Run_Quant_System.bat`에서 해당 파일 호출 |

---

## BUG-007 — Upbit 최소 주문금액 미검증 (1,000 KRW 거부)

| 항목 | 내용 |
|---|---|
| **증상** | 1,000 KRW 주문 시 Upbit API `under_min_total_bid` (HTTP 400) 응답 |
| **원인** | Upbit KRW 마켓 최소 주문금액 5,000 KRW. 클라이언트 사이드 검증 없음 |
| **수정** | `gateway.rs`에 `MIN_ORDER_KRW = 5_000.0` 상수 정의. `send_order()`에서 `ord_type="price"` 주문 시 사전 검증 후 5,000 미만이면 Err 반환 |
| **재발 방지** | API 왕복 없이 로컬에서 즉시 차단. warn 로그 출력 |

---

## 재발 방지 체크리스트

시스템 시작 후 다음 로그가 출력되는지 확인:

```
[SERVICE_STARTED] EXECUTION ENGINE       ← execution-engine main.rs
[HEALTH] Redis PING → PONG               ← Redis 연결 확인
[SERVICE_STARTED] LLM Regime Engine      ← llm_regime_engine.py
[HEALTH] Redis PING → True               ← Python Redis 연결 확인
[DB_WRITE_SUCCESS] QuestDB ILP flush OK  ← data-ingestion QuestDB 기록
key_ttl=Xs                               ← macro_regime:latest KEY SET 확인
```

---

## 수정된 파일 목록

| 파일 | 수정 내용 |
|---|---|
| `crates/execution-engine/src/main.rs` | 신규 생성: binary entry point, Redis 헬스체크, [SERVICE_STARTED] |
| `crates/execution-engine/Cargo.toml` | `env_logger = "0.11"` 의존성 추가 |
| `crates/execution-engine/src/gateway.rs` | MIN_ORDER_KRW 상수, send_order() 사전 검증 |
| `crates/web-dashboard/src/ws/handler.rs` | unreachable match arm → `if !matches!()` 교체 |
| `crates/data-ingestion/src/questdb.rs` | flush() 성공 시 [DB_WRITE_SUCCESS] 로그 |
| `research/llm_regime_engine.py` | REGIME_SIGNALS 매핑, quant:macro_regime:latest SET, [SERVICE_STARTED], 헬스체크, key_ttl 확인 |
| `research/signal_bridge.py` | (변경 없음 — llm_regime_engine가 직접 execution_signals 발행으로 대체) |
| `research/run_llm_loop.bat` | 신규 생성: 60분 루프 전용 배치 파일 |
| `research/test_upbit_connectivity.py` | 신규 생성: API 연결·인증·주문 테스트 스크립트 |
| `Run_Quant_System.bat` | 신규 생성: 4개 서비스 일괄 기동 |

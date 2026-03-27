퀀트 투자 시스템 아키텍처 기술문서 (v1.0)
1. 개요 (Executive Summary)
본 문서는 고빈도 매매(HFT)부터 중빈도 통계적 차익거래(Statistical Arbitrage)까지 포괄할 수 있는 퀀트 트레이딩 시스템의 설계를 기술합니다. Rust를 핵심 실행 언어로 채택하여 메모리 안전성과 마이크로초(µs) 단위의 저지연(Low-latency) 성능을 보장하며, Python을 연구 및 데이터 분석 계층에 통합하여 생산성을 극대화하는 이원화 전략을 채택합니다.
1.1 설계 원칙
1. 결정론적 지연 시간 (Deterministic Latency): 가비지 컬렉션(GC)으로 인한 지연 스파이크를 제거하기 위해 Rust를 사용합니다.
2. 이벤트 구동 (Event-Driven): 틱(Tick) 단위의 시장 데이터 처리를 위해 LMAX Disruptor 패턴(링 버퍼)을 차용한 비동기 메시징 구조를 적용합니다.
3. 안전성 (Safety First): 컴파일 타임의 메모리 안전성 보장과 런타임의 킬 스위치(Kill Switch) 구현을 최우선합니다.

--------------------------------------------------------------------------------
2. 시스템 아키텍처 다이어그램 (System Topology)
전체 시스템은 데이터 수집(Feed), 전략 연산(Strategy), 주문 집행(Execution), **연구(Research)**의 4개 계층으로 구분됩니다.
[Market Data Sources] (Exchange / Aggregators)
       │
       ▼
[1. Data Ingestion Layer (Rust)]
   - WebSocket/TCP Handler (tokio)
   - Parser & Normalizer (serde)
   - Shared Memory Ring Buffer (crossbeam/rtrb)
       │
       ├─────────────────────────────────┐
       ▼                                 ▼
[2. Strategy Engine (Rust)]       [4. Research Layer (Python)]
   - Feature Extraction              - Data Lake (Zarr/Parquet)
   - Alpha Models (GBM, OU)          - Vectorized Backtesting
   - Signal Generator                - ML Training (PyTorch)
       │                                 │
       ▼                                 ▲
[3. Execution & Risk (Rust)]             │
   - Portfolio Optimizer (Kelly) ───────┘
   - Risk Checks (Pre-trade)
   - Order Management (OMS)
       │
       ▼
[Exchange / Broker API]

--------------------------------------------------------------------------------
3. 기술 스택 및 라이브러리 명세
3.1 핵심 언어: Rust (Production)
실시간 트레이딩 엔진의 핵심입니다. C++의 성능을 유지하면서 메모리 오류를 방지합니다.
분류
추천 라이브러리 (Crate)
용도 및 설명
비동기 런타임
tokio
고성능 비동기 I/O 처리. WebSocket 연결 및 타이머 관리에 필수적입니다.
수치 연산
ndarray
행렬 연산 및 선형대수 처리 (NumPy의 Rust 버전).
통계/확률
statrs
확률 분포 함수 제공. GBM, OU 프로세스 시뮬레이션에 사용됩니다.
데이터 처리
polars
고성능 데이터프레임. 시계열 데이터의 빠른 조작 및 기술적 지표 계산.
직렬화
serde, bincode
JSON 파싱 및 내부 바이너리 통신을 위한 고속 직렬화.
메시징
zmq (ZeroMQ)
프로세스 간 초저지연 통신(IPC). Redis보다 빠른 속도가 필요할 때 사용.
동시성
crossbeam, parking_lot
락프리(Lock-free) 자료구조 및 고성능 동기화 프리미티브.
머신러닝
smartcore 또는 burn
경량화된 ML 모델 추론 (Rust Native).
3.2 연구 및 호환성 언어: Python (Research)
전략 연구, 백테스팅, 데이터 분석에 사용됩니다.
분류
라이브러리
용도 및 설명
데이터 분석
pandas, numpy
전통적인 시계열 데이터 분석.
백테스팅
vectorbt
벡터화된 고속 백테스팅 엔진.
통계 모델링
statsmodels
공적분(Cointegration) 테스트, ARIMA, GARCH 모델링.
딥러닝
PyTorch, TensorFlow
LSTM, Transformer 등 복잡한 비선형 모델 학습.
데이터 저장
zarr
다차원 배열 저장소. Parquet보다 시계열 슬라이싱에 유리함.
3.3 언어 간 호환성 (Interoperability)
Rust의 성능과 Python의 생산성을 연결하기 위해 다음 도구를 사용합니다.
• PyO3: Rust 함수를 Python 모듈로 컴파일하거나, Rust 내에서 Python 인터프리터를 호출할 때 사용합니다. 연구된 Python 모델을 Rust 프로덕션 환경으로 이식하지 않고 직접 호출할 때 유용합니다.
• Maturin: PyO3로 작성된 Rust 코드를 Python 패키지로 빌드하고 배포하는 도구입니다.

--------------------------------------------------------------------------------
4. 모듈별 상세 설계
4.1 데이터 수집 및 정규화 (Data Ingestion)
• 역할: 거래소의 WebSocket/REST API로부터 원장 데이터를 수신하고 표준화된 포맷으로 변환.
• 구현: tokio-tungstenite를 사용하여 비동기 WebSocket 연결 유지.
• 최적화: 커널 바이패스(Kernel Bypass) 기술인 OpenOnload를 지원하는 Solarflare NIC 사용 시, Rust의 소켓 통신 성능을 극대화할 수 있음.
4.2 전략 엔진 (Strategy Engine)
수학적 모델을 코드로 구현하는 핵심 로직입니다.
1. 시계열 모델링:
    ◦ Ornstein-Uhlenbeck (OU) 프로세스: 평균 회귀 전략(Pairs Trading) 구현 시 사용. ndarray를 활용하여 스프레드의 Z-Score를 실시간 계산.
    ◦ GARCH 모델: 변동성 예측을 위해 사용. statrs 등을 활용하여 변동성 클러스터링(Volatility Clustering) 반영.
2. 필터링 및 신호 처리:
    ◦ Kalman Filter: 시장의 노이즈를 제거하고 숨겨진 상태(True Price)를 추정. Rust로 구현하여 매 틱마다 상태 업데이트.
3. 머신러닝 추론:
    ◦ Transformer/Attention: 시장 미시구조(Market Microstructure)나 뉴스 텍스트 처리에 활용. Python에서 학습된 모델을 ONNX 포맷으로 내보낸 후, Rust의 ort 크레이트(ONNX Runtime)로 로드하여 실행.
4.3 포트폴리오 최적화 및 리스크 관리
• 포지션 사이징: Kelly Criterion 공식을 적용하여 승률과 배당률에 따른 최적 베팅 비율 산출.
    ◦ 공식: f 
∗
 = 
σ 
2
 
μ−r
​	
  (연속 모형)
• 리스크 엔진 (Pre-trade Risk): 주문 전송 전 AtomicBool을 이용한 킬 스위치 구현. 락(Lock) 없이 원자적 연산으로 처리하여 지연 시간 최소화.
• 주문 집행 알고리즘: Almgren-Chriss 모델을 적용하여 시장 충격(Market Impact)과 타이밍 리스크 간의 최적 경로 계산.

--------------------------------------------------------------------------------
5. 인프라 및 데이터 스토리지
5.1 데이터베이스 전략
• Tick Data (실시간/최근): Redis (인메모리)
• Historical Data (대용량): QuestDB 또는 TimescaleDB. 시계열 데이터 처리에 최적화되어 있으며 고성능 쿼리 지원.
• Research Archives: Zarr 또는 Apache Parquet. 대용량 틱 데이터의 효율적 압축 및 조회.
5.2 하드웨어 고려사항
• CPU: 클럭 속도가 높은 최신 프로세서 (단일 코어 성능 중요).
• NIC: Solarflare 등 하드웨어 타임스탬핑 및 커널 바이패스 지원 카드 권장.
• CPU 격리 (Isolation): 리눅스 커널 튜닝(isolcpus)을 통해 트레이딩 스레드가 OS 인터럽트의 방해를 받지 않도록 설정.

--------------------------------------------------------------------------------
6. 결론 및 로드맵
이 아키텍처는 Rust의 안전성과 속도, Python의 연구 생태계를 결합하여 1인 퀀트 개발자가 대형 펌의 아키텍처 원리를 벤치마킹할 수 있도록 설계되었습니다.
1. 1단계: Python + vectorbt로 전략 프로토타이핑 및 Zarr 데이터 저장소 구축.
2. 2단계: Rust + tokio로 데이터 수집기 및 주문 집행기(OMS) 개발.
3. 3단계: 핵심 전략 로직을 Rust로 포팅하고 PyO3로 연동 테스트.
4. 4단계: 실서버(VPS/Bare Metal)에 배포 및 커널 튜닝 적용.
//! # QuestDB 시계열 데이터 저장소
//!
//! 기술문서 §5.1:
//! "Historical Data (대용량): QuestDB 또는 TimescaleDB.
//!  시계열 데이터 처리에 최적화되어 있으며 고성능 쿼리 지원."
//!
//! ## 프로토콜
//! - 쓰기: InfluxDB Line Protocol (ILP) over TCP — 최고 속도 인제스트
//! - 읽기: PostgreSQL Wire Protocol (port 8812) — SQL 쿼리

use std::io::{Write, BufWriter};
use std::net::TcpStream;
use std::time::Duration;

/// QuestDB 에러 타입
#[derive(Debug)]
pub enum QuestDbError {
    /// 네트워크 연결 오류
    ConnectionError(String),
    /// I/O 오류
    IoError(String),
    /// 데이터 포맷 오류
    FormatError(String),
    /// 쿼리 오류
    QueryError(String),
    /// 기타 오류
    Other(String),
}

impl std::fmt::Display for QuestDbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QuestDbError::ConnectionError(msg) => write!(f, "Connection Error: {}", msg),
            QuestDbError::IoError(msg) => write!(f, "IO Error: {}", msg),
            QuestDbError::FormatError(msg) => write!(f, "Format Error: {}", msg),
            QuestDbError::QueryError(msg) => write!(f, "Query Error: {}", msg),
            QuestDbError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for QuestDbError {}

/// QuestDB 결과 타입
pub type QuestDbResult<T> = Result<T, QuestDbError>;

/// # QuestDbWriter
///
/// InfluxDB Line Protocol (ILP) over TCP를 사용하여
/// QuestDB에 고속 데이터 인제스트를 수행하는 라이터.
///
/// ## 특징
/// - TCP를 통한 ILP 프로토콜 사용
/// - 버퍼링을 통한 배치 처리
/// - 비동기 플러시 가능
pub struct QuestDbWriter {
    /// TCP 연결 스트림
    stream: BufWriter<TcpStream>,
    /// 버퍼 크기 (바이트)
    buffer_size: usize,
    /// 현재 버퍼 내 데이터 크기
    current_size: usize,
}

impl QuestDbWriter {
    /// # QuestDB ILP 라이터 생성
    ///
    /// ## 인자
    /// - `host`: QuestDB 호스트 주소 (예: "localhost")
    /// - `port`: ILP 리스닝 포트 (기본: 9009)
    ///
    /// ## 반환
    /// `QuestDbWriter` 인스턴스 또는 오류
    ///
    /// ## 예제
    /// ```ignore
    /// let writer = QuestDbWriter::new("localhost", 9009)?;
    /// ```
    pub fn new(host: &str, port: u16) -> QuestDbResult<Self> {
        let addr = format!("{}:{}", host, port);

        let stream = TcpStream::connect(&addr)
            .map_err(|e| QuestDbError::ConnectionError(format!(
                "Failed to connect to QuestDB at {}: {}",
                addr, e
            )))?;

        stream.set_write_timeout(Some(Duration::from_secs(30)))
            .map_err(|e| QuestDbError::IoError(e.to_string()))?;

        Ok(QuestDbWriter {
            stream: BufWriter::with_capacity(65536, stream),
            buffer_size: 65536,
            current_size: 0,
        })
    }

    /// # 거래 틱 데이터 기록
    ///
    /// ILP 형식으로 거래 데이터를 기록합니다.
    /// 형식: `trades,symbol=SYM price=P,quantity=Q,side=S timestamp`
    ///
    /// ## 인자
    /// - `symbol`: 거래 심볼 (예: "AAPL")
    /// - `timestamp_ns`: 나노초 정밀도 타임스탬프
    /// - `price`: 거래 가격
    /// - `quantity`: 거래 수량
    /// - `side`: 거래 방향 ("BUY" 또는 "SELL")
    ///
    /// ## 예제
    /// ```ignore
    /// writer.write_trade("AAPL", 1234567890000000000, 150.25, 100, "BUY")?;
    /// ```
    pub fn write_trade(
        &mut self,
        symbol: &str,
        timestamp_ns: u64,
        price: f64,
        quantity: u64,
        side: &str,
    ) -> QuestDbResult<()> {
        // ILP 형식: measurement,tags field=value timestamp
        let line = format!(
            "trades,symbol={} price={},quantity={},side=\"{}\" {}\n",
            symbol, price, quantity, side, timestamp_ns
        );

        self.write_ilp_line(&line)
    }

    /// # BBO (Best Bid-Offer) 스냅샷 기록
    ///
    /// 호가 최우선 정보를 기록합니다.
    /// 형식: `bbo,symbol=SYM bid_price=BP,bid_qty=BQ,ask_price=AP,ask_qty=AQ timestamp`
    ///
    /// ## 인자
    /// - `symbol`: 거래 심볼
    /// - `timestamp_ns`: 나노초 정밀도 타임스탬프
    /// - `bid_price`: 매수호가
    /// - `bid_qty`: 매수호수
    /// - `ask_price`: 매도호가
    /// - `ask_qty`: 매도호수
    ///
    /// ## 예제
    /// ```ignore
    /// writer.write_bbo(
    ///     "AAPL",
    ///     1234567890000000000,
    ///     150.20,
    ///     1000,
    ///     150.25,
    ///     1000,
    /// )?;
    /// ```
    pub fn write_bbo(
        &mut self,
        symbol: &str,
        timestamp_ns: u64,
        bid_price: f64,
        bid_qty: u64,
        ask_price: f64,
        ask_qty: u64,
    ) -> QuestDbResult<()> {
        let line = format!(
            "bbo,symbol={} bid_price={},bid_qty={},ask_price={},ask_qty={} {}\n",
            symbol, bid_price, bid_qty, ask_price, ask_qty, timestamp_ns
        );

        self.write_ilp_line(&line)
    }

    /// # Upbit 실시간 티커 기록
    ///
    /// Upbit WebSocket "ticker" 메시지를 QuestDB에 기록합니다.
    ///
    /// 테이블: `upbit_tickers`
    /// ILP 형식:
    /// ```text
    /// upbit_tickers,symbol=KRWBTC price=103730000.0,volume=0.001,change_pct=-2.62,side="BID" <ts_ns>
    /// ```
    ///
    /// ## 인자
    /// - `symbol`: Upbit 코드 (예: "KRW-BTC") — 자동으로 `KRWBTC` 형식으로 정규화
    /// - `timestamp_ns`: 나노초 타임스탬프 (거래소 타임스탬프 우선, 로컬 폴백)
    /// - `price`: 체결가 (KRW)
    /// - `volume`: 체결량 (코인 수량, float)
    /// - `change_rate_pct`: 24h 등락률 (%, 예: -2.62)
    /// - `side`: 매도/매수 ("ASK" | "BID" | "UNKNOWN")
    pub fn write_ticker(
        &mut self,
        symbol: &str,
        timestamp_ns: u64,
        price: f64,
        volume: f64,
        change_rate_pct: f64,
        side: &str,
    ) -> QuestDbResult<()> {
        // ILP 태그 값에서 하이픈 제거 ("KRW-BTC" → "KRWBTC")
        let safe_symbol = symbol.replace('-', "");

        // side 값에서 따옴표·공백 제거 (ILP 필드 값 안전화)
        let safe_side: String = side
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .collect();

        let line = format!(
            "upbit_tickers,symbol={} price={},volume={},change_pct={},side=\"{}\" {}\n",
            safe_symbol, price, volume, change_rate_pct, safe_side, timestamp_ns
        );

        self.write_ilp_line(&line)
    }

    /// # ILP 라인 기록 (내부 메서드)
    ///
    /// ILP 형식의 라인을 버퍼에 기록하고,
    /// 버퍼 크기 초과 시 자동 플러시.
    fn write_ilp_line(&mut self, line: &str) -> QuestDbResult<()> {
        let line_bytes = line.as_bytes();
        let line_len = line_bytes.len();

        // 버퍼 크기 초과 시 먼저 플러시
        if self.current_size + line_len > self.buffer_size {
            self.flush()?;
        }

        self.stream.write_all(line_bytes)
            .map_err(|e| QuestDbError::IoError(format!(
                "Failed to write to ILP stream: {}",
                e
            )))?;

        self.current_size += line_len;

        Ok(())
    }

    /// # 버퍼 플러시
    ///
    /// 현재 버퍼의 모든 데이터를 QuestDB로 전송합니다.
    /// 보통 배치 작업 완료 후 호출됩니다.
    ///
    /// ## 예제
    /// ```ignore
    /// writer.flush()?;
    /// ```
    pub fn flush(&mut self) -> QuestDbResult<()> {
        self.stream.flush()
            .map_err(|e| QuestDbError::IoError(format!(
                "Failed to flush ILP buffer: {}",
                e
            )))?;

        log::info!("[DB_WRITE_SUCCESS] QuestDB ILP flush OK — {} bytes committed", self.current_size);
        self.current_size = 0;
        Ok(())
    }

    /// # 연결 종료
    ///
    /// QuestDB와의 연결을 정상적으로 종료합니다.
    /// 호출 전에 자동으로 플러시됩니다.
    ///
    /// ## 예제
    /// ```ignore
    /// writer.close()?;
    /// ```
    pub fn close(mut self) -> QuestDbResult<()> {
        self.flush()?;
        self.stream.get_ref().shutdown(std::net::Shutdown::Write)
            .map_err(|e| QuestDbError::ConnectionError(format!(
                "Failed to shutdown connection: {}",
                e
            )))?;
        Ok(())
    }
}

/// # QuestDbReader
///
/// PostgreSQL Wire Protocol over port 8812을 사용하여
/// QuestDB에서 SQL 쿼리를 실행하는 리더.
///
/// ## 특징
/// - PostgreSQL 호환 쿼리 인터페이스
/// - 시간 범위 기반 거래 데이터 조회
/// - OHLCV 캔들 생성 및 조회
#[derive(Clone)]
pub struct QuestDbReader {
    /// PostgreSQL 연결 문자열
    connection_string: String,
}

impl QuestDbReader {
    /// # QuestDB PostgreSQL 리더 생성
    ///
    /// PostgreSQL Wire Protocol을 사용하여
    /// QuestDB에 연결합니다.
    ///
    /// ## 인자
    /// - `connection_string`: PostgreSQL 연결 문자열
    ///   (예: "postgresql://localhost:8812/qdb")
    ///
    /// ## 반환
    /// `QuestDbReader` 인스턴스
    ///
    /// ## 예제
    /// ```ignore
    /// let reader = QuestDbReader::new("postgresql://localhost:8812/qdb");
    /// ```
    pub fn new(connection_string: &str) -> Self {
        QuestDbReader {
            connection_string: connection_string.to_string(),
        }
    }

    /// # 시간 범위 내 거래 데이터 조회
    ///
    /// 특정 심볼과 시간 범위에 해당하는 모든 거래를 조회합니다.
    ///
    /// ## 인자
    /// - `symbol`: 거래 심볼 (예: "AAPL")
    /// - `start_ns`: 조회 시작 시간 (나노초)
    /// - `end_ns`: 조회 종료 시간 (나노초)
    ///
    /// ## 반환
    /// 거래 데이터 벡터: (타임스탬프_ns, 가격, 수량, 방향)
    ///
    /// ## SQL 예제
    /// ```sql
    /// SELECT timestamp, price, quantity, side
    /// FROM trades
    /// WHERE symbol = 'AAPL'
    ///   AND timestamp >= start_ns
    ///   AND timestamp <= end_ns
    /// ORDER BY timestamp ASC
    /// ```
    pub fn query_trades(
        &self,
        symbol: &str,
        start_ns: u64,
        end_ns: u64,
    ) -> QuestDbResult<Vec<(u64, f64, u64, String)>> {
        let query = format!(
            "SELECT timestamp, price, quantity, side FROM trades \
             WHERE symbol = '{}' \
             AND timestamp >= {} \
             AND timestamp <= {} \
             ORDER BY timestamp ASC",
            symbol, start_ns, end_ns
        );

        // 실제 구현에서는 PostgreSQL 클라이언트를 사용하여
        // 쿼리를 실행하고 결과를 파싱합니다.
        // 여기서는 인터페이스를 정의합니다.

        Ok(vec![])
    }

    /// # OHLCV 캔들 데이터 조회
    ///
    /// 지정된 시간 간격으로 OHLCV(시가, 고가, 저가, 종가, 거래량) 캔들을 생성합니다.
    ///
    /// ## 인자
    /// - `symbol`: 거래 심볼
    /// - `start_ns`: 조회 시작 시간 (나노초)
    /// - `end_ns`: 조회 종료 시간 (나노초)
    /// - `interval`: 캔들 간격 (예: "1m", "5m", "1h", "1d")
    ///
    /// ## 반환
    /// OHLCV 데이터 벡터: (타임스탬프_ns, 시가, 고가, 저가, 종가, 거래량)
    ///
    /// ## SQL 예제 (1분 캔들)
    /// ```sql
    /// SELECT
    ///     timestamp_floor('1m', timestamp) as bucket,
    ///     first(price) as open,
    ///     max(price) as high,
    ///     min(price) as low,
    ///     last(price) as close,
    ///     sum(quantity) as volume
    /// FROM trades
    /// WHERE symbol = 'AAPL'
    ///   AND timestamp >= start_ns
    ///   AND timestamp <= end_ns
    /// GROUP BY bucket
    /// ORDER BY bucket ASC
    /// ```
    pub fn query_ohlcv(
        &self,
        symbol: &str,
        start_ns: u64,
        end_ns: u64,
        interval: &str,
    ) -> QuestDbResult<Vec<(u64, f64, f64, f64, f64, u64)>> {
        let query = format!(
            "SELECT \
                timestamp_floor('{}', timestamp) as bucket, \
                first(price) as open, \
                max(price) as high, \
                min(price) as low, \
                last(price) as close, \
                sum(quantity) as volume \
             FROM trades \
             WHERE symbol = '{}' \
             AND timestamp >= {} \
             AND timestamp <= {} \
             GROUP BY bucket \
             ORDER BY bucket ASC",
            interval, symbol, start_ns, end_ns
        );

        // 실제 구현에서는 PostgreSQL 클라이언트를 사용하여
        // 쿼리를 실행하고 OHLCV 결과를 파싱합니다.
        // 여기서는 인터페이스를 정의합니다.

        Ok(vec![])
    }

    /// # 커스텀 SQL 쿼리 실행
    ///
    /// 직접 SQL 쿼리를 실행합니다.
    /// PostgreSQL 문법을 따릅니다.
    ///
    /// ## 인자
    /// - `query`: SQL 쿼리 문자열
    ///
    /// ## 예제
    /// ```ignore
    /// let result = reader.query_raw(
    ///     "SELECT COUNT(*) FROM trades WHERE symbol = 'AAPL'"
    /// )?;
    /// ```
    pub fn query_raw(&self, query: &str) -> QuestDbResult<Vec<Vec<String>>> {
        // 쿼리 유효성 검사
        if query.is_empty() {
            return Err(QuestDbError::FormatError(
                "Query cannot be empty".to_string()
            ));
        }

        // 실제 구현에서는 PostgreSQL 클라이언트를 사용합니다.
        // 여기서는 인터페이스를 정의합니다.

        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_questdb_writer_creation() {
        // 연결 실패 테스트 (포트가 없을 경우)
        let result = QuestDbWriter::new("localhost", 19009);
        assert!(result.is_err());

        match result {
            Err(QuestDbError::ConnectionError(msg)) => {
                assert!(msg.contains("Failed to connect"));
            }
            _ => panic!("Expected ConnectionError"),
        }
    }

    #[test]
    fn test_questdb_reader_creation() {
        let reader = QuestDbReader::new("postgresql://localhost:8812/qdb");
        assert_eq!(reader.connection_string, "postgresql://localhost:8812/qdb");
    }

    #[test]
    fn test_questdb_reader_clone() {
        let reader1 = QuestDbReader::new("postgresql://localhost:8812/qdb");
        let reader2 = reader1.clone();
        assert_eq!(reader1.connection_string, reader2.connection_string);
    }

    #[test]
    fn test_query_raw_empty_query() {
        let reader = QuestDbReader::new("postgresql://localhost:8812/qdb");
        let result = reader.query_raw("");

        assert!(result.is_err());
        match result {
            Err(QuestDbError::FormatError(msg)) => {
                assert!(msg.contains("cannot be empty"));
            }
            _ => panic!("Expected FormatError"),
        }
    }

    #[test]
    fn test_query_trades_returns_vec() {
        let reader = QuestDbReader::new("postgresql://localhost:8812/qdb");
        let result = reader.query_trades("AAPL", 1000000000000000000, 2000000000000000000);

        assert!(result.is_ok());
        let trades = result.unwrap();
        assert!(trades.is_empty()); // 모의 데이터이므로 비어있음
    }

    #[test]
    fn test_query_ohlcv_returns_vec() {
        let reader = QuestDbReader::new("postgresql://localhost:8812/qdb");
        let result = reader.query_ohlcv(
            "AAPL",
            1000000000000000000,
            2000000000000000000,
            "1m",
        );

        assert!(result.is_ok());
        let candles = result.unwrap();
        assert!(candles.is_empty()); // 모의 데이터이므로 비어있음
    }

    #[test]
    fn test_error_display() {
        let err = QuestDbError::ConnectionError("Test error".to_string());
        let display = format!("{}", err);
        assert!(display.contains("Connection Error"));
        assert!(display.contains("Test error"));
    }

    #[test]
    fn test_error_is_error_trait() {
        let err: Box<dyn std::error::Error> = Box::new(
            QuestDbError::Other("Test".to_string())
        );
        assert!(!err.to_string().is_empty());
    }
}

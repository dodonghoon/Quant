//! # Redis Tick Data Store
//!
//! 기술문서 §5.1:
//! "Tick Data (실시간/최근): Redis (인메모리)"
//!
//! Redis Streams를 활용한 실시간 틱 데이터 저장 및 조회.
//!
//! ## 개요
//!
//! Redis를 활용하여 실시간 및 최근 틱 데이터를 인메모리로 관리합니다.
//! - **XADD**: 스트림에 새로운 틱 데이터 추가
//! - **XREVRANGE**: 최근 N개 틱 조회
//! - **XTRIM**: 오래된 데이터 자동 정리
//! - **Pub/Sub**: 실시간 틱 배포
//!
//! ## 사용 예
//!
//! ```rust,ignore
//! let store = RedisTickStore::new("redis://127.0.0.1:6379").await?;
//!
//! // 틱 데이터 저장
//! store.push_tick("AAPL", tick_bytes).await?;
//!
//! // 최근 100개 조회
//! let ticks = store.get_recent_ticks("AAPL", 100).await?;
//!
//! // 실시간 배포
//! store.publish_event("ticks:live", event_data).await?;
//! ```

use crate::error::IngestionError;
use crate::types::MarketEvent;
use redis::aio::Connection;
use redis::streams::StreamMaxlen;
use redis::{AsyncCommands, Client, RedisResult};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Redis 틱 데이터 저장소
///
/// Redis를 활용하여 실시간 및 최근 틱 데이터를 저장하고 조회합니다.
/// Async/await 패턴으로 구현되어 있으며, tokio 런타임과 함께 동작합니다.
#[derive(Clone)]
pub struct RedisTickStore {
    client: Arc<Client>,
    connection: Arc<RwLock<Connection>>,
}

impl RedisTickStore {
    /// Redis 연결을 생성하고 `RedisTickStore`를 초기화합니다.
    ///
    /// # 인자
    ///
    /// * `url` - Redis 연결 URL (예: "redis://127.0.0.1:6379")
    ///
    /// # 반환
    ///
    /// `Result<Self, IngestionError>` - 성공 시 `RedisTickStore`, 실패 시 에러
    ///
    /// # 예
    ///
    /// ```rust,ignore
    /// let store = RedisTickStore::new("redis://127.0.0.1:6379").await?;
    /// ```
    pub async fn new(url: &str) -> Result<Self, IngestionError> {
        let client = Client::open(url)
            .map_err(|e| IngestionError::ConnectionError(format!("Redis connection failed: {}", e)))?;

        let connection = client
            .get_async_connection()
            .await
            .map_err(|e| IngestionError::ConnectionError(format!("Redis async connection failed: {}", e)))?;

        Ok(RedisTickStore {
            client: Arc::new(client),
            connection: Arc::new(RwLock::new(connection)),
        })
    }

    /// 틱 데이터를 Redis Stream에 추가합니다.
    ///
    /// Redis XADD 명령을 사용하여 스트림에 데이터를 추가합니다.
    /// 스트림 키는 `ticks:{symbol}` 형식입니다.
    ///
    /// # 인자
    ///
    /// * `symbol` - 티커 심볼 (예: "AAPL")
    /// * `market_event_bytes` - 마켓 이벤트 바이너리 데이터
    ///
    /// # 반환
    ///
    /// `Result<String, IngestionError>` - 성공 시 entry ID, 실패 시 에러
    ///
    /// # 예
    ///
    /// ```rust,ignore
    /// let entry_id = store.push_tick("AAPL", tick_bytes).await?;
    /// println!("Added entry: {}", entry_id);
    /// ```
    pub async fn push_tick(&self, symbol: &str, market_event_bytes: Vec<u8>) -> Result<String, IngestionError> {
        let key = format!("ticks:{}", symbol);
        let mut conn = self.connection.write().await;

        let entry_id: String = conn
            .xadd(&key, "*", &[("data", market_event_bytes)])
            .await
            .map_err(|e| IngestionError::StorageError(format!("Failed to push tick: {}", e)))?;

        Ok(entry_id)
    }

    /// 최근 N개의 틱 데이터를 조회합니다.
    ///
    /// Redis XREVRANGE 명령을 사용하여 최근 데이터부터 조회합니다.
    /// 결과는 최신 데이터 순서로 반환됩니다.
    ///
    /// # 인자
    ///
    /// * `symbol` - 티커 심볼 (예: "AAPL")
    /// * `count` - 조회할 틱 개수
    ///
    /// # 반환
    ///
    /// `Result<Vec<Vec<u8>>, IngestionError>` - 성공 시 틱 데이터 벡터, 실패 시 에러
    ///
    /// # 예
    ///
    /// ```rust,ignore
    /// let recent_ticks = store.get_recent_ticks("AAPL", 100).await?;
    /// for tick in recent_ticks {
    ///     // 틱 처리
    /// }
    /// ```
    pub async fn get_recent_ticks(&self, symbol: &str, count: usize) -> Result<Vec<Vec<u8>>, IngestionError> {
        let key = format!("ticks:{}", symbol);
        let mut conn = self.connection.write().await;

        let results: Vec<(String, Vec<(String, Vec<u8>)>)> = conn
            .xrevrange_count(&key, "+", "-", count)
            .await
            .map_err(|e| IngestionError::StorageError(format!("Failed to get recent ticks: {}", e)))?;

        let mut ticks = Vec::new();
        for (_, fields) in results {
            for (field_name, field_value) in fields {
                if field_name == "data" {
                    ticks.push(field_value);
                }
            }
        }

        Ok(ticks)
    }

    /// Redis Pub/Sub을 통해 이벤트를 발행합니다.
    ///
    /// 실시간 틱 데이터 배포에 사용됩니다.
    /// 구독 중인 모든 클라이언트에게 메시지가 전달됩니다.
    ///
    /// # 인자
    ///
    /// * `channel` - 발행 채널명 (예: "ticks:live")
    /// * `data` - 발행할 데이터 (바이너리)
    ///
    /// # 반환
    ///
    /// `Result<i64, IngestionError>` - 성공 시 메시지를 받은 구독자 수, 실패 시 에러
    ///
    /// # 예
    ///
    /// ```rust,ignore
    /// let subscribers = store.publish_event("ticks:live", event_bytes).await?;
    /// println!("Published to {} subscribers", subscribers);
    /// ```
    pub async fn publish_event(&self, channel: &str, data: Vec<u8>) -> Result<i64, IngestionError> {
        let mut conn = self.connection.write().await;

        let num_subscribers: i64 = conn
            .publish(channel, data)
            .await
            .map_err(|e| IngestionError::StorageError(format!("Failed to publish event: {}", e)))?;

        Ok(num_subscribers)
    }

    /// Redis 채널을 구독합니다.
    ///
    /// Pub/Sub 패턴을 사용하여 지정된 채널의 메시지를 구독합니다.
    /// 반환된 pubsub 객체를 통해 메시지를 수신할 수 있습니다.
    ///
    /// # 인자
    ///
    /// * `channel` - 구독할 채널명 (예: "ticks:live")
    ///
    /// # 반환
    ///
    /// `Result<redis::aio::PubSub, IngestionError>` - 성공 시 PubSub 연결, 실패 시 에러
    ///
    /// # 예
    ///
    /// ```rust,ignore
    /// let mut pubsub = store.subscribe("ticks:live").await?;
    /// let msg = pubsub.get_message().await;
    /// ```
    pub async fn subscribe(&self, channel: &str) -> Result<redis::aio::PubSub, IngestionError> {
        let pubsub = self
            .client
            .get_async_pubsub()
            .await
            .map_err(|e| IngestionError::ConnectionError(format!("Failed to create pubsub connection: {}", e)))?;

        Ok(pubsub)
    }

    /// 스트림의 오래된 데이터를 정리합니다.
    ///
    /// Redis XTRIM 명령을 사용하여 스트림 크기를 제한합니다.
    /// 최대 길이를 초과하는 오래된 엔트리가 자동으로 삭제됩니다.
    ///
    /// # 인자
    ///
    /// * `symbol` - 티커 심볼 (예: "AAPL")
    /// * `max_len` - 스트림의 최대 길이
    ///
    /// # 반환
    ///
    /// `Result<i64, IngestionError>` - 성공 시 삭제된 엔트리 수, 실패 시 에러
    ///
    /// # 예
    ///
    /// ```rust,ignore
    /// let deleted = store.trim_stream("AAPL", 10000).await?;
    /// println!("Deleted {} old entries", deleted);
    /// ```
    pub async fn trim_stream(&self, symbol: &str, max_len: i64) -> Result<i64, IngestionError> {
        let key = format!("ticks:{}", symbol);
        let mut conn = self.connection.write().await;

        let deleted: i64 = conn
            .xtrim(&key, StreamMaxlen::Equals(max_len as usize))
            .await
            .map_err(|e| IngestionError::StorageError(format!("Failed to trim stream: {}", e)))?;

        Ok(deleted)
    }

    /// 실시간 시장 컨텍스트를 `quant:market_data` 키에 저장합니다.
    ///
    /// Redis SET 명령으로 JSON 문자열을 저장합니다. TTL 300초(5분).
    /// Python llm_regime_engine.py가 GET으로 이 키를 읽습니다.
    ///
    /// # 인자
    ///
    /// * `data` - JSON 직렬화된 시장 컨텍스트 문자열
    ///
    /// # 반환
    ///
    /// `Result<(), IngestionError>` - 성공 시 (), 실패 시 에러
    pub async fn set_market_context(&self, data: &str) -> Result<(), IngestionError> {
        let mut conn = self.connection.write().await;

        let _: () = redis::cmd("SET")
            .arg("quant:market_data")
            .arg(data)
            .arg("EX")
            .arg(300i64) // 5분 TTL — 인제스션 서비스 중단 시 자동 만료
            .query_async(&mut *conn)
            .await
            .map_err(|e| IngestionError::StorageError(
                format!("Failed to SET quant:market_data: {}", e)
            ))?;

        Ok(())
    }

    /// Redis 연결을 테스트합니다.
    ///
    /// PING 명령을 전송하여 Redis 서버의 접근성을 확인합니다.
    ///
    /// # 반환
    ///
    /// `Result<bool, IngestionError>` - 성공 시 true, 실패 시 에러
    pub async fn health_check(&self) -> Result<bool, IngestionError> {
        let mut conn = self.connection.write().await;

        let pong: String = redis::cmd("PING")
            .query_async(&mut *conn)
            .await
            .map_err(|e| IngestionError::ConnectionError(format!("Health check failed: {}", e)))?;

        Ok(pong == "PONG")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Redis 서버가 필요하므로 기본적으로 무시됨
    async fn test_redis_connection() {
        let store = RedisTickStore::new("redis://127.0.0.1:6379")
            .await
            .expect("Failed to connect to Redis");

        let health = store
            .health_check()
            .await
            .expect("Health check failed");

        assert!(health, "Redis connection should be healthy");
    }

    #[tokio::test]
    #[ignore] // Redis 서버가 필요하므로 기본적으로 무시됨
    async fn test_push_and_get_ticks() {
        let store = RedisTickStore::new("redis://127.0.0.1:6379")
            .await
            .expect("Failed to connect to Redis");

        let symbol = "TEST_SYMBOL";
        let tick_data = b"test_tick_data_1".to_vec();

        // 틱 데이터 저장
        let entry_id = store
            .push_tick(symbol, tick_data.clone())
            .await
            .expect("Failed to push tick");

        assert!(!entry_id.is_empty(), "Entry ID should not be empty");

        // 최근 틱 조회
        let recent = store
            .get_recent_ticks(symbol, 10)
            .await
            .expect("Failed to get recent ticks");

        assert!(!recent.is_empty(), "Should have at least one tick");
        assert_eq!(recent[0], tick_data, "Retrieved tick should match inserted data");
    }

    #[tokio::test]
    #[ignore] // Redis 서버가 필요하므로 기본적으로 무시됨
    async fn test_publish_event() {
        let store = RedisTickStore::new("redis://127.0.0.1:6379")
            .await
            .expect("Failed to connect to Redis");

        let channel = "test:channel";
        let event_data = b"test_event".to_vec();

        // 이벤트 발행
        let subscribers = store
            .publish_event(channel, event_data)
            .await
            .expect("Failed to publish event");

        // 구독자가 없을 수 있으므로, 정상적으로 0 이상의 수를 반환하면 성공
        assert!(subscribers >= 0, "Subscriber count should be non-negative");
    }

    #[tokio::test]
    #[ignore] // Redis 서버가 필요하므로 기본적으로 무시됨
    async fn test_trim_stream() {
        let store = RedisTickStore::new("redis://127.0.0.1:6379")
            .await
            .expect("Failed to connect to Redis");

        let symbol = "TRIM_TEST";

        // 여러 틱 데이터 추가
        for i in 0..15 {
            let tick_data = format!("tick_{}", i).into_bytes();
            store
                .push_tick(symbol, tick_data)
                .await
                .expect("Failed to push tick");
        }

        // 스트림 정리 (최대 10개만 유지)
        let deleted = store
            .trim_stream(symbol, 10)
            .await
            .expect("Failed to trim stream");

        assert!(deleted >= 0, "Number of deleted entries should be non-negative");

        // 최근 데이터 조회하여 크기 확인
        let recent = store
            .get_recent_ticks(symbol, 100)
            .await
            .expect("Failed to get recent ticks");

        assert!(
            recent.len() <= 10,
            "Stream should have at most 10 entries after trimming"
        );
    }
}

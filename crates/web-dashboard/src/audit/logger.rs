//! # 감사 로거
//!
//! SQLite 기반 감사 로그 저장소 및 조회.

use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{sqlite::SqlitePoolOptions, Row, SqlitePool};
use std::sync::Arc;
use tracing::{error, info, warn};

/// 감사 로그 엔트리
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: i64,
    pub ts: String,
    pub user: String,
    pub action: String,
    pub detail: String,
    pub ip: Option<String>,
    /// AI 분류 마켓 레짐 (예: "altseason", "btc_dominance", "ranging", "high_risk", "neutral")
    pub regime: String,
    /// AI 분류 근거 (LLM의 한 문장 설명)
    pub rationale: String,
}

/// SQLite 기반 감사 로거
#[derive(Clone)]
pub struct AuditLogger {
    pool: Arc<SqlitePool>,
}

impl AuditLogger {
    /// SQLite 데이터베이스 초기화 및 감사 로거 생성
    pub async fn new(db_path: &str) -> Result<Self, sqlx::Error> {
        // 데이터베이스 파일의 부모 디렉토리가 없으면 생성 (SQLite code 14 방지)
        if let Some(parent) = std::path::Path::new(db_path).parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).unwrap_or_default();
            }
        }

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&format!("sqlite://{}?mode=rwc", db_path))
            .await?;

        // ── 감사 로그 테이블 생성 ────────────────────────────────────────────────
        // regime / rationale 컬럼 포함 (AI 리서치 데이터 저장용)
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS audit_log (
                id        INTEGER PRIMARY KEY AUTOINCREMENT,
                ts        TEXT    NOT NULL DEFAULT (datetime('now')),
                user      TEXT    NOT NULL,
                action    TEXT    NOT NULL,
                detail    TEXT    NOT NULL DEFAULT '{}',
                ip        TEXT             DEFAULT '',
                regime    TEXT    NOT NULL DEFAULT '',
                rationale TEXT    NOT NULL DEFAULT ''
            )
            "#,
        )
        .execute(&pool)
        .await?;

        // ── 기존 DB 마이그레이션 ─────────────────────────────────────────────────
        // SQLite는 "ADD COLUMN IF NOT EXISTS"를 지원하지 않으므로 오류를 무시합니다.
        // 컬럼이 이미 존재하면 ALTER TABLE은 실패하며 이를 정상으로 처리합니다.
        sqlx::query(
            "ALTER TABLE audit_log ADD COLUMN regime TEXT NOT NULL DEFAULT ''"
        )
        .execute(&pool)
        .await
        .ok(); // 이미 존재하면 무시

        sqlx::query(
            "ALTER TABLE audit_log ADD COLUMN rationale TEXT NOT NULL DEFAULT ''"
        )
        .execute(&pool)
        .await
        .ok(); // 이미 존재하면 무시

        info!("Audit logger initialized with database: {}", db_path);

        Ok(Self {
            pool: Arc::new(pool),
        })
    }

    /// 감사 로그 기록
    pub async fn log(
        &self,
        user: &str,
        action: &str,
        detail: serde_json::Value,
    ) -> Result<i64, sqlx::Error> {
        self.log_with_ip(user, action, detail, None).await
    }

    /// IP 주소를 포함한 감사 로그 기록
    pub async fn log_with_ip(
        &self,
        user: &str,
        action: &str,
        detail: serde_json::Value,
        ip: Option<String>,
    ) -> Result<i64, sqlx::Error> {
        let detail_str = detail.to_string();
        let ip_str = ip.unwrap_or_default();

        let result = sqlx::query(
            r#"
            INSERT INTO audit_log (user, action, detail, ip)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(user)
        .bind(action)
        .bind(&detail_str)
        .bind(&ip_str)
        .execute(self.pool.as_ref())
        .await?;

        let id = result.last_insert_rowid();
        info!(
            "Audit log recorded: user={}, action={}, id={}",
            user, action, id
        );

        Ok(id)
    }

    /// AI 레짐 분류 결과를 감사 로그에 기록
    ///
    /// Python LLM 엔진이 호출하는 전용 메서드.
    /// regime / rationale 컬럼에 직접 값을 삽입하여 ML 연구에 바로 활용 가능.
    pub async fn log_regime_decision(
        &self,
        regime: &str,
        rationale: &str,
        market_context: serde_json::Value,
    ) -> Result<i64, sqlx::Error> {
        let detail_str = market_context.to_string();

        let result = sqlx::query(
            r#"
            INSERT INTO audit_log (user, action, detail, ip, regime, rationale)
            VALUES ('llm_engine', 'REGIME_CLASSIFICATION', ?, '', ?, ?)
            "#,
        )
        .bind(&detail_str)
        .bind(regime)
        .bind(rationale)
        .execute(self.pool.as_ref())
        .await?;

        let id = result.last_insert_rowid();
        info!(
            "Regime decision logged: regime={}, id={}",
            regime, id
        );

        Ok(id)
    }

    /// 최근 감사 로그 조회
    pub async fn query(
        &self,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<AuditEntry>, sqlx::Error> {
        let entries = sqlx::query(
            r#"
            SELECT id, ts, user, action, detail, ip, regime, rationale
            FROM audit_log
            ORDER BY id DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pool.as_ref())
        .await?;

        let audit_entries = entries
            .iter()
            .map(|row| AuditEntry {
                id:        row.get(0),
                ts:        row.get(1),
                user:      row.get(2),
                action:    row.get(3),
                detail:    row.get(4),
                ip:        row.get(5),
                regime:    row.get(6),
                rationale: row.get(7),
            })
            .collect();

        Ok(audit_entries)
    }

    /// 특정 사용자의 감사 로그 조회
    pub async fn query_by_user(
        &self,
        user: &str,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<AuditEntry>, sqlx::Error> {
        let entries = sqlx::query(
            r#"
            SELECT id, ts, user, action, detail, ip, regime, rationale
            FROM audit_log
            WHERE user = ?
            ORDER BY id DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(user)
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pool.as_ref())
        .await?;

        let audit_entries = entries
            .iter()
            .map(|row| AuditEntry {
                id:        row.get(0),
                ts:        row.get(1),
                user:      row.get(2),
                action:    row.get(3),
                detail:    row.get(4),
                ip:        row.get(5),
                regime:    row.get(6),
                rationale: row.get(7),
            })
            .collect();

        Ok(audit_entries)
    }

    /// 특정 작업의 감사 로그 조회
    pub async fn query_by_action(
        &self,
        action: &str,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<AuditEntry>, sqlx::Error> {
        let entries = sqlx::query(
            r#"
            SELECT id, ts, user, action, detail, ip, regime, rationale
            FROM audit_log
            WHERE action = ?
            ORDER BY id DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(action)
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pool.as_ref())
        .await?;

        let audit_entries = entries
            .iter()
            .map(|row| AuditEntry {
                id:        row.get(0),
                ts:        row.get(1),
                user:      row.get(2),
                action:    row.get(3),
                detail:    row.get(4),
                ip:        row.get(5),
                regime:    row.get(6),
                rationale: row.get(7),
            })
            .collect();

        Ok(audit_entries)
    }

    /// 감사 로그 총 개수
    pub async fn count(&self) -> Result<i64, sqlx::Error> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM audit_log")
            .fetch_one(self.pool.as_ref())
            .await?;

        Ok(row.get(0))
    }

    /// 날짜 범위로 감사 로그 조회
    pub async fn query_by_date_range(
        &self,
        start_ts: &str,
        end_ts: &str,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<AuditEntry>, sqlx::Error> {
        let entries = sqlx::query(
            r#"
            SELECT id, ts, user, action, detail, ip, regime, rationale
            FROM audit_log
            WHERE ts >= ? AND ts <= ?
            ORDER BY id DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(start_ts)
        .bind(end_ts)
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pool.as_ref())
        .await?;

        let audit_entries = entries
            .iter()
            .map(|row| AuditEntry {
                id:        row.get(0),
                ts:        row.get(1),
                user:      row.get(2),
                action:    row.get(3),
                detail:    row.get(4),
                ip:        row.get(5),
                regime:    row.get(6),
                rationale: row.get(7),
            })
            .collect();

        Ok(audit_entries)
    }

    /// 모든 감사 로그 삭제 (테스트 용도)
    pub async fn clear_all(&self) -> Result<(), sqlx::Error> {
        warn!("Clearing all audit logs");
        sqlx::query("DELETE FROM audit_log")
            .execute(self.pool.as_ref())
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_audit_logger_creation() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        let logger = AuditLogger::new(db_path.to_str().unwrap())
            .await
            .expect("Failed to create logger");

        let count = logger.count().await.expect("Failed to count entries");
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_audit_log_insertion() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        let logger = AuditLogger::new(db_path.to_str().unwrap())
            .await
            .expect("Failed to create logger");

        let detail = json!({ "symbol": "EURUSD", "quantity": 100 });
        let id = logger
            .log("user1", "ORDER_PLACED", detail)
            .await
            .expect("Failed to log");

        assert!(id > 0);

        let count = logger.count().await.expect("Failed to count entries");
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_audit_log_query() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        let logger = AuditLogger::new(db_path.to_str().unwrap())
            .await
            .expect("Failed to create logger");

        logger
            .log("user1", "LOGIN", json!({}))
            .await
            .expect("Failed to log");
        logger
            .log("user1", "ORDER_PLACED", json!({ "symbol": "EURUSD" }))
            .await
            .expect("Failed to log");

        let entries = logger
            .query(10, 0)
            .await
            .expect("Failed to query logs");

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].action, "ORDER_PLACED");
        assert_eq!(entries[1].action, "LOGIN");
    }

    #[tokio::test]
    async fn test_audit_log_query_by_user() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        let logger = AuditLogger::new(db_path.to_str().unwrap())
            .await
            .expect("Failed to create logger");

        logger
            .log("user1", "LOGIN", json!({}))
            .await
            .expect("Failed to log");
        logger
            .log("user2", "ORDER_PLACED", json!({}))
            .await
            .expect("Failed to log");
        logger
            .log("user1", "LOGOUT", json!({}))
            .await
            .expect("Failed to log");

        let entries = logger
            .query_by_user("user1", 10, 0)
            .await
            .expect("Failed to query logs");

        assert_eq!(entries.len(), 2);
        assert!(entries.iter().all(|e| e.user == "user1"));
    }

    #[tokio::test]
    async fn test_audit_log_query_by_action() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        let logger = AuditLogger::new(db_path.to_str().unwrap())
            .await
            .expect("Failed to create logger");

        logger
            .log("user1", "LOGIN", json!({}))
            .await
            .expect("Failed to log");
        logger
            .log("user2", "LOGIN", json!({}))
            .await
            .expect("Failed to log");
        logger
            .log("user1", "LOGOUT", json!({}))
            .await
            .expect("Failed to log");

        let entries = logger
            .query_by_action("LOGIN", 10, 0)
            .await
            .expect("Failed to query logs");

        assert_eq!(entries.len(), 2);
        assert!(entries.iter().all(|e| e.action == "LOGIN"));
    }

    #[tokio::test]
    async fn test_audit_log_with_ip() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        let logger = AuditLogger::new(db_path.to_str().unwrap())
            .await
            .expect("Failed to create logger");

        logger
            .log_with_ip("user1", "LOGIN", json!({}), Some("192.168.1.1".to_string()))
            .await
            .expect("Failed to log");

        let entries = logger
            .query(10, 0)
            .await
            .expect("Failed to query logs");

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].ip, Some("192.168.1.1".to_string()));
    }
}

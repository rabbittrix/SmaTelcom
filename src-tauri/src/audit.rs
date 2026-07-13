//! Local SQLite audit trail for every AI interaction (sqlx).

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    pub id: i64,
    pub timestamp: String,
    pub intent: String,
    pub agent_logs: String,
    pub final_decision: String,
    pub human_approver: Option<String>,
    pub execution_status: String,
    pub risk: String,
    pub ai_duration_ms: i64,
}

#[derive(Clone)]
pub struct AuditTrail {
    pool: SqlitePool,
}

impl AuditTrail {
    pub async fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let url = format!("sqlite://{}?mode=rwc", path.as_ref().display());
        let pool = SqlitePoolOptions::new()
            .max_connections(3)
            .connect(&url)
            .await
            .map_err(|e| e.to_string())?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS audit_trail (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                intent TEXT NOT NULL,
                agent_logs TEXT NOT NULL,
                final_decision TEXT NOT NULL,
                human_approver TEXT,
                execution_status TEXT NOT NULL,
                risk TEXT NOT NULL,
                ai_duration_ms INTEGER NOT NULL DEFAULT 0
            )
            "#,
        )
        .execute(&pool)
        .await
        .map_err(|e| e.to_string())?;

        Ok(Self { pool })
    }

    pub async fn log_interaction(
        &self,
        intent: &str,
        agent_logs: &str,
        final_decision: &str,
        execution_status: &str,
        risk: &str,
        ai_duration_ms: i64,
    ) -> Result<i64, String> {
        let ts = Utc::now().to_rfc3339();
        let res = sqlx::query(
            r#"
            INSERT INTO audit_trail
              (timestamp, intent, agent_logs, final_decision, human_approver, execution_status, risk, ai_duration_ms)
            VALUES (?, ?, ?, ?, NULL, ?, ?, ?)
            "#,
        )
        .bind(&ts)
        .bind(intent)
        .bind(agent_logs)
        .bind(final_decision)
        .bind(execution_status)
        .bind(risk)
        .bind(ai_duration_ms)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(res.last_insert_rowid())
    }

    pub async fn set_human_decision(
        &self,
        action_id: &str,
        approver: &str,
        status: &str,
    ) -> Result<(), String> {
        // Match by embedding action uuid inside final_decision or use latest pending.
        let _ = action_id;
        sqlx::query(
            r#"
            UPDATE audit_trail
            SET human_approver = ?, execution_status = ?
            WHERE id = (
              SELECT id FROM audit_trail
              WHERE execution_status = 'pending_hitl'
              ORDER BY id DESC LIMIT 1
            )
            "#,
        )
        .bind(approver)
        .bind(status)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn recent(&self, limit: i64) -> Result<Vec<AuditRecord>, String> {
        let rows = sqlx::query_as::<_, AuditRow>(
            r#"
            SELECT id, timestamp, intent, agent_logs, final_decision,
                   human_approver, execution_status, risk, ai_duration_ms
            FROM audit_trail
            ORDER BY id DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(Into::into).collect())
    }
}

#[derive(Debug, sqlx::FromRow)]
struct AuditRow {
    id: i64,
    timestamp: String,
    intent: String,
    agent_logs: String,
    final_decision: String,
    human_approver: Option<String>,
    execution_status: String,
    risk: String,
    ai_duration_ms: i64,
}

impl From<AuditRow> for AuditRecord {
    fn from(r: AuditRow) -> Self {
        Self {
            id: r.id,
            timestamp: r.timestamp,
            intent: r.intent,
            agent_logs: r.agent_logs,
            final_decision: r.final_decision,
            human_approver: r.human_approver,
            execution_status: r.execution_status,
            risk: r.risk,
            ai_duration_ms: r.ai_duration_ms,
        }
    }
}

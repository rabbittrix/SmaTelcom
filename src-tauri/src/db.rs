//! Persistent audit store (SQLite via sqlx — async pool, AN Level-4 compliance).
//! Table: `audit_logs` with UUID primary keys and protocol payload previews.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub id: String,
    pub timestamp: String,
    pub intent: String,
    pub final_command: String,
    pub risk_level: String,
    /// Auto-Approved | HITL-Approved | Rejected | Blocked | HITL-Pending
    pub decision: String,
    pub conflict_resolution: Option<String>,
    pub payload_preview: Option<String>,
    pub agent_logs: Option<String>,
    pub policy_citation: Option<String>,
    pub ai_duration_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ImpactReport {
    pub intents_processed: u64,
    pub auto_approved: u64,
    pub hitl_pending_or_resolved: u64,
    pub blocked: u64,
    pub human_hours_saved: f64,
    pub critical_risks_averted: u64,
    pub minutes_per_auto_approve: f64,
}

/// Insert payload for a new Judge decision.
#[derive(Debug, Clone)]
pub struct NewAuditLog {
    pub id: String,
    pub intent: String,
    pub final_command: String,
    pub risk_level: String,
    pub decision: String,
    pub conflict_resolution: String,
    pub payload_preview: String,
    pub agent_logs: String,
    pub policy_citation: String,
    pub ai_duration_ms: i64,
}

#[derive(Clone)]
pub struct AuditDb {
    pool: SqlitePool,
}

impl AuditDb {
    pub async fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let url = format!("sqlite://{}?mode=rwc", path.as_ref().display());
        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .connect(&url)
            .await
            .map_err(|e| e.to_string())?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS audit_logs (
                id TEXT PRIMARY KEY,
                timestamp TEXT NOT NULL,
                intent TEXT NOT NULL,
                final_command TEXT NOT NULL,
                risk_level TEXT NOT NULL,
                decision TEXT NOT NULL,
                conflict_resolution TEXT,
                payload_preview TEXT,
                agent_logs TEXT,
                policy_citation TEXT,
                ai_duration_ms INTEGER NOT NULL DEFAULT 0
            )
            "#,
        )
        .execute(&pool)
        .await
        .map_err(|e| e.to_string())?;

        // Index for Impact Report aggregations
        let _ = sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_audit_logs_decision ON audit_logs(decision)",
        )
        .execute(&pool)
        .await;

        Ok(Self { pool })
    }

    pub async fn insert_log(&self, entry: &NewAuditLog) -> Result<String, String> {
        let ts = Utc::now().to_rfc3339();
        let id = if entry.id.is_empty() {
            Uuid::new_v4().to_string()
        } else {
            entry.id.clone()
        };

        sqlx::query(
            r#"
            INSERT INTO audit_logs (
              id, timestamp, intent, final_command, risk_level, decision,
              conflict_resolution, payload_preview, agent_logs, policy_citation, ai_duration_ms
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&ts)
        .bind(&entry.intent)
        .bind(&entry.final_command)
        .bind(&entry.risk_level)
        .bind(&entry.decision)
        .bind(&entry.conflict_resolution)
        .bind(&entry.payload_preview)
        .bind(&entry.agent_logs)
        .bind(&entry.policy_citation)
        .bind(entry.ai_duration_ms)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        Ok(id)
    }

    pub async fn update_decision(
        &self,
        id: &str,
        decision: &str,
        payload_preview: Option<&str>,
    ) -> Result<u64, String> {
        let res = if let Some(payload) = payload_preview {
            sqlx::query(
                r#"
                UPDATE audit_logs
                SET decision = ?, payload_preview = ?
                WHERE id = ?
                "#,
            )
            .bind(decision)
            .bind(payload)
            .bind(id)
            .execute(&self.pool)
            .await
        } else {
            sqlx::query(
                r#"
                UPDATE audit_logs
                SET decision = ?
                WHERE id = ?
                "#,
            )
            .bind(decision)
            .bind(id)
            .execute(&self.pool)
            .await
        }
        .map_err(|e| e.to_string())?;

        if res.rows_affected() == 0 {
            return Err(format!("No audit_logs row for id {id}"));
        }
        Ok(res.rows_affected())
    }

    pub async fn get_audit_history(&self, limit: i64) -> Result<Vec<AuditLogEntry>, String> {
        let rows = sqlx::query_as::<_, AuditLogRow>(
            r#"
            SELECT id, timestamp, intent, final_command, risk_level, decision,
                   conflict_resolution, payload_preview, agent_logs, policy_citation, ai_duration_ms
            FROM audit_logs
            ORDER BY timestamp DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn search_logs(&self, query: &str, limit: i64) -> Result<Vec<AuditLogEntry>, String> {
        let q = format!("%{}%", query.trim());
        if query.trim().is_empty() {
            return self.get_audit_history(limit).await;
        }
        let rows = sqlx::query_as::<_, AuditLogRow>(
            r#"
            SELECT id, timestamp, intent, final_command, risk_level, decision,
                   conflict_resolution, payload_preview, agent_logs, policy_citation, ai_duration_ms
            FROM audit_logs
            WHERE intent LIKE ? OR final_command LIKE ? OR decision LIKE ?
               OR conflict_resolution LIKE ? OR id LIKE ?
            ORDER BY timestamp DESC
            LIMIT ?
            "#,
        )
        .bind(&q)
        .bind(&q)
        .bind(&q)
        .bind(&q)
        .bind(&q)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn impact_report(&self) -> Result<ImpactReport, String> {
        let row = sqlx::query_as::<_, ImpactRow>(
            r#"
            SELECT
              COUNT(*) as intents_processed,
              SUM(CASE WHEN decision = 'Auto-Approved' THEN 1 ELSE 0 END) as auto_approved,
              SUM(CASE WHEN decision IN ('HITL-Approved','Rejected','HITL-Pending') THEN 1 ELSE 0 END) as hitl_count,
              SUM(CASE WHEN decision = 'Blocked' THEN 1 ELSE 0 END) as blocked,
              SUM(CASE WHEN decision = 'Blocked'
                        AND (UPPER(risk_level) LIKE '%HIGH%' OR UPPER(risk_level) LIKE '%CRITICAL%')
                   THEN 1 ELSE 0 END) as critical_averted
            FROM audit_logs
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        const MINUTES: f64 = 15.0;
        let auto = row.auto_approved.unwrap_or(0) as u64;
        Ok(ImpactReport {
            intents_processed: row.intents_processed as u64,
            auto_approved: auto,
            hitl_pending_or_resolved: row.hitl_count.unwrap_or(0) as u64,
            blocked: row.blocked.unwrap_or(0) as u64,
            human_hours_saved: (auto as f64 * MINUTES) / 60.0,
            critical_risks_averted: row.critical_averted.unwrap_or(0) as u64,
            minutes_per_auto_approve: MINUTES,
        })
    }
}

#[derive(Debug, sqlx::FromRow)]
struct ImpactRow {
    intents_processed: i64,
    auto_approved: Option<i64>,
    hitl_count: Option<i64>,
    blocked: Option<i64>,
    critical_averted: Option<i64>,
}

#[derive(Debug, sqlx::FromRow)]
struct AuditLogRow {
    id: String,
    timestamp: String,
    intent: String,
    final_command: String,
    risk_level: String,
    decision: String,
    conflict_resolution: Option<String>,
    payload_preview: Option<String>,
    agent_logs: Option<String>,
    policy_citation: Option<String>,
    ai_duration_ms: i64,
}

impl From<AuditLogRow> for AuditLogEntry {
    fn from(r: AuditLogRow) -> Self {
        Self {
            id: r.id,
            timestamp: r.timestamp,
            intent: r.intent,
            final_command: r.final_command,
            risk_level: r.risk_level,
            decision: r.decision,
            conflict_resolution: r.conflict_resolution,
            payload_preview: r.payload_preview,
            agent_logs: r.agent_logs,
            policy_citation: r.policy_citation,
            ai_duration_ms: r.ai_duration_ms,
        }
    }
}

/// Resolve audit.db path (local-first).
pub fn default_db_paths() -> Vec<std::path::PathBuf> {
    vec![
        std::path::PathBuf::from("../data/audit.db"),
        std::path::PathBuf::from("data/audit.db"),
        std::env::temp_dir().join("smartelcom_audit.db"),
    ]
}

pub async fn open_default() -> Result<AuditDb, String> {
    let mut last = String::new();
    for path in default_db_paths() {
        match AuditDb::open(&path).await {
            Ok(db) => {
                tracing::info!("Audit DB open at {}", path.display());
                return Ok(db);
            }
            Err(e) => last = format!("{}: {e}", path.display()),
        }
    }
    Err(format!("Failed to open audit.db ({last})"))
}

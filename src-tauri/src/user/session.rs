// Local session domain model and repository operations.
// F1.05 — Local user/session model

use crate::user::UserError;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocalSession {
    pub id: String,
    pub user_id: String,
    pub branch_id: String,
    pub auth_level: String,
    pub created_at: String,
    pub expires_at: String,
    pub revoked_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionContext {
    pub session_id: String,
    pub user_id: String,
    pub full_name: String,
    pub username: Option<String>,
    pub role: String,
    pub branch_id: String,
    pub organization_id: Option<String>,
    pub auth_level: String,
    pub expires_at: String,
}

/// Default session validity duration in hours.
pub const DEFAULT_SESSION_DURATION_HOURS: i64 = 8;

/// Creates a new local session for an active user at their assigned branch.
pub fn create_local_session(
    conn: &Connection,
    user_id: &str,
    branch_id: &str,
    auth_level: &str,
    duration_hours: Option<i64>,
) -> Result<LocalSession, UserError> {
    // 1. Verify user exists, is active, and is assigned to the requested branch
    let user = conn
        .query_row(
            "SELECT id, branch_id, is_active FROM users WHERE id = ?1",
            params![user_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)? == 1,
                ))
            },
        )
        .optional()
        .map_err(|e| UserError::Database(e.to_string()))?
        .ok_or_else(|| UserError::NotFound("User not found".into()))?;

    if !user.2 {
        return Err(UserError::Validation("User account is inactive".into()));
    }

    if user.1 != branch_id {
        return Err(UserError::Validation(
            "User is not assigned to the specified branch".into(),
        ));
    }

    // 2. Verify branch exists and is active
    let branch_active: bool = conn
        .query_row(
            "SELECT is_active FROM branches WHERE id = ?1",
            params![branch_id],
            |row| Ok(row.get::<_, i64>(0)? == 1),
        )
        .optional()
        .map_err(|e| UserError::Database(e.to_string()))?
        .ok_or_else(|| UserError::BranchNotFound("Branch not found".into()))?;

    if !branch_active {
        return Err(UserError::Validation("Branch is inactive".into()));
    }

    let hours = duration_hours.unwrap_or(DEFAULT_SESSION_DURATION_HOURS);
    let duration_modifier = if hours >= 0 {
        format!("+{hours} hours")
    } else {
        format!("{hours} hours")
    };

    conn.execute(
        "INSERT INTO local_sessions (id, user_id, branch_id, auth_level, created_at, expires_at, revoked_at)
         VALUES (?1, ?2, ?3, ?4, datetime('now'), datetime('now', ?5), NULL)",
        params![session_id, user_id, branch_id, auth_level, duration_modifier],
    )
    .map_err(|e| UserError::Database(format!("Failed to create local session: {e}")))?;

    let session = conn
        .query_row(
            "SELECT id, user_id, branch_id, auth_level, created_at, expires_at, revoked_at
             FROM local_sessions WHERE id = ?1",
            params![session_id],
            |row| {
                Ok(LocalSession {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    branch_id: row.get(2)?,
                    auth_level: row.get(3)?,
                    created_at: row.get(4)?,
                    expires_at: row.get(5)?,
                    revoked_at: row.get(6)?,
                })
            },
        )
        .map_err(|e| UserError::Database(e.to_string()))?;

    Ok(session)
}

/// Strongly typed session validation errors.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SessionValidationError {
    NotFound,
    Revoked,
    Expired,
    InactiveUser,
    InactiveBranch,
    Database(String),
}

impl std::fmt::Display for SessionValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionValidationError::NotFound => write!(f, "Session not found"),
            SessionValidationError::Revoked => write!(f, "Session has been revoked"),
            SessionValidationError::Expired => write!(f, "Session has expired"),
            SessionValidationError::InactiveUser => write!(f, "User account is inactive"),
            SessionValidationError::InactiveBranch => write!(f, "Branch is inactive"),
            SessionValidationError::Database(msg) => write!(f, "Database error: {msg}"),
        }
    }
}

impl std::error::Error for SessionValidationError {}

impl From<SessionValidationError> for UserError {
    fn from(err: SessionValidationError) -> Self {
        match err {
            SessionValidationError::NotFound => {
                UserError::InvalidCredentials("Session not found".into())
            }
            SessionValidationError::Revoked => {
                UserError::InvalidCredentials("Session has been revoked".into())
            }
            SessionValidationError::Expired => {
                UserError::InvalidCredentials("Session has expired".into())
            }
            SessionValidationError::InactiveUser => {
                UserError::InvalidCredentials("User account is inactive".into())
            }
            SessionValidationError::InactiveBranch => {
                UserError::InvalidCredentials("Branch is inactive".into())
            }
            SessionValidationError::Database(msg) => UserError::Database(msg),
        }
    }
}

/// Validates an active local session against all security and tenant boundaries:
/// 1. Session exists and revoked_at IS NULL
/// 2. expires_at > datetime('now')
/// 3. User exists and is_active = 1
/// 4. Branch exists and is_active = 1
pub fn validate_local_session(
    conn: &Connection,
    session_id: &str,
) -> Result<SessionContext, SessionValidationError> {
    let result = conn
        .query_row(
            "SELECT s.id, s.user_id, u.full_name, u.username, u.role, \
             s.branch_id, b.organization_id, s.auth_level, s.expires_at, \
             s.revoked_at, u.is_active, b.is_active, \
             CASE WHEN s.expires_at IS NOT NULL AND datetime('now') <= s.expires_at THEN 1 ELSE 0 END AS is_not_expired \
             FROM local_sessions s \
             JOIN users u ON s.user_id = u.id \
             JOIN branches b ON s.branch_id = b.id \
             WHERE s.id = ?1",
            params![session_id],
            |row| {
                let revoked_at: Option<String> = row.get(9)?;
                let user_active: bool = row.get::<_, i64>(10)? == 1;
                let branch_active: bool = row.get::<_, i64>(11)? == 1;
                let is_not_expired: bool = row.get::<_, i64>(12)? == 1;

                Ok((
                    SessionContext {
                        session_id: row.get(0)?,
                        user_id: row.get(1)?,
                        full_name: row.get(2)?,
                        username: row.get(3)?,
                        role: row.get(4)?,
                        branch_id: row.get(5)?,
                        organization_id: row.get(6)?,
                        auth_level: row.get(7)?,
                        expires_at: row.get(8)?,
                    },
                    revoked_at,
                    user_active,
                    branch_active,
                    is_not_expired,
                ))
            },
        )
        .optional()
        .map_err(|e| SessionValidationError::Database(e.to_string()))?
        .ok_or(SessionValidationError::NotFound)?;

    let (context, revoked_at, user_active, branch_active, is_not_expired) = result;

    if revoked_at.is_some() {
        return Err(SessionValidationError::Revoked);
    }

    if !is_not_expired {
        return Err(SessionValidationError::Expired);
    }

    if !user_active {
        return Err(SessionValidationError::InactiveUser);
    }

    if !branch_active {
        return Err(SessionValidationError::InactiveBranch);
    }

    Ok(context)
}

/// Explicitly revokes a local session (logout).
pub fn revoke_local_session(conn: &Connection, session_id: &str) -> Result<(), UserError> {
    let rows_affected = conn
        .execute(
            "UPDATE local_sessions SET revoked_at = datetime('now') WHERE id = ?1 AND revoked_at IS NULL",
            params![session_id],
        )
        .map_err(|e| UserError::Database(format!("Failed to revoke session: {e}")))?;

    if rows_affected == 0 {
        // Check if session exists at all
        let exists: bool = conn
            .query_row(
                "SELECT 1 FROM local_sessions WHERE id = ?1",
                params![session_id],
                |_| Ok(true),
            )
            .optional()
            .map_err(|e| UserError::Database(e.to_string()))?
            .unwrap_or(false);

        if !exists {
            return Err(UserError::NotFound("Session not found".into()));
        }
    }

    Ok(())
}

/// Retrieves the most recent active (unrevoked, unexpired) session for a user.
#[allow(dead_code)]
pub fn get_active_session(
    conn: &Connection,
    user_id: &str,
) -> Result<Option<LocalSession>, UserError> {
    conn.query_row(
        "SELECT id, user_id, branch_id, auth_level, created_at, expires_at, revoked_at
         FROM local_sessions
         WHERE user_id = ?1 AND revoked_at IS NULL AND datetime('now') <= expires_at
         ORDER BY created_at DESC, id DESC
         LIMIT 1",
        params![user_id],
        |row| {
            Ok(LocalSession {
                id: row.get(0)?,
                user_id: row.get(1)?,
                branch_id: row.get(2)?,
                auth_level: row.get(3)?,
                created_at: row.get(4)?,
                expires_at: row.get(5)?,
                revoked_at: row.get(6)?,
            })
        },
    )
    .optional()
    .map_err(|e| UserError::Database(e.to_string()))
}

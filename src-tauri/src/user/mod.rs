// Local User domain model and repository operations.
// F1.05 — Local user/session model

pub mod session;

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct User {
    pub id: String,
    pub branch_id: String,
    pub full_name: String,
    pub username: Option<String>,
    pub role: String,
    pub is_active: bool,
    pub supabase_user_id: Option<String>,
    pub auth_provider: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserInput {
    pub branch_id: String,
    pub full_name: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub pin: Option<String>,
    pub role: String,
    pub supabase_user_id: Option<String>,
    pub auth_provider: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserInput {
    pub full_name: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub pin: Option<String>,
    pub role: Option<String>,
    pub is_active: Option<bool>,
    pub supabase_user_id: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum UserError {
    #[error("Validation error: {0}")]
    Validation(String),

    #[error("User not found: {0}")]
    NotFound(String),

    #[error("Branch not found: {0}")]
    BranchNotFound(String),

    #[error("Username already exists: {0}")]
    DuplicateUsername(String),

    #[error("Supabase user ID already mapped: {0}")]
    DuplicateSupabaseId(String),

    #[error("Invalid credentials: {0}")]
    InvalidCredentials(String),

    #[error("Database error: {0}")]
    Database(String),
}

/// Generates a cryptographically salted SHA-256 hash in `salt$digest` format.
pub fn hash_secret(secret: &str) -> String {
    let salt = uuid::Uuid::new_v4().simple().to_string();
    let mut hasher = Sha256::new();
    hasher.update(format!("{salt}:{secret}").as_bytes());
    let digest = hasher.finalize();
    let hex_digest = hex_encode(&digest);
    format!("{salt}${hex_digest}")
}

/// Verifies a candidate secret against a stored `salt$digest` string using constant-time comparison.
pub fn verify_secret(candidate: &str, stored: &str) -> bool {
    let parts: Vec<&str> = stored.split('$').collect();
    if parts.len() != 2 {
        return false;
    }
    let salt = parts[0];
    let expected_hex = parts[1];

    let mut hasher = Sha256::new();
    hasher.update(format!("{salt}:{candidate}").as_bytes());
    let actual_digest = hasher.finalize();
    let actual_hex = hex_encode(&actual_digest);

    constant_time_eq(&actual_hex, expected_hex)
}

/// Formats bytes as lowercase hex without external dependencies.
fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(s, "{:02x}", b);
    }
    s
}

/// Constant-time string equality check to prevent timing side channels.
fn constant_time_eq(a: &str, b: &str) -> bool {
    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();
    if a_bytes.len() != b_bytes.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a_bytes.iter().zip(b_bytes.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Creates a new user record in the local database.
pub fn create_user(conn: &Connection, input: CreateUserInput) -> Result<User, UserError> {
    let full_name = input.full_name.trim();
    if full_name.is_empty() {
        return Err(UserError::Validation(
            "User full name cannot be empty".into(),
        ));
    }
    if full_name.chars().count() > 255 {
        return Err(UserError::Validation(
            "User full name cannot exceed 255 characters".into(),
        ));
    }

    let role = input.role.trim();
    if role.is_empty() {
        return Err(UserError::Validation("User role cannot be empty".into()));
    }

    let branch_id = input.branch_id.trim();
    if branch_id.is_empty() {
        return Err(UserError::Validation("Branch ID cannot be empty".into()));
    }

    // Verify branch exists
    let branch_exists: bool = conn
        .query_row(
            "SELECT 1 FROM branches WHERE id = ?1",
            params![branch_id],
            |_| Ok(true),
        )
        .optional()
        .map_err(|e| UserError::Database(e.to_string()))?
        .unwrap_or(false);

    if !branch_exists {
        return Err(UserError::BranchNotFound(format!(
            "Branch '{branch_id}' does not exist"
        )));
    }

    // Process username
    let username = match input.username {
        Some(u) => {
            let trimmed = u.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                // Check username uniqueness
                let existing: bool = conn
                    .query_row(
                        "SELECT 1 FROM users WHERE username = ?1",
                        params![trimmed],
                        |_| Ok(true),
                    )
                    .optional()
                    .map_err(|e| UserError::Database(e.to_string()))?
                    .unwrap_or(false);

                if existing {
                    return Err(UserError::DuplicateUsername(format!(
                        "Username '{trimmed}' already exists"
                    )));
                }
                Some(trimmed)
            }
        }
        None => None,
    };

    // Process Supabase User ID uniqueness if provided
    let supabase_user_id = match input.supabase_user_id {
        Some(s) => {
            let trimmed = s.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                let existing: bool = conn
                    .query_row(
                        "SELECT 1 FROM users WHERE supabase_user_id = ?1",
                        params![trimmed],
                        |_| Ok(true),
                    )
                    .optional()
                    .map_err(|e| UserError::Database(e.to_string()))?
                    .unwrap_or(false);

                if existing {
                    return Err(UserError::DuplicateSupabaseId(format!(
                        "Supabase user ID '{trimmed}' is already linked to a user"
                    )));
                }
                Some(trimmed)
            }
        }
        None => None,
    };

    let password_hash = input
        .password
        .as_deref()
        .filter(|p| !p.trim().is_empty())
        .map(hash_secret);
    let pin_hash = input
        .pin
        .as_deref()
        .filter(|p| !p.trim().is_empty())
        .map(hash_secret);
    let auth_provider = input.auth_provider.unwrap_or_else(|| "local".into());

    let user_id = uuid::Uuid::new_v4().to_string();

    conn.execute(
        "INSERT INTO users (id, branch_id, full_name, username, password_hash, pin_hash, role, is_active, supabase_user_id, auth_provider, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, ?8, ?9, datetime('now'))",
        params![
            user_id,
            branch_id,
            full_name,
            username,
            password_hash,
            pin_hash,
            role,
            supabase_user_id,
            auth_provider,
        ],
    )
    .map_err(|e| UserError::Database(format!("Failed to create user: {e}")))?;

    get_user(conn, &user_id)
}

/// Retrieves a user by ID.
pub fn get_user(conn: &Connection, id: &str) -> Result<User, UserError> {
    conn.query_row(
        "SELECT id, branch_id, full_name, username, role, is_active, supabase_user_id, auth_provider, created_at
         FROM users WHERE id = ?1",
        params![id],
        |row| {
            Ok(User {
                id: row.get(0)?,
                branch_id: row.get(1)?,
                full_name: row.get(2)?,
                username: row.get(3)?,
                role: row.get(4)?,
                is_active: row.get::<_, i64>(5)? == 1,
                supabase_user_id: row.get(6)?,
                auth_provider: row.get(7)?,
                created_at: row.get(8)?,
            })
        },
    )
    .optional()
    .map_err(|e| UserError::Database(e.to_string()))?
    .ok_or_else(|| UserError::NotFound(format!("User '{id}' not found")))
}

/// Retrieves a user by username.
pub fn get_user_by_username(conn: &Connection, username: &str) -> Result<User, UserError> {
    conn.query_row(
        "SELECT id, branch_id, full_name, username, role, is_active, supabase_user_id, auth_provider, created_at
         FROM users WHERE username = ?1",
        params![username.trim()],
        |row| {
            Ok(User {
                id: row.get(0)?,
                branch_id: row.get(1)?,
                full_name: row.get(2)?,
                username: row.get(3)?,
                role: row.get(4)?,
                is_active: row.get::<_, i64>(5)? == 1,
                supabase_user_id: row.get(6)?,
                auth_provider: row.get(7)?,
                created_at: row.get(8)?,
            })
        },
    )
    .optional()
    .map_err(|e| UserError::Database(e.to_string()))?
    .ok_or_else(|| UserError::NotFound(format!("User with username '{username}' not found")))
}

/// Retrieves a user by linked Supabase user ID.
pub fn get_user_by_supabase_id(
    conn: &Connection,
    supabase_user_id: &str,
) -> Result<User, UserError> {
    conn.query_row(
        "SELECT id, branch_id, full_name, username, role, is_active, supabase_user_id, auth_provider, created_at
         FROM users WHERE supabase_user_id = ?1",
        params![supabase_user_id.trim()],
        |row| {
            Ok(User {
                id: row.get(0)?,
                branch_id: row.get(1)?,
                full_name: row.get(2)?,
                username: row.get(3)?,
                role: row.get(4)?,
                is_active: row.get::<_, i64>(5)? == 1,
                supabase_user_id: row.get(6)?,
                auth_provider: row.get(7)?,
                created_at: row.get(8)?,
            })
        },
    )
    .optional()
    .map_err(|e| UserError::Database(e.to_string()))?
    .ok_or_else(|| UserError::NotFound(format!("User with Supabase ID '{supabase_user_id}' not found")))
}

/// Updates user fields safely.
pub fn update_user(conn: &Connection, id: &str, input: UpdateUserInput) -> Result<User, UserError> {
    let existing = get_user(conn, id)?;

    if let Some(ref name) = input.full_name {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(UserError::Validation("Full name cannot be empty".into()));
        }
        if trimmed.chars().count() > 255 {
            return Err(UserError::Validation(
                "Full name cannot exceed 255 characters".into(),
            ));
        }
    }

    if let Some(ref role) = input.role {
        if role.trim().is_empty() {
            return Err(UserError::Validation("Role cannot be empty".into()));
        }
    }

    if let Some(ref u) = input.username {
        let trimmed = u.trim();
        if !trimmed.is_empty() && Some(trimmed) != existing.username.as_deref() {
            let conflict: bool = conn
                .query_row(
                    "SELECT 1 FROM users WHERE username = ?1 AND id != ?2",
                    params![trimmed, id],
                    |_| Ok(true),
                )
                .optional()
                .map_err(|e| UserError::Database(e.to_string()))?
                .unwrap_or(false);

            if conflict {
                return Err(UserError::DuplicateUsername(format!(
                    "Username '{trimmed}' already exists"
                )));
            }
        }
    }

    if let Some(ref s) = input.supabase_user_id {
        let trimmed = s.trim();
        if !trimmed.is_empty() && Some(trimmed) != existing.supabase_user_id.as_deref() {
            let conflict: bool = conn
                .query_row(
                    "SELECT 1 FROM users WHERE supabase_user_id = ?1 AND id != ?2",
                    params![trimmed, id],
                    |_| Ok(true),
                )
                .optional()
                .map_err(|e| UserError::Database(e.to_string()))?
                .unwrap_or(false);

            if conflict {
                return Err(UserError::DuplicateSupabaseId(format!(
                    "Supabase ID '{trimmed}' already mapped"
                )));
            }
        }
    }

    let mut query = String::from("UPDATE users SET ");
    let mut clauses = Vec::new();
    let mut param_values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(ref name) = input.full_name {
        clauses.push("full_name = ?");
        param_values.push(Box::new(name.trim().to_string()));
    }
    if let Some(ref username) = input.username {
        let val = if username.trim().is_empty() {
            None
        } else {
            Some(username.trim().to_string())
        };
        clauses.push("username = ?");
        param_values.push(Box::new(val));
    }
    if let Some(ref pw) = input.password {
        let hash = if pw.trim().is_empty() {
            None
        } else {
            Some(hash_secret(pw))
        };
        clauses.push("password_hash = ?");
        param_values.push(Box::new(hash));
    }
    if let Some(ref pin) = input.pin {
        let hash = if pin.trim().is_empty() {
            None
        } else {
            Some(hash_secret(pin))
        };
        clauses.push("pin_hash = ?");
        param_values.push(Box::new(hash));
    }
    if let Some(ref role) = input.role {
        clauses.push("role = ?");
        param_values.push(Box::new(role.trim().to_string()));
    }
    if let Some(is_active) = input.is_active {
        clauses.push("is_active = ?");
        param_values.push(Box::new(if is_active { 1i64 } else { 0i64 }));
    }
    if let Some(ref s) = input.supabase_user_id {
        let val = if s.trim().is_empty() {
            None
        } else {
            Some(s.trim().to_string())
        };
        clauses.push("supabase_user_id = ?");
        param_values.push(Box::new(val));
    }

    if clauses.is_empty() {
        return Ok(existing);
    }

    query.push_str(&clauses.join(", "));
    query.push_str(" WHERE id = ?");
    param_values.push(Box::new(id.to_string()));

    let params_ref: Vec<&dyn rusqlite::ToSql> = param_values.iter().map(|b| b.as_ref()).collect();

    conn.execute(&query, params_ref.as_slice())
        .map_err(|e| UserError::Database(format!("Failed to update user: {e}")))?;

    get_user(conn, id)
}

/// Lists all users for a given branch.
pub fn list_users(conn: &Connection, branch_id: &str) -> Result<Vec<User>, UserError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, branch_id, full_name, username, role, is_active, supabase_user_id, auth_provider, created_at
             FROM users WHERE branch_id = ?1
             ORDER BY created_at ASC, id ASC",
        )
        .map_err(|e| UserError::Database(e.to_string()))?;

    let rows = stmt
        .query_map(params![branch_id], |row| {
            Ok(User {
                id: row.get(0)?,
                branch_id: row.get(1)?,
                full_name: row.get(2)?,
                username: row.get(3)?,
                role: row.get(4)?,
                is_active: row.get::<_, i64>(5)? == 1,
                supabase_user_id: row.get(6)?,
                auth_provider: row.get(7)?,
                created_at: row.get(8)?,
            })
        })
        .map_err(|e| UserError::Database(e.to_string()))?;

    let mut users = Vec::new();
    for row in rows {
        users.push(row.map_err(|e| UserError::Database(e.to_string()))?);
    }
    Ok(users)
}

/// Verifies user password and returns the authenticated User record.
pub fn verify_user_password(
    conn: &Connection,
    username: &str,
    password: &str,
) -> Result<User, UserError> {
    let user_opt: Option<(User, Option<String>)> = conn
        .query_row(
            "SELECT id, branch_id, full_name, username, role, is_active, supabase_user_id, auth_provider, created_at, password_hash
             FROM users WHERE username = ?1",
            params![username.trim()],
            |row| {
                Ok((
                    User {
                        id: row.get(0)?,
                        branch_id: row.get(1)?,
                        full_name: row.get(2)?,
                        username: row.get(3)?,
                        role: row.get(4)?,
                        is_active: row.get::<_, i64>(5)? == 1,
                        supabase_user_id: row.get(6)?,
                        auth_provider: row.get(7)?,
                        created_at: row.get(8)?,
                    },
                    row.get::<_, Option<String>>(9)?,
                ))
            },
        )
        .optional()
        .map_err(|e| UserError::Database(e.to_string()))?;

    match user_opt {
        Some((user, Some(stored_hash))) => {
            if !verify_secret(password, &stored_hash) {
                return Err(UserError::InvalidCredentials(
                    "Invalid username or password".into(),
                ));
            }
            if !user.is_active {
                return Err(UserError::InvalidCredentials(
                    "User account is inactive".into(),
                ));
            }
            Ok(user)
        }
        _ => Err(UserError::InvalidCredentials(
            "Invalid username or password".into(),
        )),
    }
}

/// Verifies user cashier PIN and returns the authenticated User record.
pub fn verify_user_pin(conn: &Connection, user_id: &str, pin: &str) -> Result<User, UserError> {
    let user_opt: Option<(User, Option<String>)> = conn
        .query_row(
            "SELECT id, branch_id, full_name, username, role, is_active, supabase_user_id, auth_provider, created_at, pin_hash
             FROM users WHERE id = ?1",
            params![user_id.trim()],
            |row| {
                Ok((
                    User {
                        id: row.get(0)?,
                        branch_id: row.get(1)?,
                        full_name: row.get(2)?,
                        username: row.get(3)?,
                        role: row.get(4)?,
                        is_active: row.get::<_, i64>(5)? == 1,
                        supabase_user_id: row.get(6)?,
                        auth_provider: row.get(7)?,
                        created_at: row.get(8)?,
                    },
                    row.get::<_, Option<String>>(9)?,
                ))
            },
        )
        .optional()
        .map_err(|e| UserError::Database(e.to_string()))?;

    match user_opt {
        Some((user, Some(stored_hash))) => {
            if !verify_secret(pin, &stored_hash) {
                return Err(UserError::InvalidCredentials("Invalid PIN".into()));
            }
            if !user.is_active {
                return Err(UserError::InvalidCredentials(
                    "User account is inactive".into(),
                ));
            }
            Ok(user)
        }
        _ => Err(UserError::InvalidCredentials("Invalid PIN".into())),
    }
}

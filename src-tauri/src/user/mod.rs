// Local User domain model and repository operations.
// F1.05 — Local user/session model

pub mod session;

use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

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

#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UserError {
    Validation(String),
    NotFound(String),
    BranchNotFound(String),
    DuplicateUsername(String),
    DuplicateSupabaseId(String),
    InvalidCredentials(String),
    Database(String),
}

impl std::fmt::Display for UserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserError::Validation(msg) => write!(f, "Validation error: {msg}"),
            UserError::NotFound(msg) => write!(f, "User not found: {msg}"),
            UserError::BranchNotFound(msg) => write!(f, "Branch not found: {msg}"),
            UserError::DuplicateUsername(msg) => write!(f, "Username already exists: {msg}"),
            UserError::DuplicateSupabaseId(msg) => {
                write!(f, "Supabase user ID already mapped: {msg}")
            }
            UserError::InvalidCredentials(msg) => write!(f, "Invalid credentials: {msg}"),
            UserError::Database(msg) => write!(f, "Database error: {msg}"),
        }
    }
}

impl std::error::Error for UserError {}

/// Precomputed valid Argon2id hash for constant-time decoy verification to defeat timing-based user enumeration.
pub const DECOY_ARGON2_HASH: &str =
    "$argon2id$v=19$m=19456,t=2,p=1$ZGVjb3lzYWx0MTIzNDU2Nw$DecoyHashToPreventUserEnumerationTimingAttack123";

/// Generates a cryptographically secure Argon2id hash in standard PHC format.
#[allow(dead_code)]
pub fn hash_secret(secret: &str) -> Result<String, UserError> {
    let salt_bytes = uuid::Uuid::new_v4();
    let salt = SaltString::encode_b64(salt_bytes.as_bytes())
        .map_err(|e| UserError::Database(format!("Salt encoding error: {e}")))?;
    let argon2 = Argon2::default();
    argon2
        .hash_password(secret.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| UserError::Database(format!("Argon2 hashing error: {e}")))
}

/// Verifies a candidate secret against a stored Argon2id PHC string.
pub fn verify_secret(candidate: &str, stored_hash: &str) -> bool {
    let parsed_hash = match PasswordHash::new(stored_hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(candidate.as_bytes(), &parsed_hash)
        .is_ok()
}

struct RateLimiterState {
    map: HashMap<String, (u32, Instant)>,
    client_saturated_until: HashMap<String, Instant>,
    client_overflow_failures: HashMap<String, u32>,
}

/// Thread-safe in-memory rate limiter with bounded memory growth, strict active-lockout preservation,
/// client-scoped admission throttling under saturation, and composite client scoping.
pub struct RateLimiter {
    state: Mutex<RateLimiterState>,
    max_attempts: u32,
    lockout_duration: Duration,
    max_entries: usize,
}

impl RateLimiter {
    pub fn new(max_attempts: u32, lockout_secs: u64, max_entries: usize) -> Self {
        Self {
            state: Mutex::new(RateLimiterState {
                map: HashMap::new(),
                client_saturated_until: HashMap::new(),
                client_overflow_failures: HashMap::new(),
            }),
            max_attempts,
            lockout_duration: Duration::from_secs(lockout_secs),
            max_entries,
        }
    }

    /// Returns the number of currently tracked entries (useful for test assertions).
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.state.lock().map(|s| s.map.len()).unwrap_or(0)
    }

    /// Checks if the limiter has no tracked entries.
    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Checks if a client-identity key is currently locked out or subject to client admission throttle.
    pub fn check(&self, rate_key: &str) -> Result<(), UserError> {
        let mut state = self
            .state
            .lock()
            .map_err(|e| UserError::Database(e.to_string()))?;

        let client_id = rate_key.split(':').next().unwrap_or(rate_key);

        if let Some((count, last_failed)) = state.map.get(rate_key) {
            if *count >= self.max_attempts {
                if last_failed.elapsed() < self.lockout_duration {
                    return Err(UserError::InvalidCredentials(
                        "Too many failed attempts. Account is temporarily locked. Please try again later.".into(),
                    ));
                }
                state.map.remove(rate_key);
            }
        } else if let Some(until) = state.client_saturated_until.get(client_id) {
            if Instant::now() < *until {
                return Err(UserError::InvalidCredentials(
                    "Too many failed attempts. Authentication service is temporarily throttling requests from this client. Please try again later.".into(),
                ));
            }
            state.client_saturated_until.remove(client_id);
            state.client_overflow_failures.remove(client_id);
        }
        Ok(())
    }

    /// Records a failure. Active lockouts are strictly preserved and never evicted.
    /// Under total saturation of active lockouts, client-scoped admission throttling prevents unthrottled bypass.
    pub fn record_failure(&self, rate_key: &str) {
        if let Ok(mut state) = self.state.lock() {
            let client_id = rate_key.split(':').next().unwrap_or(rate_key).to_string();

            // Update existing entry directly
            if let Some(entry) = state.map.get_mut(rate_key) {
                if entry.0 >= self.max_attempts && entry.1.elapsed() >= self.lockout_duration {
                    *entry = (1, Instant::now());
                } else {
                    entry.0 += 1;
                    entry.1 = Instant::now();
                }
                return;
            }

            // 1. Purge expired entries using a single predicate
            let lockout = self.lockout_duration;
            state
                .map
                .retain(|_, (_, last_failed)| last_failed.elapsed() < lockout);

            // Purge expired client saturation throttle records
            state
                .client_saturated_until
                .retain(|_, until| Instant::now() < *until);

            // 2. If at capacity, evict the oldest non-locked entry only
            if state.map.len() >= self.max_entries {
                let oldest_non_locked = state
                    .map
                    .iter()
                    .filter(|(_, (count, _))| *count < self.max_attempts)
                    .min_by_key(|(_, (_, time))| *time)
                    .map(|(k, _)| k.clone());

                if let Some(non_locked_key) = oldest_non_locked {
                    state.map.remove(&non_locked_key);
                }
            }

            // 3. Insert new entry if space is available (never evicting an active lockout)
            if state.map.len() < self.max_entries {
                state.map.insert(rate_key.to_string(), (1, Instant::now()));
            } else {
                // Capacity fully saturated by active lockouts: apply client-scoped admission throttling
                let count = state
                    .client_overflow_failures
                    .entry(client_id.clone())
                    .or_insert(0);
                *count += 1;
                if *count >= self.max_attempts {
                    state
                        .client_saturated_until
                        .insert(client_id, Instant::now() + self.lockout_duration);
                }
            }
        }
    }

    /// Clears the failure record on successful authentication.
    pub fn record_success(&self, rate_key: &str) {
        if let Ok(mut state) = self.state.lock() {
            state.map.remove(rate_key);
            let client_id = rate_key.split(':').next().unwrap_or(rate_key);
            state.client_overflow_failures.remove(client_id);
        }
    }

    /// Clears all entries (useful in testing).
    #[cfg(test)]
    pub fn reset_all(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.map.clear();
            state.client_saturated_until.clear();
            state.client_overflow_failures.clear();
        }
    }
}

/// Global rate limiter enforcing max 5 failed attempts per client identity with 30-second lockout.
static GLOBAL_AUTH_RATE_LIMITER: OnceLock<RateLimiter> = OnceLock::new();

/// Exposes the global rate limiter for production operations.
pub fn get_auth_rate_limiter() -> &'static RateLimiter {
    GLOBAL_AUTH_RATE_LIMITER.get_or_init(|| RateLimiter::new(5, 30, 10_000))
}

/// Maps a database row to a User entity.
pub fn map_user_row(row: &Row<'_>) -> rusqlite::Result<User> {
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
}

/// Creates a new user record in the local database.
#[allow(dead_code)]
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

    let username = match input.username {
        Some(u) => {
            let trimmed = u.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
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

    let password_hash = match input.password.as_deref().filter(|p| !p.trim().is_empty()) {
        Some(p) => Some(hash_secret(p)?),
        None => None,
    };

    let pin_hash = match input.pin.as_deref().filter(|p| !p.trim().is_empty()) {
        Some(p) => Some(hash_secret(p)?),
        None => None,
    };

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
#[allow(dead_code)]
pub fn get_user(conn: &Connection, id: &str) -> Result<User, UserError> {
    conn.query_row(
        "SELECT id, branch_id, full_name, username, role, is_active, supabase_user_id, auth_provider, created_at
         FROM users WHERE id = ?1",
        params![id],
        map_user_row,
    )
    .optional()
    .map_err(|e| UserError::Database(e.to_string()))?
    .ok_or_else(|| UserError::NotFound(format!("User '{id}' not found")))
}

/// Retrieves a user by username.
#[allow(dead_code)]
pub fn get_user_by_username(conn: &Connection, username: &str) -> Result<User, UserError> {
    conn.query_row(
        "SELECT id, branch_id, full_name, username, role, is_active, supabase_user_id, auth_provider, created_at
         FROM users WHERE username = ?1",
        params![username.trim()],
        map_user_row,
    )
    .optional()
    .map_err(|e| UserError::Database(e.to_string()))?
    .ok_or_else(|| UserError::NotFound(format!("User with username '{username}' not found")))
}

/// Retrieves a user by linked Supabase user ID.
#[allow(dead_code)]
pub fn get_user_by_supabase_id(
    conn: &Connection,
    supabase_user_id: &str,
) -> Result<User, UserError> {
    conn.query_row(
        "SELECT id, branch_id, full_name, username, role, is_active, supabase_user_id, auth_provider, created_at
         FROM users WHERE supabase_user_id = ?1",
        params![supabase_user_id.trim()],
        map_user_row,
    )
    .optional()
    .map_err(|e| UserError::Database(e.to_string()))?
    .ok_or_else(|| UserError::NotFound(format!("User with Supabase ID '{supabase_user_id}' not found")))
}

/// Updates user fields safely.
#[allow(dead_code)]
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
            Some(hash_secret(pw)?)
        };
        clauses.push("password_hash = ?");
        param_values.push(Box::new(hash));
    }
    if let Some(ref pin) = input.pin {
        let hash = if pin.trim().is_empty() {
            None
        } else {
            Some(hash_secret(pin)?)
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

    let params_ref: Vec<&dyn rusqlite::ToSql> = param_values.iter().map(Box::as_ref).collect();

    conn.execute(&query, params_ref.as_slice())
        .map_err(|e| UserError::Database(format!("Failed to update user: {e}")))?;

    get_user(conn, id)
}

/// Lists all users for a given branch.
#[allow(dead_code)]
pub fn list_users(conn: &Connection, branch_id: &str) -> Result<Vec<User>, UserError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, branch_id, full_name, username, role, is_active, supabase_user_id, auth_provider, created_at
             FROM users WHERE branch_id = ?1
             ORDER BY created_at ASC, id ASC",
        )
        .map_err(|e| UserError::Database(e.to_string()))?;

    let rows = stmt
        .query_map(params![branch_id], map_user_row)
        .map_err(|e| UserError::Database(e.to_string()))?;

    let mut users = Vec::new();
    for row in rows {
        users.push(row.map_err(|e| UserError::Database(e.to_string()))?);
    }
    Ok(users)
}

/// Shared internal credential verification with timing decoy defense and rate limiting.
fn verify_credential_internal(
    limiter: &RateLimiter,
    rate_key: &str,
    user_and_hash: Option<(User, Option<String>)>,
    candidate: &str,
    error_msg: &'static str,
) -> Result<User, UserError> {
    match user_and_hash {
        Some((user, Some(stored_hash))) => {
            if !verify_secret(candidate, &stored_hash) {
                limiter.record_failure(rate_key);
                return Err(UserError::InvalidCredentials(error_msg.into()));
            }
            if !user.is_active {
                limiter.record_failure(rate_key);
                return Err(UserError::InvalidCredentials(error_msg.into()));
            }
            limiter.record_success(rate_key);
            Ok(user)
        }
        _ => {
            let _ = verify_secret(candidate, DECOY_ARGON2_HASH);
            limiter.record_failure(rate_key);
            Err(UserError::InvalidCredentials(error_msg.into()))
        }
    }
}

/// Verifies user password with injectable rate limiter and client scoping.
pub fn verify_user_password_with_limiter(
    conn: &Connection,
    limiter: &RateLimiter,
    client_id: &str,
    username: &str,
    password: &str,
) -> Result<User, UserError> {
    let rate_key = format!("{client_id}:user:{}", username.trim().to_lowercase());
    limiter.check(&rate_key)?;

    let user_opt: Option<(User, Option<String>)> = conn
        .query_row(
            "SELECT id, branch_id, full_name, username, role, is_active, supabase_user_id, auth_provider, created_at, password_hash
             FROM users WHERE username = ?1",
            params![username.trim()],
            |row| Ok((map_user_row(row)?, row.get::<_, Option<String>>(9)?)),
        )
        .optional()
        .map_err(|e| UserError::Database(e.to_string()))?;

    verify_credential_internal(
        limiter,
        &rate_key,
        user_opt,
        password,
        "Invalid username or password",
    )
}

/// Verifies user password using the global rate limiter and standard client context.
pub fn verify_user_password(
    conn: &Connection,
    username: &str,
    password: &str,
) -> Result<User, UserError> {
    verify_user_password_with_limiter(
        conn,
        get_auth_rate_limiter(),
        "local_terminal",
        username,
        password,
    )
}

/// Verifies user cashier PIN with injectable rate limiter and client scoping.
pub fn verify_user_pin_with_limiter(
    conn: &Connection,
    limiter: &RateLimiter,
    client_id: &str,
    user_id: &str,
    pin: &str,
) -> Result<User, UserError> {
    let rate_key = format!("{client_id}:pin:{}", user_id.trim());
    limiter.check(&rate_key)?;

    let user_opt: Option<(User, Option<String>)> = conn
        .query_row(
            "SELECT id, branch_id, full_name, username, role, is_active, supabase_user_id, auth_provider, created_at, pin_hash
             FROM users WHERE id = ?1",
            params![user_id.trim()],
            |row| Ok((map_user_row(row)?, row.get::<_, Option<String>>(9)?)),
        )
        .optional()
        .map_err(|e| UserError::Database(e.to_string()))?;

    verify_credential_internal(limiter, &rate_key, user_opt, pin, "Invalid PIN")
}

/// Verifies user cashier PIN using the global rate limiter and standard client context.
pub fn verify_user_pin(conn: &Connection, user_id: &str, pin: &str) -> Result<User, UserError> {
    verify_user_pin_with_limiter(
        conn,
        get_auth_rate_limiter(),
        "local_terminal",
        user_id,
        pin,
    )
}

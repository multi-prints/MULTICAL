use std::collections::HashMap;
use std::sync::Mutex;

use pbkdf2::pbkdf2_hmac;
use rand::Rng;
use sha2::Sha512;

use crate::db::Database;
use crate::models::*;

const SALT: &[u8] = b"multiprints-salt-key";
const PBKDF2_ITERATIONS: u32 = 1000;

pub struct AuthManager {
    pub sessions: Mutex<HashMap<String, Session>>,
}

#[derive(Debug, Clone)]
pub struct Session {
    pub token: String,
    pub username: String,
    pub role: String,
    pub permissions: Vec<String>,
    pub created_at: u64,
}

impl AuthManager {
    pub fn new() -> Self {
        AuthManager {
            sessions: Mutex::new(HashMap::new()),
        }
    }

    /// Hash a password using PBKDF2-SHA512 (same as JS version)
    pub fn hash_password(password: &str) -> String {
        let mut hash = [0u8; 64];
        pbkdf2_hmac::<Sha512>(password.as_bytes(), SALT, PBKDF2_ITERATIONS, &mut hash);
        hex::encode(hash)
    }

    /// Verify a password against a hash
    pub fn verify_password(password: &str, hash: &str) -> bool {
        Self::hash_password(password) == hash
    }

    /// Generate a random session token
    fn generate_token() -> String {
        let mut rng = rand::thread_rng();
        let bytes: [u8; 32] = rng.gen();
        hex::encode(bytes)
    }

    /// Initialize default users (admin/admin, employee/employee) if they don't exist
    pub fn init_default_users(&self, db: &Database) {
        if db.get_user_by_username("admin").ok().flatten().is_none() {
            println!("Creating default admin user...");
            let hash = Self::hash_password("admin");
            db.add_user("admin", &hash, "admin", "[\"all\"]").ok();
        }

        if db.get_user_by_username("employee").ok().flatten().is_none() {
            println!("Creating default employee user...");
            let hash = Self::hash_password("employee");
            db.add_user("employee", &hash, "employee",
                "[\"view_printing\",\"edit_printing\",\"convert_to_debt\",\"view_sales\",\"edit_sales\"]"
            ).ok();
        }
    }

    /// Authenticate a user
    pub fn authenticate(&self, db: &Database, username: &str, password: &str) -> LoginResponse {
        match db.get_user_by_username(username) {
            Ok(Some(user)) => {
                if Self::verify_password(password, &user.password_hash) {
                    // Parse permissions
                    let permissions: Vec<String> = user
                        .permissions
                        .as_deref()
                        .and_then(|p| serde_json::from_str(p).ok())
                        .unwrap_or_else(|| {
                            if user.role == "admin" {
                                vec!["all".to_string()]
                            } else {
                                vec![
                                    "view_printing".to_string(),
                                    "edit_printing".to_string(),
                                    "convert_to_debt".to_string(),
                                    "view_sales".to_string(),
                                    "edit_sales".to_string(),
                                ]
                            }
                        });

                    let token = Self::generate_token();
                    let session = Session {
                        token: token.clone(),
                        username: username.to_string(),
                        role: user.role.clone(),
                        permissions: permissions.clone(),
                        created_at: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                    };

                    if let Ok(mut sessions) = self.sessions.lock() {
                        sessions.insert(token.clone(), session);
                    }

                    LoginResponse {
                        success: true,
                        token: Some(token),
                        user: Some(UserInfo {
                            username: username.to_string(),
                            role: user.role,
                            permissions,
                        }),
                        error: None,
                    }
                } else {
                    LoginResponse {
                        success: false,
                        token: None,
                        user: None,
                        error: Some("Invalid username or password".to_string()),
                    }
                }
            }
            Ok(None) => LoginResponse {
                success: false,
                token: None,
                user: None,
                error: Some("Invalid username or password".to_string()),
            },
            Err(e) => LoginResponse {
                success: false,
                token: None,
                user: None,
                error: Some(e),
            },
        }
    }

    /// Validate a session token
    pub fn validate_token(&self, token: &str) -> bool {
        if let Ok(sessions) = self.sessions.lock() {
            sessions.contains_key(token)
        } else {
            false
        }
    }

    /// Get session info by token
    pub fn get_session(&self, token: &str) -> Option<Session> {
        if let Ok(sessions) = self.sessions.lock() {
            sessions.get(token).cloned()
        } else {
            None
        }
    }

    /// Logout (invalidate token)
    pub fn logout(&self, token: &str) {
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.remove(token);
        }
    }

    /// Add a new user (admin only)
    pub fn add_user(&self, db: &Database, username: &str, password: &str, role: &str) -> SuccessResponse {
        if db.get_user_by_username(username).ok().flatten().is_some() {
            return SuccessResponse {
                success: false,
                error: Some("User already exists".to_string()),
                message: None,
            };
        }

        let permissions = if role == "admin" {
            "[\"all\"]"
        } else {
            "[\"view_printing\",\"edit_printing\",\"convert_to_debt\",\"view_sales\",\"edit_sales\"]"
        };

        let hash = Self::hash_password(password);

        match db.add_user(username, &hash, role, permissions) {
            Ok(_) => SuccessResponse {
                success: true,
                error: None,
                message: Some("User created successfully".to_string()),
            },
            Err(e) => SuccessResponse {
                success: false,
                error: Some(e),
                message: None,
            },
        }
    }

    /// Update user password
    pub fn update_password(&self, db: &Database, username: &str, old_password: &str, new_password: &str) -> SuccessResponse {
        match db.get_user_by_username(username) {
            Ok(Some(user)) => {
                if !Self::verify_password(old_password, &user.password_hash) {
                    return SuccessResponse {
                        success: false,
                        error: Some("Current password is incorrect".to_string()),
                        message: None,
                    };
                }

                let new_hash = Self::hash_password(new_password);
                match db.update_user_password(username, &new_hash) {
                    Ok(_) => SuccessResponse {
                        success: true,
                        error: None,
                        message: Some("Password updated successfully".to_string()),
                    },
                    Err(e) => SuccessResponse {
                        success: false,
                        error: Some(e),
                        message: None,
                    },
                }
            }
            Ok(None) => SuccessResponse {
                success: false,
                error: Some("User not found".to_string()),
                message: None,
            },
            Err(e) => SuccessResponse {
                success: false,
                error: Some(e),
                message: None,
            },
        }
    }

    /// Update username
    pub fn update_username(&self, db: &Database, old_username: &str, new_username: &str) -> SuccessResponse {
        if old_username != new_username {
            if db.get_user_by_username(new_username).ok().flatten().is_some() {
                return SuccessResponse {
                    success: false,
                    error: Some("Username already taken".to_string()),
                    message: None,
                };
            }
        }

        match db.update_username(old_username, new_username) {
            Ok(_) => {
                // Update active sessions
                if let Ok(mut sessions) = self.sessions.lock() {
                    for session in sessions.values_mut() {
                        if session.username == old_username {
                            session.username = new_username.to_string();
                        }
                    }
                }

                SuccessResponse {
                    success: true,
                    error: None,
                    message: Some("Username updated successfully".to_string()),
                }
            }
            Err(e) => SuccessResponse {
                success: false,
                error: Some(e),
                message: None,
            },
        }
    }

    /// Get all users (admin only)
    pub fn get_all_users(&self, db: &Database) -> Result<Vec<User>, String> {
        db.get_all_users()
    }

    /// Delete a user (admin only, cannot delete admin)
    pub fn delete_user(&self, db: &Database, username: &str) -> SuccessResponse {
        if username == "admin" {
            return SuccessResponse {
                success: false,
                error: Some("Cannot delete admin user".to_string()),
                message: None,
            };
        }

        if db.get_user_by_username(username).ok().flatten().is_none() {
            return SuccessResponse {
                success: false,
                error: Some("User not found".to_string()),
                message: None,
            };
        }

        match db.delete_user(username) {
            Ok(_) => {
                // Invalidate all sessions for this user
                if let Ok(mut sessions) = self.sessions.lock() {
                    sessions.retain(|_, s| s.username != username);
                }

                SuccessResponse {
                    success: true,
                    error: None,
                    message: Some("User deleted successfully".to_string()),
                }
            }
            Err(e) => SuccessResponse {
                success: false,
                error: Some(e),
                message: None,
            },
        }
    }
}

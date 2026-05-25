use tauri::State;

use crate::auth::AuthManager;
use crate::db::Database;
use crate::models::*;

#[tauri::command]
pub fn login(
    db: State<'_, Database>,
    auth: State<'_, AuthManager>,
    username: String,
    password: String,
) -> LoginResponse {
    auth.authenticate(&db, &username, &password)
}

#[tauri::command]
pub fn logout(auth: State<'_, AuthManager>, token: String) -> SuccessResponse {
    auth.logout(&token);
    SuccessResponse {
        success: true,
        error: None,
        message: Some("Logged out".to_string()),
    }
}

#[tauri::command]
pub fn validate_session(auth: State<'_, AuthManager>, token: Option<String>) -> bool {
    match token {
        Some(t) => auth.validate_token(&t),
        None => false,
    }
}

#[tauri::command]
pub fn get_session(auth: State<'_, AuthManager>, token: String) -> SessionResponse {
    match auth.get_session(&token) {
        Some(session) => SessionResponse {
            success: true,
            session: Some(UserInfo {
                username: session.username,
                role: session.role,
                permissions: session.permissions,
            }),
        },
        None => SessionResponse {
            success: false,
            session: None,
        },
    }
}

#[tauri::command]
pub fn add_user(
    db: State<'_, Database>,
    auth: State<'_, AuthManager>,
    username: String,
    password: String,
    role: String,
) -> SuccessResponse {
    auth.add_user(&db, &username, &password, &role)
}

#[tauri::command]
pub fn update_password(
    db: State<'_, Database>,
    auth: State<'_, AuthManager>,
    username: String,
    old_password: String,
    new_password: String,
) -> SuccessResponse {
    auth.update_password(&db, &username, &old_password, &new_password)
}

#[tauri::command]
pub fn update_username(
    db: State<'_, Database>,
    auth: State<'_, AuthManager>,
    old_username: String,
    new_username: String,
) -> SuccessResponse {
    auth.update_username(&db, &old_username, &new_username)
}

#[tauri::command]
pub fn get_all_users(db: State<'_, Database>) -> Result<Vec<User>, String> {
    db.get_all_users()
}

#[tauri::command]
pub fn delete_user(
    db: State<'_, Database>,
    auth: State<'_, AuthManager>,
    username: String,
) -> SuccessResponse {
    auth.delete_user(&db, &username)
}

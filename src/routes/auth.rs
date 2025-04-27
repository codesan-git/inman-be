use actix_web::{post, get, web, HttpResponse, Responder, cookie::{Cookie, SameSite}};
use serde::{Deserialize, Serialize};
use jsonwebtoken::{encode, Header, EncodingKey};
use chrono::{Utc, Duration};
use sqlx::PgPool;
use uuid::Uuid;
use crate::routes::user::UserRole;

// SECRET diambil dari env JWT_SECRET

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub role: String,
}

#[derive(Debug, sqlx::FromRow)]
struct UserRow {
    id: Uuid,
    name: String,
    password_hash: Option<String>,
    role: UserRole,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub name: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct CheckUserRequest {
    pub name: String,
}

#[post("/api/check-user")]
pub async fn check_user(
    pool: web::Data<PgPool>,
    form: web::Json<CheckUserRequest>,
) -> impl Responder {
    let user = sqlx::query_as::<_, UserRow>(
        "SELECT id, name, password_hash, role FROM users WHERE name = $1"
    )
    .bind(&form.name)
    .fetch_optional(pool.get_ref())
    .await
    .unwrap();

    if let Some(user) = user {
        let password_exists = user.password_hash.as_ref().map(|h| !h.is_empty()).unwrap_or(false);
        return HttpResponse::Ok().json(serde_json::json!({
            "id": user.id,
            "name": user.name,
            "password_exists": password_exists
        }));
    } else {
        HttpResponse::NotFound().body("User tidak ditemukan")
    }
}


#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user_id: Uuid, 
    pub username: String,
    pub role: String,
}

#[post("/api/login")]
pub async fn login(
    pool: web::Data<PgPool>,
    form: web::Json<LoginRequest>,
) -> impl Responder {
    let user = sqlx::query_as::<_, UserRow>(
        "SELECT id, name, password_hash, role FROM users WHERE name = $1"
    )
    .bind(&form.name)
    .fetch_optional(pool.get_ref())
    .await
    .unwrap();

    if let Some(user) = user {
        if let Some(ref hash) = user.password_hash {
            if verify_password(&form.password, hash) {
                let exp = (Utc::now() + Duration::hours(24)).timestamp() as usize;
                let claims = Claims {
                    sub: user.id.to_string(),
                    exp,
                    role: user.role.to_string(),
                };
                let secret = std::env::var("JWT_SECRET").expect("JWT_SECRET harus di-set di .env");
                let token = encode(
                    &Header::default(),
                    &claims,
                    &EncodingKey::from_secret(secret.as_ref()),
                ).expect("JWT encode error");
                let cookie = Cookie::build("token", token.clone())
                    .http_only(true)
                    .secure(false) // HARUS false untuk dev HTTP agar cookie terkirim
                    .same_site(SameSite::Lax)
                    .path("/")
                    .finish();
                return HttpResponse::Ok()
                    .cookie(cookie)
                    .json(LoginResponse {
                        token,
                        user_id: user.id,
                        username: user.name,
                        role: user.role.to_string(),
                    });
            }
        }
    }
    HttpResponse::Unauthorized().json(serde_json::json!({ "error": "Username atau password salah" }))
}

// Password verification menggunakan argon2
use argon2::{Argon2, PasswordHash, PasswordVerifier};

fn verify_password(password: &str, hash: &str) -> bool {
    if let Ok(parsed_hash) = PasswordHash::new(hash) {
        Argon2::default().verify_password(password.as_bytes(), &parsed_hash).is_ok()
    } else {
        false
    }
}

// Helper untuk middleware validasi JWT bisa dibuat menyusul

#[get("/api/logout")]
pub async fn logout() -> impl Responder {
    let cookie = Cookie::build("token", "")
        .http_only(true)
        .secure(false)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(actix_web::cookie::time::Duration::new(-1, 0)) // Expired cookie
        .finish();
    
    HttpResponse::Ok()
        .cookie(cookie)
        .json(serde_json::json!({ "success": true, "message": "Logged out successfully" }))
}

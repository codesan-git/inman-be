use actix_web::{post, web, HttpResponse, Responder, cookie::{Cookie, SameSite}};
use serde::{Deserialize, Serialize};
use jsonwebtoken::{encode, Header, EncodingKey};
use chrono::{Utc, Duration};
use sqlx::PgPool;
use uuid::Uuid;

const SECRET: &[u8] = b"super_secret_key_change_me";

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
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
    let user = sqlx::query!(
        "SELECT id, name, password_hash FROM users WHERE name = $1",
        form.name
    )
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
}

#[post("/api/login")]
pub async fn login(
    pool: web::Data<PgPool>,
    form: web::Json<LoginRequest>,
) -> impl Responder {
    let user = sqlx::query!(
        "SELECT id, name, password_hash FROM users WHERE name = $1",
        form.name
    )
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
                };
                let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(SECRET)).unwrap();
                let cookie = Cookie::build("token", token.clone())
                    .http_only(true)
                    .secure(true) // aktifkan hanya di HTTPS production
                    .same_site(SameSite::Lax)
                    .path("/")
                    .finish();
                return HttpResponse::Ok()
                    .cookie(cookie)
                    .json(LoginResponse {
                        token,
                        user_id: user.id,
                        username: user.name,
                    });
            }
        }
    }
    HttpResponse::Unauthorized().body("Username atau password salah")
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

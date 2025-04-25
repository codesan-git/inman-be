use actix_web::{get, HttpRequest, HttpResponse, Responder};
use crate::routes::auth::Claims;
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::Serialize;
use uuid::Uuid;
use sqlx::PgPool;

#[derive(Serialize)]
pub struct MeResponse {
    pub id: Uuid,
    pub name: String,
    pub role: String,
}

#[get("/api/me")]
pub async fn me(req: HttpRequest, pool: actix_web::web::Data<PgPool>) -> impl Responder {
    let secret = std::env::var("JWT_SECRET").expect("JWT_SECRET harus di-set di .env");
    let token_cookie = req.cookie("token");
    if token_cookie.is_none() {
        return HttpResponse::Unauthorized().json(serde_json::json!({ "error": "No token" }));
    }
    let token_cookie = token_cookie.unwrap();
    let token = token_cookie.value().to_owned();
    let token_data = decode::<Claims>(&token, &DecodingKey::from_secret(secret.as_bytes()), &Validation::default());
    if let Ok(data_token) = token_data {
        let user_id = match Uuid::parse_str(&data_token.claims.sub) {
            Ok(uuid) => uuid,
            Err(_) => return HttpResponse::Unauthorized().json(serde_json::json!({ "error": "Invalid user id in token" })),
        };
        // Query DB untuk ambil nama dan role
        let row = sqlx::query_as::<_, crate::routes::user::User>("SELECT id, name, email, phone_number, avatar_url, role, created_at FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(pool.get_ref())
            .await;
        match row {
            Ok(user) => HttpResponse::Ok().json(MeResponse {
                id: user.id,
                name: user.name,
                role: user.role.to_string(),
            }),
            Err(_) => HttpResponse::Unauthorized().json(serde_json::json!({ "error": "User not found" })),
        }
    } else {
        HttpResponse::Unauthorized().json(serde_json::json!({ "error": "Invalid token" }))
    }
}

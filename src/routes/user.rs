use actix_web::{get, post, patch, delete, web, HttpResponse, Responder};
use argon2::password_hash::rand_core;
use sqlx::{PgPool, FromRow};
use actix_web::web::Data;
use serde::{Serialize, Deserialize};
use uuid::Uuid;



use crate::middleware::admin_guard::is_admin;

#[derive(Serialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub email: Option<String>,
    pub phone_number: Option<String>,
    pub avatar_url: Option<String>,
    pub role_id: Uuid,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize)]
pub struct NewUser {
    pub name: String,
    pub role_id: Option<Uuid>,
}

#[derive(Deserialize)]
pub struct UpdateUser {
    pub name: Option<String>,
    pub email: Option<String>,
    pub phone_number: Option<String>,
    pub avatar_url: Option<String>,
    pub role_id: Option<Uuid>,
    pub password: Option<String>,
    pub from_login: Option<bool>,
}

#[get("")]
pub async fn get_all_users(db: Data<PgPool>, claims: crate::middleware::jwt_extractor::Claims) -> impl Responder {
    if !is_admin(&claims, db.get_ref()).await {
        return HttpResponse::Forbidden().json(serde_json::json!({ "message": "Hanya admin yang boleh akses" }));
    }
    let users = sqlx::query_as::<_, User>(
        "SELECT id, name, email, phone_number, avatar_url, role_id, created_at FROM users"
    )
    .fetch_all(db.get_ref())
    .await;

    match users {
        Ok(users) => HttpResponse::Ok().json(users),
        Err(e) => {
            eprintln!("DB error: {:?}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({ "message": format!("DB error: {:?}", e) }))
        }
    }
}

#[post("")]
pub async fn create_user(db: Data<PgPool>, new_user: web::Json<NewUser>, claims: crate::middleware::jwt_extractor::Claims) -> impl Responder {
    if !is_admin(&claims, db.get_ref()).await {
        return HttpResponse::Forbidden().json(serde_json::json!({ "message": "Hanya admin yang boleh akses" }));
    }
    let role_id = new_user.role_id.unwrap_or_else(|| Uuid::parse_str("f27eecfb-897d-493a-aeb8-1bbce725f5c4").unwrap());
let user = sqlx::query_as::<_, User>(
    "INSERT INTO users (name, role_id) VALUES ($1, $2) RETURNING id, name, email, phone_number, avatar_url, role_id, created_at",
    )
    .bind(&new_user.name)
    .bind(role_id)
    .fetch_one(db.get_ref())
    .await;

match user {
    Ok(user) => HttpResponse::Ok().json(user),
    Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "message": format!("DB error: {}", e) })),
}
}

#[patch("/{id}")]
pub async fn update_user(
    db: Data<PgPool>,
    path: web::Path<Uuid>,
    update: web::Json<UpdateUser>,
) -> impl Responder {
    use sqlx::QueryBuilder;
    let id = path.into_inner();
    enum FieldValue<'a> {
        Name(&'a String),
        Email(&'a String),
        PhoneNumber(&'a String),
        AvatarUrl(&'a String),
        RoleId(&'a Uuid),
    }
    let mut sets = Vec::new();
    if let Some(name) = &update.name {
        sets.push(("name", FieldValue::Name(name)));
    }
    if let Some(email) = &update.email {
        sets.push(("email", FieldValue::Email(email)));
    }
    if let Some(phone_number) = &update.phone_number {
        sets.push(("phone_number", FieldValue::PhoneNumber(phone_number)));
    }
    if let Some(avatar_url) = &update.avatar_url {
        sets.push(("avatar_url", FieldValue::AvatarUrl(avatar_url)));
    }
    if let Some(role_id) = &update.role_id {
        sets.push(("role_id", FieldValue::RoleId(role_id)));
    }
    let mut password_updated = false;
    if let Some(new_password) = &update.password {
        // hash password baru pakai argon2
        use argon2::{Argon2, PasswordHasher};
        use rand_core::OsRng;
        let salt = argon2::password_hash::SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2.hash_password(new_password.as_bytes(), &salt).unwrap().to_string();
        let res = sqlx::query!(
            "UPDATE users SET password_hash = $1 WHERE id = $2",
            password_hash,
            id
        )
        .execute(db.get_ref())
        .await;
        match res {
            Ok(_) => password_updated = true,
            Err(e) => return HttpResponse::InternalServerError().json(serde_json::json!({ "message": format!("DB error: {}", e) })),
        }
    }
    let from_login = update.from_login.unwrap_or(false);
    if sets.is_empty() {
        // Jika hanya update password saja, anggap sukses
        if password_updated {
            if from_login {
                return HttpResponse::Ok().json(serde_json::json!({
                    "redirect": true,
                    "message": "Password berhasil dibuat, silakan login!"
                }));
            } else {
                return HttpResponse::Ok().json(serde_json::json!({ "message": "Password berhasil diupdate!" }));
            }
        } else {
            return HttpResponse::BadRequest().json(serde_json::json!({ "message": "No fields to update" }));
        }
    }
    let mut qb = QueryBuilder::new("UPDATE users SET ");
    for (i, (field, value)) in sets.iter().enumerate() {
        if i > 0 {
            qb.push(", ");
        }
        qb.push(format!("{} = ", field));
        match value {
            FieldValue::Name(val) => { qb.push_bind(val); },
            FieldValue::Email(val) => { qb.push_bind(val); },
            FieldValue::PhoneNumber(val) => { qb.push_bind(val); },
            FieldValue::AvatarUrl(val) => { qb.push_bind(val); },
            FieldValue::RoleId(val) => { qb.push_bind(val); },
        }
    }
    qb.push(" WHERE id = ").push_bind(id);

    let query = qb.build();
    let result = query.execute(db.get_ref()).await;
    match result {
        Ok(_) => HttpResponse::Ok().json("User berhasil diupdate!"),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "message": format!("DB error: {}", e) })),
    }
}


#[delete("/{id}")]
pub async fn delete_user(db: Data<PgPool>, path: web::Path<Uuid>, claims: crate::middleware::jwt_extractor::Claims) -> impl Responder {
    if !is_admin(&claims, db.get_ref()).await {
        return HttpResponse::Forbidden().json(serde_json::json!({ "message": "Hanya admin yang boleh akses" }));
    }
    let id = path.into_inner();
    let result = sqlx::query!("DELETE FROM users WHERE id = $1", id)
        .execute(db.get_ref())
        .await;
    match result {
        Ok(_) => HttpResponse::Ok().json("User berhasil dihapus!"),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "message": format!("DB error: {}", e) })),
    }
}


pub async fn protected_admin(db: Data<PgPool>, claims: crate::middleware::jwt_extractor::Claims) -> impl actix_web::Responder {
    if !is_admin(&claims, db.get_ref()).await {
        return actix_web::HttpResponse::Forbidden().json(serde_json::json!({ "message": "Hanya admin yang boleh akses endpoint ini" }));
    }
    actix_web::HttpResponse::Ok().body("Hello admin!")
}

pub fn user_config(cfg: &mut web::ServiceConfig) {
    cfg
        .service(get_all_users)
        .service(create_user)
        .service(update_user)
        .service(delete_user)
        .service(
            actix_web::web::resource("/admin-only").route(actix_web::web::get().to(protected_admin))
        );
}
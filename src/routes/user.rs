use actix_web::{get, post, patch, delete, web, HttpResponse, Responder};
use sqlx::{PgPool, FromRow};
use actix_web::web::Data;
use serde::{Serialize, Deserialize};
use uuid::Uuid;

#[derive(sqlx::Type, Serialize, Deserialize, Debug)]
#[sqlx(type_name = "user_role")]
pub enum UserRole {
    #[sqlx(rename = "admin")]
    Admin,
    #[sqlx(rename = "staff")]
    Staff,
}

#[derive(Serialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub email: Option<String>,
    pub phone_number: Option<String>,
    pub avatar_url: Option<String>,
    pub role: UserRole,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize)]
pub struct NewUser {
    pub name: String,
}

#[derive(Deserialize)]
pub struct UpdateUser {
    pub name: Option<String>,
    pub email: Option<String>,
    pub phone_number: Option<String>,
    pub avatar_url: Option<String>,
    pub role: Option<UserRole>,
}

#[get("/api/users")]
pub async fn get_all_users(db: Data<PgPool>) -> impl Responder {
    let users = sqlx::query_as::<_, User>(
        "SELECT id, name, email, phone_number, avatar_url, role, created_at FROM users"
    )
    .fetch_all(db.get_ref())
    .await;

    match users {
        Ok(users) => HttpResponse::Ok().json(users),
        Err(e) => {
            eprintln!("DB error: {:?}", e);
            HttpResponse::InternalServerError().body(format!("DB error: {:?}", e))
        }
    }
}

#[post("/api/users")]
pub async fn create_user(db: Data<PgPool>, new_user: web::Json<NewUser>) -> impl Responder {
    let result = sqlx::query!(
        "INSERT INTO users (name) VALUES ($1)",
        new_user.name
    )
    .execute(db.get_ref())
    .await;

    match result {
        Ok(_) => HttpResponse::Ok().json("User berhasil ditambahkan!"),
        Err(e) => HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    }
}

#[patch("/api/users/{id}")]
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
        Role(&'a UserRole),
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
    if let Some(role) = &update.role {
        sets.push(("role", FieldValue::Role(role)));
    }
    if sets.is_empty() {
        return HttpResponse::BadRequest().body("No fields to update");
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
            FieldValue::Role(val) => { qb.push_bind(val); },
        }
    }
    qb.push(" WHERE id = ").push_bind(id);

    let query = qb.build();
    let result = query.execute(db.get_ref()).await;
    match result {
        Ok(_) => HttpResponse::Ok().json("User berhasil diupdate!"),
        Err(e) => HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    }
}


#[delete("/api/users/{id}")]
pub async fn delete_user(db: Data<PgPool>, path: web::Path<Uuid>) -> impl Responder {
    let id = path.into_inner();
    let result = sqlx::query!("DELETE FROM users WHERE id = $1", id)
        .execute(db.get_ref())
        .await;
    match result {
        Ok(_) => HttpResponse::Ok().json("User berhasil dihapus!"),
        Err(e) => HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    }
}

use actix_web::{get, post, delete, web, HttpResponse, Responder, patch};
use actix_web::HttpMessage;
use crate::middleware::jwt_extractor::Claims;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct ItemLog {
    pub id: Uuid,
    pub item_id: Uuid,
    pub item_name: Option<String>,
    pub action: String,
    pub before: Option<serde_json::Value>,
    pub after: Option<serde_json::Value>,
    pub note: Option<String>,
    pub by: Option<Uuid>,
    pub user_name: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[get("/item_logs/{item_id}")]
pub async fn get_item_logs(pool: web::Data<PgPool>, path: web::Path<Uuid>) -> impl Responder {
    let item_id = path.into_inner();
    let logs = sqlx::query_as::<_, ItemLog>("SELECT * FROM item_logs WHERE item_id = $1 ORDER BY created_at DESC")
        .bind(item_id)
        .fetch_all(pool.get_ref())
        .await;
    match logs {
        Ok(logs) => HttpResponse::Ok().json(logs),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

#[get("/item_logs")]
pub async fn get_all_item_logs(pool: web::Data<PgPool>) -> impl Responder {
    let logs = sqlx::query_as::<_, ItemLog>(
        r#"SELECT l.id, l.item_id, i.name as item_name, l.action, l.before, l.after, l.note, l.by, u.name as user_name, l.created_at
        FROM item_logs l
        LEFT JOIN users u ON l.by = u.id
        LEFT JOIN items i ON l.item_id = i.id
        ORDER BY l.created_at DESC"#
    )
    .fetch_all(pool.get_ref())
    .await;
    match logs {
        Ok(logs) => HttpResponse::Ok().json(logs),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Item {
    pub id: Uuid,
    pub name: String,
    pub category: Category,
    pub quantity: i32,
    pub condition: Condition,
    pub location_id: Option<Uuid>,
    pub photo_url: Option<String>,
    pub source: ItemSource,
    pub donor_id: Option<Uuid>,
    pub procurement_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "category")]
#[serde(rename_all = "lowercase")]
pub enum Category {
    #[sqlx(rename = "electronics")]
    Electronics,
    #[sqlx(rename = "prayer")]
    Prayer,
    #[sqlx(rename = "furniture")]
    Furniture,
}

#[derive(Debug, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "condition")]
#[serde(rename_all = "lowercase")]
pub enum Condition {
    #[sqlx(rename = "good")]
    Good,
    #[sqlx(rename = "damaged")]
    Damaged,
    #[sqlx(rename = "lost")]
    Lost,
}

#[derive(Debug, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "item_source")]
#[serde(rename_all = "lowercase")]
pub enum ItemSource {
    #[sqlx(rename = "existing")]
    Existing,
    #[sqlx(rename = "donation")]
    Donation,
    #[sqlx(rename = "procurement")]
    Procurement,
}

#[get("")]
pub async fn get_items(pool: web::Data<PgPool>) -> impl Responder {
    println!("[Handler] get_items dipanggil");
    let items = sqlx::query_as::<_, Item>("SELECT * FROM items ORDER BY created_at DESC")
        .fetch_all(pool.get_ref())
        .await;
    match items {
        Ok(items) => HttpResponse::Ok().json(items),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

#[get("/{id}")]
pub async fn get_item_by_id(pool: web::Data<PgPool>, path: web::Path<String>) -> impl Responder {
    let id_str = path.into_inner();
    let id = match uuid::Uuid::parse_str(&id_str) {
        Ok(uuid) => uuid,
        Err(_) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Invalid UUID format"
            }));
        }
    };
    let item = sqlx::query_as::<_, Item>("SELECT * FROM items WHERE id = $1")
        .bind(id)
        .fetch_optional(pool.get_ref())
        .await;
    match item {
        Ok(Some(item)) => HttpResponse::Ok().json(item),
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({"error": "Item not found"})),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

#[derive(Debug, Deserialize)]
pub struct NewItem {
    pub name: String,
    pub category: Category,
    pub quantity: Option<i32>,
    pub condition: Condition,
    pub location_id: Option<Uuid>,
    pub photo_url: Option<String>,
    pub source: ItemSource,
    pub donor_id: Option<Uuid>,
    pub procurement_id: Option<Uuid>,
}

#[post("")]
pub async fn create_item(claims: Claims, pool: web::Data<PgPool>, form: web::Json<NewItem>) -> impl Responder {
    println!("DEBUG payload: {:?}", form);

    let q = sqlx::query_as::<_, Item>(
        "INSERT INTO items (name, category, quantity, condition, location_id, photo_url, source, donor_id, procurement_id) \
        VALUES ($1, $2, COALESCE($3, 1), $4, $5, $6, $7, $8, $9) RETURNING *"
    )
    .bind(&form.name)
    .bind(&form.category)
    .bind(form.quantity)
    .bind(&form.condition)
    .bind(form.location_id)
    .bind(&form.photo_url)
    .bind(&form.source)
    .bind(form.donor_id)
    .bind(form.procurement_id)
    .fetch_one(pool.get_ref())
    .await;
    match q {
        Ok(item) => {
            // Insert log
            let _ = sqlx::query("INSERT INTO item_logs (item_id, action, before, after, by) VALUES ($1, $2, $3, $4, $5)")
                .bind(item.id)
                .bind("create")
                .bind(None::<serde_json::Value>)
                .bind(Some(serde_json::to_value(&item).unwrap()))
                .bind(uuid::Uuid::parse_str(&claims.sub).ok())
                .execute(pool.get_ref()).await;
            HttpResponse::Ok().json(item)
        },
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateItem {
    pub name: Option<String>,
    pub category: Option<Category>,
    pub quantity: Option<i32>,
    pub condition: Option<Condition>,
    pub location_id: Option<Uuid>,
    pub photo_url: Option<String>,
    pub source: Option<ItemSource>,
    pub donor_id: Option<Uuid>,
    pub procurement_id: Option<Uuid>,
}

#[patch("/{id}")]
pub async fn update_item(claims: Claims, pool: web::Data<PgPool>, path: web::Path<Uuid>, form: web::Json<UpdateItem>) -> impl Responder {
    let id = path.into_inner();
    // Ambil data sebelum update
    let before = sqlx::query_as::<_, Item>("SELECT * FROM items WHERE id = $1")
        .bind(id)
        .fetch_optional(pool.get_ref())
        .await
        .ok()
        .flatten();
    let q = sqlx::query_as::<_, Item>(
        "UPDATE items SET \
            name = COALESCE($1, name), \
            category = COALESCE($2, category), \
            quantity = COALESCE($3, quantity), \
            condition = COALESCE($4, condition), \
            location_id = COALESCE($5, location_id), \
            photo_url = COALESCE($6, photo_url), \
            source = COALESCE($7, source), \
            donor_id = COALESCE($8, donor_id), \
            procurement_id = COALESCE($9, procurement_id) \
        WHERE id = $10 RETURNING *"
    )
    .bind(&form.name)
    .bind(&form.category)
    .bind(form.quantity)
    .bind(&form.condition)
    .bind(form.location_id)
    .bind(&form.photo_url)
    .bind(&form.source)
    .bind(form.donor_id)
    .bind(form.procurement_id)
    .bind(id)
    .fetch_optional(pool.get_ref())
    .await;
    match q {
        Ok(Some(item)) => {
            // Insert log
            let _ = sqlx::query("INSERT INTO item_logs (item_id, action, before, after, by) VALUES ($1, $2, $3, $4, $5)")
                .bind(item.id)
                .bind("update")
                .bind(before.map(|b| serde_json::to_value(&b).unwrap()))
                .bind(Some(serde_json::to_value(&item).unwrap()))
                .bind(uuid::Uuid::parse_str(&claims.sub).ok())
                .execute(pool.get_ref()).await;
            HttpResponse::Ok().json(item)
        },
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({"error": "Item not found"})),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

#[delete("/{id}")]
pub async fn delete_item(claims: Claims, pool: web::Data<PgPool>, path: web::Path<Uuid>) -> impl Responder {
    let id = path.into_inner();
    // Ambil data sebelum delete
    let before = sqlx::query_as::<_, Item>("SELECT * FROM items WHERE id = $1")
        .bind(id)
        .fetch_optional(pool.get_ref())
        .await
        .ok()
        .flatten();
    let q = sqlx::query("DELETE FROM items WHERE id = $1 RETURNING id")
        .bind(id)
        .fetch_optional(pool.get_ref())
        .await;
    match q {
        Ok(Some(row)) => {
            // Insert log
            if let Some(b) = before {
                let _ = sqlx::query("INSERT INTO item_logs (item_id, action, before, after, by) VALUES ($1, $2, $3, $4, $5)")
                    .bind(b.id)
                    .bind("delete")
                    .bind(Some(serde_json::to_value(&b).unwrap()))
                    .bind(None::<serde_json::Value>)
                    .bind(uuid::Uuid::parse_str(&claims.sub).ok())
                    .execute(pool.get_ref()).await;
            }
            HttpResponse::Ok().json(serde_json::json!({"success": true}))
        },
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({"error": "Item not found"})),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

pub fn items_config(cfg: &mut web::ServiceConfig) {
    cfg.service(get_items)
    .service(get_all_item_logs)
    .service(get_item_logs)
        .service(get_item_by_id)
        .service(create_item)
        .service(update_item)
        .service(delete_item);
}

use actix_web::{get, post, delete, web, HttpResponse, Responder, patch};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

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

#[get("/api/items")]
pub async fn get_items(pool: web::Data<PgPool>) -> impl Responder {
    let items = sqlx::query_as::<_, Item>("SELECT * FROM items ORDER BY created_at DESC")
        .fetch_all(pool.get_ref())
        .await;
    match items {
        Ok(items) => HttpResponse::Ok().json(items),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

#[get("/api/items/{id}")]
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

#[post("/api/items")]
pub async fn create_item(pool: web::Data<PgPool>, form: web::Json<NewItem>) -> impl Responder {
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
        Ok(item) => HttpResponse::Ok().json(item),
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

#[patch("/api/items/{id}")]
pub async fn update_item(
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
    form: web::Json<UpdateItem>,
) -> impl Responder {
    let id = path.into_inner();
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
        Ok(Some(item)) => HttpResponse::Ok().json(item),
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({"error": "Item not found"})),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

#[delete("/api/items/{id}")]
pub async fn delete_item(pool: web::Data<PgPool>, path: web::Path<Uuid>) -> impl Responder {
    let id = path.into_inner();
    let q = sqlx::query("DELETE FROM items WHERE id = $1 RETURNING id")
        .bind(id)
        .fetch_optional(pool.get_ref())
        .await;
    match q {
        Ok(Some(_)) => HttpResponse::Ok().json(serde_json::json!({"success": true})),
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({"error": "Item not found"})),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(get_items)
        .service(get_item_by_id)
        .service(create_item)
        .service(update_item)
        .service(delete_item);
}

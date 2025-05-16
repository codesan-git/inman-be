use actix_web::{get, post, delete, web, HttpResponse, Responder, patch};
use actix_web::http::header::ContentType;
use qrcode::QrCode;
use image::Luma;
use image::EncodableLayout;
use image::ImageEncoder;
use std::io::Cursor;

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
    let logs = sqlx::query_as::<_, ItemLog>(
        r#"SELECT l.id, l.item_id, i.name as item_name, l.action, l.before, l.after, l.note, l.by, u.name as user_name, l.created_at
        FROM item_logs l
        LEFT JOIN items i ON l.item_id = i.id
        LEFT JOIN users u ON l.by = u.id
        WHERE l.item_id = $1
        ORDER BY l.created_at DESC"#
    )
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
    pub category_id: Uuid,
    pub quantity: i32,
    pub condition_id: Uuid,
    pub location_id: Option<Uuid>,
    pub photo_url: Option<String>,
    pub source_id: Uuid,
    pub donor_id: Option<Uuid>,
    pub procurement_id: Option<Uuid>,
    pub status_id: Uuid,
    pub created_at: DateTime<Utc>,
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
    pub category_id: Uuid,
    pub quantity: Option<i32>,
    pub condition_id: Uuid,
    pub location_id: Option<Uuid>,
    pub photo_url: Option<String>,
    pub source_id: Uuid,
    pub donor_id: Option<Uuid>,
    pub procurement_id: Option<Uuid>,
    pub status_id: Option<Uuid>,
}

#[post("")]
pub async fn create_item(claims: Claims, pool: web::Data<PgPool>, form: web::Json<NewItem>) -> impl Responder {
    println!("DEBUG payload: {:?}", form);

    let id = uuid::Uuid::new_v4();
    
    // Get default status if not provided (assuming 'available' is the default status)
    let status_id = match form.status_id {
        Some(id) => id,
        None => {
            // Try to get the default status
            let default_status = sqlx::query!("SELECT id FROM item_statuses WHERE name = 'active' LIMIT 1")
                .fetch_optional(pool.get_ref())
                .await;
            
            match default_status {
                Ok(Some(status)) => status.id,
                _ => {
                    // If the item_statuses table doesn't exist yet or no 'available' status,
                    // create a default UUID to use temporarily
                    // This will be replaced when the migration is applied
                    uuid::Uuid::new_v4()
                }
            }
        }
    };
    
    let q = sqlx::query_as::<_, Item>(
        "INSERT INTO items (id, name, category_id, quantity, condition_id, location_id, photo_url, source_id, donor_id, procurement_id, status_id, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) RETURNING *"
    )
    .bind(id)
    .bind(&form.name)
    .bind(&form.category_id)
    .bind(form.quantity.unwrap_or(1))
    .bind(&form.condition_id)
    .bind(form.location_id)
    .bind(&form.photo_url)
    .bind(&form.source_id)
    .bind(form.donor_id)
    .bind(form.procurement_id)
    .bind(status_id)
    .bind(chrono::Utc::now())
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
    pub category_id: Option<Uuid>,
    pub quantity: Option<i32>,
    pub condition_id: Option<Uuid>,
    pub location_id: Option<Uuid>,
    pub photo_url: Option<String>,
    pub source_id: Option<Uuid>,
    pub donor_id: Option<Uuid>,
    pub procurement_id: Option<Uuid>,
    pub status_id: Option<Uuid>,
}

#[patch("/{id}")]
pub async fn update_item(claims: Claims, pool: web::Data<PgPool>, path: web::Path<Uuid>, form: web::Json<UpdateItem>) -> impl Responder {
    let id = path.into_inner();
    // Ambil data sebelum update dengan query eksplisit
    let before = match sqlx::query_as::<_, Item>(
        "SELECT id, name, category_id, quantity, condition_id, location_id, photo_url, source_id, donor_id, procurement_id, status_id, created_at FROM items WHERE id = $1"
    )
        .bind(id)
        .fetch_optional(pool.get_ref())
        .await {
            Ok(Some(item)) => {
                println!("[DEBUG] Item sebelum update: ID: {}, photo_url: {:?}", item.id, item.photo_url);
                Some(item)
            },
            Ok(None) => return HttpResponse::NotFound().json(serde_json::json!({"error": "Item not found"})),
            Err(e) => return HttpResponse::InternalServerError().json(serde_json::json!({"error": format!("Failed to fetch item: {}", e)}))
        };
    let q = sqlx::query_as::<_, Item>("UPDATE items SET name = COALESCE($1, name), category_id = COALESCE($2, category_id), quantity = COALESCE($3, quantity), condition_id = COALESCE($4, condition_id), location_id = $5, photo_url = $6, source_id = COALESCE($7, source_id), donor_id = $8, procurement_id = $9, status_id = COALESCE($10, status_id) WHERE id = $11 RETURNING *")
    .bind(form.name.clone())
    .bind(form.category_id)
    .bind(form.quantity)
    .bind(&form.condition_id)
    .bind(form.location_id)
    .bind(&form.photo_url)
    .bind(&form.source_id)
    .bind(form.donor_id)
    .bind(form.procurement_id)
    .bind(form.status_id)
    .bind(id)
    .fetch_optional(pool.get_ref())
    .await;
    match q {
        Ok(Some(item)) => {
            // Debug: Cetak nilai after untuk debugging
            println!("[DEBUG] Item setelah update: ID: {}, photo_url: {:?}", item.id, item.photo_url);
            
            let before_json = before.map(|b| serde_json::to_value(&b).unwrap());
            let after_json = serde_json::to_value(&item).unwrap();
            
            println!("[DEBUG] Before JSON: {:?}", before_json);
            println!("[DEBUG] After JSON: {:?}", after_json);
            
            // Insert log
            let log_result = sqlx::query("INSERT INTO item_logs (item_id, action, before, after, by) VALUES ($1, $2, $3, $4, $5)")
                .bind(item.id)
                .bind("update")
                .bind(before_json)
                .bind(Some(after_json))
                .bind(uuid::Uuid::parse_str(&claims.sub).ok())
                .execute(pool.get_ref()).await;
                
            if let Err(e) = log_result {
                println!("[ERROR] Failed to create log: {}", e);
            }
            
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
        Ok(Some(_row)) => {
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

#[get("/{id}/qrcode")]
pub async fn get_item_qrcode(
    path: web::Path<String>,
    // req: actix_web::HttpRequest
) -> impl Responder {
    let id_str = path.into_inner();
    // URL detail item, ambil dari env FRONTEND_URL
    let frontend_url = std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:5173".to_string());
    let url = format!("{}/items/{}", frontend_url.trim_end_matches('/'), id_str);
    match QrCode::new(url) {
        Ok(code) => {
            let image = code.render::<Luma<u8>>().build();
            let mut cursor = Cursor::new(Vec::new());
            let encoder = image::codecs::png::PngEncoder::new(&mut cursor);
            let encode_result = encoder.write_image(
                image.as_bytes(),
                image.width(),
                image.height(),
                image::ColorType::L8.into()
            );
            match encode_result {
                Ok(_) => {
                    let bytes = cursor.into_inner();
                    HttpResponse::Ok()
                        .content_type(ContentType::png())
                        .body(bytes)
                },
                Err(e) => HttpResponse::InternalServerError().body(format!("QR encode error: {}", e)),
            }
        },
        Err(e) => HttpResponse::InternalServerError().body(format!("QR gen error: {}", e)),
    }
}

pub fn items_config(cfg: &mut web::ServiceConfig) {
    cfg
        .service(get_items)
        .service(get_all_item_logs)
        .service(get_item_logs)
        .service(get_item_by_id)
        .service(create_item)
        .service(update_item)
        .service(delete_item)
        .service(get_item_qrcode);
}

use actix_web::{get, post, patch, delete, web, HttpResponse, Responder};

use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::middleware::admin_guard::is_admin;


// ----------------- Categories -----------------
#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct Category {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
}

#[get("")]
pub async fn get_categories(_claims: crate::middleware::jwt_extractor::Claims, pool: web::Data<PgPool>) -> impl Responder {
    let rows = sqlx::query_as::<_, Category>("SELECT id, name, description FROM categories ORDER BY name")
        .fetch_all(pool.get_ref())
        .await;
    match rows {
        Ok(rows) => HttpResponse::Ok().json(rows),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

#[derive(Deserialize)]
pub struct CategoryPayload {
    pub name: String,
    pub description: Option<String>,
}

#[post("")]
pub async fn create_category(claims: crate::middleware::jwt_extractor::Claims, pool: web::Data<PgPool>, form: web::Json<CategoryPayload>) -> impl Responder {
    if !is_admin(&claims, pool.get_ref()).await {
        return HttpResponse::Forbidden().json(serde_json::json!({ "message": "Admin only" }));
    }
    let row = sqlx::query_as::<_, Category>("INSERT INTO categories (name, description) VALUES ($1, $2) RETURNING id, name, description")
        .bind(&form.name)
        .bind(&form.description)
        .fetch_one(pool.get_ref())
        .await;
    match row {
        Ok(row) => HttpResponse::Ok().json(row),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

#[patch("/{id}")]
pub async fn update_category(claims: crate::middleware::jwt_extractor::Claims, pool: web::Data<PgPool>, path: web::Path<Uuid>, form: web::Json<CategoryPayload>) -> impl Responder {
    if !is_admin(&claims, pool.get_ref()).await {
        return HttpResponse::Forbidden().json(serde_json::json!({ "message": "Admin only" }));
    }
    let id = path.into_inner();
    let row = sqlx::query_as::<_, Category>("UPDATE categories SET name = $1, description = $2 WHERE id = $3 RETURNING id, name, description")
        .bind(&form.name)
        .bind(&form.description)
        .bind(id)
        .fetch_one(pool.get_ref())
        .await;
    match row {
        Ok(row) => HttpResponse::Ok().json(row),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

#[delete("/{id}")]
pub async fn delete_category(claims: crate::middleware::jwt_extractor::Claims, pool: web::Data<PgPool>, path: web::Path<Uuid>) -> impl Responder {
    if !is_admin(&claims, pool.get_ref()).await {
        return HttpResponse::Forbidden().json(serde_json::json!({ "message": "Admin only" }));
    }
    let id = path.into_inner();
    let row = sqlx::query("DELETE FROM categories WHERE id = $1 RETURNING id")
        .bind(id)
        .fetch_optional(pool.get_ref())
        .await;
    match row {
        Ok(Some(_)) => HttpResponse::Ok().json(serde_json::json!({"success": true})),
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({"error": "Not found"})),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

pub fn categories_config(cfg: &mut web::ServiceConfig) {
    cfg.service(get_categories)
        .service(create_category)
        .service(update_category)
        .service(delete_category);
}

// ----------------- ItemSources -----------------
#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct ItemSource {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
}

#[get("")]
pub async fn get_item_sources(_claims: crate::middleware::jwt_extractor::Claims, pool: web::Data<PgPool>) -> impl Responder {
    let rows = sqlx::query_as::<_, ItemSource>("SELECT id, name, description FROM item_sources ORDER BY name")
        .fetch_all(pool.get_ref())
        .await;
    match rows {
        Ok(rows) => HttpResponse::Ok().json(rows),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

#[derive(Deserialize)]
pub struct ItemSourcePayload {
    pub name: String,
    pub description: Option<String>,
}

#[post("")]
pub async fn create_item_source(claims: crate::middleware::jwt_extractor::Claims, pool: web::Data<PgPool>, form: web::Json<ItemSourcePayload>) -> impl Responder {
    if !is_admin(&claims, pool.get_ref()).await {
        return HttpResponse::Forbidden().json(serde_json::json!({ "message": "Admin only" }));
    }
    let row = sqlx::query_as::<_, ItemSource>("INSERT INTO item_sources (name, description) VALUES ($1, $2) RETURNING id, name, description")
        .bind(&form.name)
        .bind(&form.description)
        .fetch_one(pool.get_ref())
        .await;
    match row {
        Ok(row) => HttpResponse::Ok().json(row),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

#[patch("/{id}")]
pub async fn update_item_source(claims: crate::middleware::jwt_extractor::Claims, pool: web::Data<PgPool>, path: web::Path<Uuid>, form: web::Json<ItemSourcePayload>) -> impl Responder {
    if !is_admin(&claims, pool.get_ref()).await {
        return HttpResponse::Forbidden().json(serde_json::json!({ "message": "Admin only" }));
    }
    let id = path.into_inner();
    let row = sqlx::query_as::<_, ItemSource>("UPDATE item_sources SET name = $1, description = $2 WHERE id = $3 RETURNING id, name, description")
        .bind(&form.name)
        .bind(&form.description)
        .bind(id)
        .fetch_one(pool.get_ref())
        .await;
    match row {
        Ok(row) => HttpResponse::Ok().json(row),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

#[delete("/{id}")]
pub async fn delete_item_source(claims: crate::middleware::jwt_extractor::Claims, pool: web::Data<PgPool>, path: web::Path<Uuid>) -> impl Responder {
    if !is_admin(&claims, pool.get_ref()).await {
        return HttpResponse::Forbidden().json(serde_json::json!({ "message": "Admin only" }));
    }
    let id = path.into_inner();
    let row = sqlx::query("DELETE FROM item_sources WHERE id = $1 RETURNING id")
        .bind(id)
        .fetch_optional(pool.get_ref())
        .await;
    match row {
        Ok(Some(_)) => HttpResponse::Ok().json(serde_json::json!({"success": true})),
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({"error": "Not found"})),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

pub fn item_sources_config(cfg: &mut web::ServiceConfig) {
    cfg.service(get_item_sources)
        .service(create_item_source)
        .service(update_item_source)
        .service(delete_item_source);
}

// ----------------- Conditions -----------------
#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct Condition {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
}

#[get("")]
pub async fn get_conditions(_claims: crate::middleware::jwt_extractor::Claims, pool: web::Data<PgPool>) -> impl Responder {
    let rows = sqlx::query_as::<_, Condition>("SELECT id, name, description FROM conditions ORDER BY name")
        .fetch_all(pool.get_ref())
        .await;
    match rows {
        Ok(rows) => HttpResponse::Ok().json(rows),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

#[derive(Deserialize)]
pub struct ConditionPayload {
    pub name: String,
    pub description: Option<String>,
}

#[post("")]
pub async fn create_condition(claims: crate::middleware::jwt_extractor::Claims, pool: web::Data<PgPool>, form: web::Json<ConditionPayload>) -> impl Responder {
    if !is_admin(&claims, pool.get_ref()).await {
        return HttpResponse::Forbidden().json(serde_json::json!({ "message": "Admin only" }));
    }
    let row = sqlx::query_as::<_, Condition>("INSERT INTO conditions (name, description) VALUES ($1, $2) RETURNING id, name, description")
        .bind(&form.name)
        .bind(&form.description)
        .fetch_one(pool.get_ref())
        .await;
    match row {
        Ok(row) => HttpResponse::Ok().json(row),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

#[patch("/{id}")]
pub async fn update_condition(claims: crate::middleware::jwt_extractor::Claims, pool: web::Data<PgPool>, path: web::Path<Uuid>, form: web::Json<ConditionPayload>) -> impl Responder {
    if !is_admin(&claims, pool.get_ref()).await {
        return HttpResponse::Forbidden().json(serde_json::json!({ "message": "Admin only" }));
    }
    let id = path.into_inner();
    let row = sqlx::query_as::<_, Condition>("UPDATE conditions SET name = $1, description = $2 WHERE id = $3 RETURNING id, name, description")
        .bind(&form.name)
        .bind(&form.description)
        .bind(id)
        .fetch_one(pool.get_ref())
        .await;
    match row {
        Ok(row) => HttpResponse::Ok().json(row),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

#[delete("/{id}")]
pub async fn delete_condition(claims: crate::middleware::jwt_extractor::Claims, pool: web::Data<PgPool>, path: web::Path<Uuid>) -> impl Responder {
    if !is_admin(&claims, pool.get_ref()).await {
        return HttpResponse::Forbidden().json(serde_json::json!({ "message": "Admin only" }));
    }
    let id = path.into_inner();
    let row = sqlx::query("DELETE FROM conditions WHERE id = $1 RETURNING id")
        .bind(id)
        .fetch_optional(pool.get_ref())
        .await;
    match row {
        Ok(Some(_)) => HttpResponse::Ok().json(serde_json::json!({"success": true})),
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({"error": "Not found"})),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

pub fn conditions_config(cfg: &mut web::ServiceConfig) {
    cfg.service(get_conditions)
        .service(create_condition)
        .service(update_condition)
        .service(delete_condition);
}

// ----------------- ProcurementStatuses -----------------
#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct ProcurementStatus {
    pub id: Uuid,
    pub name: String,
}

#[get("")]
pub async fn get_procurement_statuses(_claims: crate::middleware::jwt_extractor::Claims, pool: web::Data<PgPool>) -> impl Responder {
    let rows = sqlx::query_as::<_, ProcurementStatus>("SELECT id, name FROM procurement_statuses ORDER BY name")
        .fetch_all(pool.get_ref())
        .await;
    match rows {
        Ok(rows) => HttpResponse::Ok().json(rows),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

#[post("")]
pub async fn create_procurement_status(claims: crate::middleware::jwt_extractor::Claims, pool: web::Data<PgPool>, form: web::Json<ProcurementStatus>) -> impl Responder {
    if !is_admin(&claims, pool.get_ref()).await {
        return HttpResponse::Forbidden().json(serde_json::json!({ "message": "Admin only" }));
    }
    let row = sqlx::query_as::<_, ProcurementStatus>("INSERT INTO procurement_statuses (name) VALUES ($1) RETURNING id, name")
        .bind(&form.name)
        .fetch_one(pool.get_ref())
        .await;
    match row {
        Ok(row) => HttpResponse::Ok().json(row),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

#[patch("/{id}")]
pub async fn update_procurement_status(claims: crate::middleware::jwt_extractor::Claims, pool: web::Data<PgPool>, path: web::Path<Uuid>, form: web::Json<ProcurementStatus>) -> impl Responder {
    if !is_admin(&claims, pool.get_ref()).await {
        return HttpResponse::Forbidden().json(serde_json::json!({ "message": "Admin only" }));
    }
    let id = path.into_inner();
    let row = sqlx::query_as::<_, ProcurementStatus>("UPDATE procurement_statuses SET name = $1 WHERE id = $2 RETURNING id, name")
        .bind(&form.name)
        .bind(id)
        .fetch_one(pool.get_ref())
        .await;
    match row {
        Ok(row) => HttpResponse::Ok().json(row),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

#[delete("/{id}")]
pub async fn delete_procurement_status(claims: crate::middleware::jwt_extractor::Claims, pool: web::Data<PgPool>, path: web::Path<Uuid>) -> impl Responder {
    if !is_admin(&claims, pool.get_ref()).await {
        return HttpResponse::Forbidden().json(serde_json::json!({ "message": "Admin only" }));
    }
    let id = path.into_inner();
    let row = sqlx::query("DELETE FROM procurement_statuses WHERE id = $1 RETURNING id")
        .bind(id)
        .fetch_optional(pool.get_ref())
        .await;
    match row {
        Ok(Some(_)) => HttpResponse::Ok().json(serde_json::json!({"success": true})),
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({"error": "Not found"})),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

pub fn procurement_statuses_config(cfg: &mut web::ServiceConfig) {
    cfg.service(get_procurement_statuses)
        .service(create_procurement_status)
        .service(update_procurement_status)
        .service(delete_procurement_status);
}

// ----------------- UserRoles -----------------
#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct UserRole {
    pub id: Uuid,
    pub name: String,
}

#[get("")]
pub async fn get_user_roles(_claims: crate::middleware::jwt_extractor::Claims, pool: web::Data<PgPool>) -> impl Responder {
    let rows = sqlx::query_as::<_, UserRole>("SELECT id, name FROM user_roles ORDER BY name")
        .fetch_all(pool.get_ref())
        .await;
    match rows {
        Ok(rows) => HttpResponse::Ok().json(rows),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

#[post("")]
pub async fn create_user_role(claims: crate::middleware::jwt_extractor::Claims, pool: web::Data<PgPool>, form: web::Json<UserRole>) -> impl Responder {
    if !is_admin(&claims, pool.get_ref()).await {
        return HttpResponse::Forbidden().json(serde_json::json!({ "message": "Admin only" }));
    }
    let row = sqlx::query_as::<_, UserRole>("INSERT INTO user_roles (name) VALUES ($1) RETURNING id, name")
        .bind(&form.name)
        .fetch_one(pool.get_ref())
        .await;
    match row {
        Ok(row) => HttpResponse::Ok().json(row),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

#[patch("/{id}")]
pub async fn update_user_role(claims: crate::middleware::jwt_extractor::Claims, pool: web::Data<PgPool>, path: web::Path<Uuid>, form: web::Json<UserRole>) -> impl Responder {
    if !is_admin(&claims, pool.get_ref()).await {
        return HttpResponse::Forbidden().json(serde_json::json!({ "message": "Admin only" }));
    }
    let id = path.into_inner();
    let row = sqlx::query_as::<_, UserRole>("UPDATE user_roles SET name = $1 WHERE id = $2 RETURNING id, name")
        .bind(&form.name)
        .bind(id)
        .fetch_one(pool.get_ref())
        .await;
    match row {
        Ok(row) => HttpResponse::Ok().json(row),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

#[delete("/{id}")]
pub async fn delete_user_role(claims: crate::middleware::jwt_extractor::Claims, pool: web::Data<PgPool>, path: web::Path<Uuid>) -> impl Responder {
    if !is_admin(&claims, pool.get_ref()).await {
        return HttpResponse::Forbidden().json(serde_json::json!({ "message": "Admin only" }));
    }
    let id = path.into_inner();
    let row = sqlx::query("DELETE FROM user_roles WHERE id = $1 RETURNING id")
        .bind(id)
        .fetch_optional(pool.get_ref())
        .await;
    match row {
        Ok(Some(_)) => HttpResponse::Ok().json(serde_json::json!({"success": true})),
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({"error": "Not found"})),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

pub fn user_roles_config(cfg: &mut web::ServiceConfig) {
    cfg.service(get_user_roles)
        .service(create_user_role)
        .service(update_user_role)
        .service(delete_user_role);
}

// ----------------- Locations -----------------
#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct Location {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub struct LocationPayload {
    pub name: String,
    pub description: Option<String>,
}

#[get("")]
pub async fn get_locations(_claims: crate::middleware::jwt_extractor::Claims, pool: web::Data<PgPool>) -> impl Responder {
    let rows = sqlx::query_as::<_, Location>("SELECT id, name, description FROM locations ORDER BY name")
        .fetch_all(pool.get_ref())
        .await;
    match rows {
        Ok(rows) => HttpResponse::Ok().json(rows),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

#[post("")]
pub async fn create_location(claims: crate::middleware::jwt_extractor::Claims, pool: web::Data<PgPool>, form: web::Json<LocationPayload>) -> impl Responder {
    if !is_admin(&claims, pool.get_ref()).await {
        return HttpResponse::Forbidden().json(serde_json::json!({ "message": "Admin only" }));
    }
    let row = sqlx::query_as::<_, Location>("INSERT INTO locations (name, description) VALUES ($1, $2) RETURNING id, name, description")
        .bind(&form.name)
        .bind(&form.description)
        .fetch_one(pool.get_ref())
        .await;
    match row {
        Ok(row) => HttpResponse::Ok().json(row),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

#[patch("/{id}")]
pub async fn update_location(claims: crate::middleware::jwt_extractor::Claims, pool: web::Data<PgPool>, path: web::Path<Uuid>, form: web::Json<LocationPayload>) -> impl Responder {
    if !is_admin(&claims, pool.get_ref()).await {
        return HttpResponse::Forbidden().json(serde_json::json!({ "message": "Admin only" }));
    }
    let id = path.into_inner();
    let row = sqlx::query_as::<_, Location>("UPDATE locations SET name = $1, description = $2 WHERE id = $3 RETURNING id, name, description")
        .bind(&form.name)
        .bind(&form.description)
        .bind(id)
        .fetch_one(pool.get_ref())
        .await;
    match row {
        Ok(row) => HttpResponse::Ok().json(row),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

#[delete("/{id}")]
pub async fn delete_location(claims: crate::middleware::jwt_extractor::Claims, pool: web::Data<PgPool>, path: web::Path<Uuid>) -> impl Responder {
    if !is_admin(&claims, pool.get_ref()).await {
        return HttpResponse::Forbidden().json(serde_json::json!({ "message": "Admin only" }));
    }
    let id = path.into_inner();
    let row = sqlx::query("DELETE FROM locations WHERE id = $1 RETURNING id")
        .bind(id)
        .fetch_optional(pool.get_ref())
        .await;
    match row {
        Ok(Some(_)) => HttpResponse::Ok().json(serde_json::json!({"success": true})),
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({"error": "Not found"})),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

pub fn locations_config(cfg: &mut web::ServiceConfig) {
    cfg.service(get_locations)
        .service(create_location)
        .service(update_location)
        .service(delete_location);
}

// ----------------- ItemStatuses -----------------
#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct ItemStatus {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
}

#[derive(Deserialize)]
pub struct ItemStatusPayload {
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
}

#[get("")]
pub async fn get_item_statuses(_claims: crate::middleware::jwt_extractor::Claims, pool: web::Data<PgPool>) -> impl Responder {
    let rows = sqlx::query_as::<_, ItemStatus>("SELECT id, name, description, color FROM item_statuses ORDER BY name")
        .fetch_all(pool.get_ref())
        .await;
    match rows {
        Ok(rows) => HttpResponse::Ok().json(rows),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

#[post("")]
pub async fn create_item_status(claims: crate::middleware::jwt_extractor::Claims, pool: web::Data<PgPool>, form: web::Json<ItemStatusPayload>) -> impl Responder {
    if !is_admin(&claims, pool.get_ref()).await {
        return HttpResponse::Forbidden().json(serde_json::json!({ "message": "Admin only" }));
    }
    let row = sqlx::query_as::<_, ItemStatus>("INSERT INTO item_statuses (name, description, color) VALUES ($1, $2, $3) RETURNING id, name, description, color")
        .bind(&form.name)
        .bind(&form.description)
        .bind(&form.color)
        .fetch_one(pool.get_ref())
        .await;
    match row {
        Ok(row) => HttpResponse::Ok().json(row),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

#[patch("/{id}")]
pub async fn update_item_status(claims: crate::middleware::jwt_extractor::Claims, pool: web::Data<PgPool>, path: web::Path<Uuid>, form: web::Json<ItemStatusPayload>) -> impl Responder {
    if !is_admin(&claims, pool.get_ref()).await {
        return HttpResponse::Forbidden().json(serde_json::json!({ "message": "Admin only" }));
    }
    let id = path.into_inner();
    let row = sqlx::query_as::<_, ItemStatus>("UPDATE item_statuses SET name = $1, description = $2, color = $3 WHERE id = $4 RETURNING id, name, description, color")
        .bind(&form.name)
        .bind(&form.description)
        .bind(&form.color)
        .bind(id)
        .fetch_one(pool.get_ref())
        .await;
    match row {
        Ok(row) => HttpResponse::Ok().json(row),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

#[delete("/{id}")]
pub async fn delete_item_status(claims: crate::middleware::jwt_extractor::Claims, pool: web::Data<PgPool>, path: web::Path<Uuid>) -> impl Responder {
    if !is_admin(&claims, pool.get_ref()).await {
        return HttpResponse::Forbidden().json(serde_json::json!({ "message": "Admin only" }));
    }
    let id = path.into_inner();
    let row = sqlx::query("DELETE FROM item_statuses WHERE id = $1 RETURNING id")
        .bind(id)
        .fetch_optional(pool.get_ref())
        .await;
    match row {
        Ok(Some(_)) => HttpResponse::Ok().json(serde_json::json!({"success": true})),
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({"error": "Not found"})),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

pub fn item_statuses_config(cfg: &mut web::ServiceConfig) {
    cfg.service(get_item_statuses)
        .service(create_item_status)
        .service(update_item_status)
        .service(delete_item_status);
}

// -------- Register all configs --------
pub fn lookup_config(cfg: &mut web::ServiceConfig) {
    use actix_web::web::scope;
    cfg.service(scope("/categories").configure(categories_config));
    cfg.service(scope("/item_sources").configure(item_sources_config));
    cfg.service(scope("/locations").configure(locations_config));
    cfg.service(scope("/conditions").configure(conditions_config));
    cfg.service(scope("/procurement_statuses").configure(procurement_statuses_config));
    cfg.service(scope("/user_roles").configure(user_roles_config));
    cfg.service(scope("/item_statuses").configure(item_statuses_config));
}



use actix_web::{get, post, patch, delete, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::middleware::jwt_extractor::Claims;
use crate::middleware::permission_guard::has_permission;

// ----------------- Permissions -----------------
#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct Permission {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub struct PermissionPayload {
    pub name: String,
    pub description: Option<String>,
}

#[get("")]
pub async fn get_permissions(_claims: Claims, pool: web::Data<PgPool>) -> impl Responder {
    let rows = sqlx::query_as::<_, Permission>("SELECT id, name, description FROM permissions ORDER BY name")
        .fetch_all(pool.get_ref())
        .await;
    match rows {
        Ok(rows) => HttpResponse::Ok().json(rows),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

#[post("")]
pub async fn create_permission(claims: Claims, pool: web::Data<PgPool>, form: web::Json<PermissionPayload>) -> impl Responder {
    // Only users with manage_permissions permission can create permissions
    if !has_permission(&claims, pool.get_ref(), "manage_permissions").await {
        return HttpResponse::Forbidden().json(serde_json::json!({ "message": "Insufficient permissions" }));
    }

    let row = sqlx::query_as::<_, Permission>("INSERT INTO permissions (name, description) VALUES ($1, $2) RETURNING id, name, description")
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
pub async fn update_permission(claims: Claims, pool: web::Data<PgPool>, path: web::Path<Uuid>, form: web::Json<PermissionPayload>) -> impl Responder {
    // Only users with manage_permissions permission can update permissions
    if !has_permission(&claims, pool.get_ref(), "manage_permissions").await {
        return HttpResponse::Forbidden().json(serde_json::json!({ "message": "Insufficient permissions" }));
    }

    let id = path.into_inner();
    let row = sqlx::query_as::<_, Permission>("UPDATE permissions SET name = $1, description = $2 WHERE id = $3 RETURNING id, name, description")
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
pub async fn delete_permission(claims: Claims, pool: web::Data<PgPool>, path: web::Path<Uuid>) -> impl Responder {
    // Only users with manage_permissions permission can delete permissions
    if !has_permission(&claims, pool.get_ref(), "manage_permissions").await {
        return HttpResponse::Forbidden().json(serde_json::json!({ "message": "Insufficient permissions" }));
    }

    let id = path.into_inner();
    let row = sqlx::query("DELETE FROM permissions WHERE id = $1 RETURNING id")
        .bind(id)
        .fetch_optional(pool.get_ref())
        .await;
    match row {
        Ok(Some(_)) => HttpResponse::Ok().json(serde_json::json!({"success": true})),
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({"error": "Not found"})),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

// ----------------- Role Permissions -----------------
#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct RolePermission {
    pub id: Uuid,
    pub role_id: Uuid,
    pub permission_id: Uuid,
}

#[derive(Deserialize)]
pub struct RolePermissionPayload {
    pub role_id: Uuid,
    pub permission_id: Uuid,
}

#[get("/role/{role_id}")]
pub async fn get_role_permissions(_claims: Claims, pool: web::Data<PgPool>, path: web::Path<Uuid>) -> impl Responder {
    let role_id = path.into_inner();
    let rows = sqlx::query_as::<_, Permission>(
        "SELECT p.id, p.name, p.description 
         FROM permissions p
         JOIN role_permissions rp ON p.id = rp.permission_id
         WHERE rp.role_id = $1
         ORDER BY p.name"
    )
    .bind(role_id)
    .fetch_all(pool.get_ref())
    .await;
    match rows {
        Ok(rows) => HttpResponse::Ok().json(rows),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

#[post("/role")]
pub async fn assign_permission_to_role(claims: Claims, pool: web::Data<PgPool>, form: web::Json<RolePermissionPayload>) -> impl Responder {
    // Only users with manage_roles permission can assign permissions to roles
    if !has_permission(&claims, pool.get_ref(), "manage_roles").await {
        return HttpResponse::Forbidden().json(serde_json::json!({ "message": "Insufficient permissions" }));
    }

    // Check if the role-permission mapping already exists
    let existing = sqlx::query("SELECT 1 FROM role_permissions WHERE role_id = $1 AND permission_id = $2")
        .bind(form.role_id)
        .bind(form.permission_id)
        .fetch_optional(pool.get_ref())
        .await;
    
    match existing {
        Ok(Some(_)) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "This permission is already assigned to the role"
            }));
        },
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": e.to_string()
            }));
        },
        _ => {}
    }

    let row = sqlx::query_as::<_, RolePermission>(
        "INSERT INTO role_permissions (role_id, permission_id) VALUES ($1, $2) RETURNING id, role_id, permission_id"
    )
    .bind(form.role_id)
    .bind(form.permission_id)
    .fetch_one(pool.get_ref())
    .await;
    
    match row {
        Ok(row) => HttpResponse::Ok().json(row),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

#[delete("/role/{role_id}/permission/{permission_id}")]
pub async fn remove_permission_from_role(claims: Claims, pool: web::Data<PgPool>, path: web::Path<(Uuid, Uuid)>) -> impl Responder {
    // Only users with manage_roles permission can remove permissions from roles
    if !has_permission(&claims, pool.get_ref(), "manage_roles").await {
        return HttpResponse::Forbidden().json(serde_json::json!({ "message": "Insufficient permissions" }));
    }

    let (role_id, permission_id) = path.into_inner();
    let row = sqlx::query("DELETE FROM role_permissions WHERE role_id = $1 AND permission_id = $2 RETURNING id")
        .bind(role_id)
        .bind(permission_id)
        .fetch_optional(pool.get_ref())
        .await;
    
    match row {
        Ok(Some(_)) => HttpResponse::Ok().json(serde_json::json!({"success": true})),
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({"error": "Role-permission mapping not found"})),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

pub fn permissions_config(cfg: &mut web::ServiceConfig) {
    cfg.service(get_permissions)
        .service(create_permission)
        .service(update_permission)
        .service(delete_permission)
        .service(get_role_permissions)
        .service(assign_permission_to_role)
        .service(remove_permission_from_role);
}

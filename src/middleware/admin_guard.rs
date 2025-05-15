use crate::middleware::jwt_extractor::Claims;
use crate::middleware::permission_guard::has_permission;
use sqlx::PgPool;
use sqlx::Row;

/// Check if a user has admin role
pub async fn is_admin(claims: &Claims, pool: &PgPool) -> bool {
    // First try to check using the new permissions system
    if has_permission(claims, pool, "admin_access").await {
        return true;
    }
    
    // Fallback to the old role-based check for backward compatibility
    let user_id = match uuid::Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => {
            return false;
        }
    };
    let user_row = sqlx::query("SELECT role_id FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await;
    let user_role_id = match user_row {
        Ok(Some(row)) => row.get::<uuid::Uuid, _>("role_id"),
        _ => {
            return false;
        }
    };
    let admin_role_row = sqlx::query("SELECT id FROM user_roles WHERE name = 'admin'")
        .fetch_optional(pool)
        .await;
    let admin_role_id = match admin_role_row {
        Ok(Some(row)) => row.get::<uuid::Uuid, _>("id"),
        _ => {
            return false;
        }
    };
    user_role_id == admin_role_id
}

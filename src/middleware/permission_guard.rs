use crate::middleware::jwt_extractor::Claims;
use sqlx::PgPool;
use sqlx::Row;
use uuid::Uuid;

/// Check if a user has a specific permission
pub async fn has_permission(claims: &Claims, pool: &PgPool, permission_name: &str) -> bool {
    // Parse user ID from claims
    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => {
            return false;
        }
    };

    // Get user's role ID
    let user_row = sqlx::query("SELECT role_id FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await;
    
    let user_role_id = match user_row {
        Ok(Some(row)) => row.get::<Uuid, _>("role_id"),
        _ => {
            return false;
        }
    };

    // Check if the role has the required permission
    let permission_check = sqlx::query(
        "SELECT 1 FROM role_permissions rp 
         JOIN permissions p ON rp.permission_id = p.id 
         WHERE rp.role_id = $1 AND p.name = $2"
    )
    .bind(user_role_id)
    .bind(permission_name)
    .fetch_optional(pool)
    .await;

    match permission_check {
        Ok(Some(_)) => true,
        _ => false,
    }
}

/// Check if a user has any of the specified permissions
pub async fn has_any_permission(claims: &Claims, pool: &PgPool, permission_names: &[&str]) -> bool {
    for permission_name in permission_names {
        if has_permission(claims, pool, permission_name).await {
            return true;
        }
    }
    false
}

/// Check if a user has all of the specified permissions
pub async fn has_all_permissions(claims: &Claims, pool: &PgPool, permission_names: &[&str]) -> bool {
    for permission_name in permission_names {
        if !has_permission(claims, pool, permission_name).await {
            return false;
        }
    }
    true
}

use crate::middleware::jwt_extractor::Claims;
use sqlx::PgPool;
use sqlx::Row;

pub async fn is_admin(claims: &Claims, pool: &PgPool) -> bool {
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
    let result = user_role_id == admin_role_id;
    result
}

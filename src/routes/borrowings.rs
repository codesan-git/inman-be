use actix_web::{get, post, patch, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row, Executor, Postgres};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::middleware::jwt_extractor::Claims;
use crate::middleware::permission_guard::has_permission;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ItemBorrowing {
    pub id: Uuid,
    pub item_id: Uuid,
    pub borrower_id: Uuid,
    pub quantity: i32,
    pub borrowed_at: DateTime<Utc>,
    pub expected_return_date: DateTime<Utc>,
    pub actual_return_date: Option<DateTime<Utc>>,
    pub approved_by: Option<Uuid>,
    pub notes: Option<String>,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ItemBorrowingWithDetails {
    pub id: Uuid,
    pub item_id: Uuid,
    pub item_name: String,
    pub borrower_id: Uuid,
    pub borrower_name: String,
    pub quantity: i32,
    pub borrowed_at: DateTime<Utc>,
    pub expected_return_date: DateTime<Utc>,
    pub actual_return_date: Option<DateTime<Utc>>,
    pub approved_by: Option<Uuid>,
    pub approver_name: Option<String>,
    pub notes: Option<String>,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct NewItemBorrowing {
    pub item_id: Uuid,
    pub quantity: Option<i32>,
    pub expected_return_date: DateTime<Utc>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateItemBorrowing {
    pub quantity: Option<i32>,
    pub expected_return_date: Option<DateTime<Utc>>,
    pub actual_return_date: Option<DateTime<Utc>>,
    pub notes: Option<String>,
    pub status: Option<String>,
}

#[get("")]
pub async fn get_borrowings(claims: Claims, pool: web::Data<PgPool>) -> impl Responder {
    // Check if user has permission to view all borrowings
    let can_view_all = has_permission(&claims, pool.get_ref(), "view_all_borrowings").await;
    
    let query = if can_view_all {
        // Admin/staff can see all borrowings
        "SELECT b.id, b.item_id, i.name as item_name, b.borrower_id, 
                u.name as borrower_name, b.quantity, b.borrowed_at, 
                b.expected_return_date, b.actual_return_date, b.approved_by, 
                a.name as approver_name, b.notes, b.status
         FROM item_borrowings b
         JOIN items i ON b.item_id = i.id
         JOIN users u ON b.borrower_id = u.id
         LEFT JOIN users a ON b.approved_by = a.id
         ORDER BY b.borrowed_at DESC".to_string()
    } else {
        // Regular users can only see their own borrowings
        format!(
            "SELECT b.id, b.item_id, i.name as item_name, b.borrower_id, 
                    u.name as borrower_name, b.quantity, b.borrowed_at, 
                    b.expected_return_date, b.actual_return_date, b.approved_by, 
                    a.name as approver_name, b.notes, b.status
             FROM item_borrowings b
             JOIN items i ON b.item_id = i.id
             JOIN users u ON b.borrower_id = u.id
             LEFT JOIN users a ON b.approved_by = a.id
             WHERE b.borrower_id = '{}'
             ORDER BY b.borrowed_at DESC",
            claims.sub
        )
    };
    
    let borrowings = sqlx::query(&query)
    .map(|row: sqlx::postgres::PgRow| {
        ItemBorrowingWithDetails {
            id: row.get("id"),
            item_id: row.get("item_id"),
            item_name: row.get("item_name"),
            borrower_id: row.get("borrower_id"),
            borrower_name: row.get("borrower_name"),
            quantity: row.get("quantity"),
            borrowed_at: row.get("borrowed_at"),
            expected_return_date: row.get("expected_return_date"),
            actual_return_date: row.get("actual_return_date"),
            approved_by: row.get("approved_by"),
            approver_name: row.get("approver_name"),
            notes: row.get("notes"),
            status: row.get("status"),
        }
    })
    .fetch_all(pool.get_ref())
    .await;
    
    match borrowings {
        Ok(borrowings) => HttpResponse::Ok().json(borrowings),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

#[get("/{id}")]
pub async fn get_borrowing_by_id(claims: Claims, pool: web::Data<PgPool>, path: web::Path<Uuid>) -> impl Responder {
    let id = path.into_inner();
    
    // Check if user has permission to view all borrowings
    let can_view_all = has_permission(&claims, pool.get_ref(), "view_all_borrowings").await;
    
    let query = if can_view_all {
        // Admin/staff can see any borrowing
        format!(
            "SELECT b.id, b.item_id, i.name as item_name, b.borrower_id, 
                    u.name as borrower_name, b.quantity, b.borrowed_at, 
                    b.expected_return_date, b.actual_return_date, b.approved_by, 
                    a.name as approver_name, b.notes, b.status
             FROM item_borrowings b
             JOIN items i ON b.item_id = i.id
             JOIN users u ON b.borrower_id = u.id
             LEFT JOIN users a ON b.approved_by = a.id
             WHERE b.id = '{}'",
            id
        )
    } else {
        // Regular users can only see their own borrowings
        format!(
            "SELECT b.id, b.item_id, i.name as item_name, b.borrower_id, 
                    u.name as borrower_name, b.quantity, b.borrowed_at, 
                    b.expected_return_date, b.actual_return_date, b.approved_by, 
                    a.name as approver_name, b.notes, b.status
             FROM item_borrowings b
             JOIN items i ON b.item_id = i.id
             JOIN users u ON b.borrower_id = u.id
             LEFT JOIN users a ON b.approved_by = a.id
             WHERE b.id = '{}' AND b.borrower_id = '{}'",
            id, claims.sub
        )
    };
    
    let borrowing = sqlx::query(&query)
    .map(|row: sqlx::postgres::PgRow| {
        ItemBorrowingWithDetails {
            id: row.get("id"),
            item_id: row.get("item_id"),
            item_name: row.get("item_name"),
            borrower_id: row.get("borrower_id"),
            borrower_name: row.get("borrower_name"),
            quantity: row.get("quantity"),
            borrowed_at: row.get("borrowed_at"),
            expected_return_date: row.get("expected_return_date"),
            actual_return_date: row.get("actual_return_date"),
            approved_by: row.get("approved_by"),
            approver_name: row.get("approver_name"),
            notes: row.get("notes"),
            status: row.get("status"),
        }
    })
    .fetch_optional(pool.get_ref())
    .await;
    
    match borrowing {
        Ok(Some(borrowing)) => HttpResponse::Ok().json(borrowing),
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({"error": "Borrowing not found"})),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

#[post("")]
pub async fn create_borrowing(claims: Claims, pool: web::Data<PgPool>, form: web::Json<NewItemBorrowing>) -> impl Responder {
    // Check if user has permission to borrow items
    if !has_permission(&claims, pool.get_ref(), "borrow_items").await {
        return HttpResponse::Forbidden().json(serde_json::json!({
            "error": "You don't have permission to borrow items"
        }));
    }
    
    // Check if the item exists and is available
    let item = sqlx::query!(
        "SELECT i.id, i.name, i.quantity, s.name as status_name 
         FROM items i 
         JOIN item_statuses s ON i.status_id = s.id 
         WHERE i.id = $1",
        form.item_id
    )
    .fetch_optional(pool.get_ref())
    .await;
    
    let item = match item {
        Ok(Some(item)) => item,
        Ok(None) => {
            return HttpResponse::NotFound().json(serde_json::json!({
                "error": "Item not found"
            }));
        },
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": e.to_string()
            }));
        }
    };
    
    // Check if item is active
    if item.status_name != "active" {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Item is not available for borrowing"
        }));
    }
    
    // Check if quantity is valid
    let quantity = form.quantity.unwrap_or(1);
    if quantity <= 0 || quantity > item.quantity {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Invalid quantity"
        }));
    }
    
    // Parse user ID from claims
    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Invalid user ID"
            }));
        }
    };
    
    // Create the borrowing record
    let borrowing = sqlx::query_as::<_, ItemBorrowing>(
        "INSERT INTO item_borrowings (item_id, borrower_id, quantity, expected_return_date, notes, status) 
         VALUES ($1, $2, $3, $4, $5, $6) 
         RETURNING *"
    )
    .bind(form.item_id)
    .bind(user_id)
    .bind(quantity)
    .bind(form.expected_return_date)
    .bind(form.notes.clone())
    .bind("pending")
    .fetch_one(pool.get_ref())
    .await;
    
    match borrowing {
        Ok(borrowing) => {
            // Log the borrowing request
            let _ = sqlx::query(
                "INSERT INTO item_logs (item_id, action, note, by) 
                 VALUES ($1, $2, $3, $4)"
            )
            .bind(form.item_id)
            .bind("borrowing_requested")
            .bind(format!("Borrowing requested: {} units, expected return: {}", 
                          quantity, form.expected_return_date))
            .bind(user_id)
            .execute(pool.get_ref())
            .await;
            
            HttpResponse::Ok().json(borrowing)
        },
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

#[patch("/{id}/approve")]
pub async fn approve_borrowing(claims: Claims, pool: web::Data<PgPool>, path: web::Path<Uuid>) -> impl Responder {
    let id = path.into_inner();
    
    // Check if user has permission to approve borrowings
    if !has_permission(&claims, pool.get_ref(), "approve_borrowings").await {
        return HttpResponse::Forbidden().json(serde_json::json!({
            "error": "You don't have permission to approve borrowings"
        }));
    }
    
    // Parse user ID from claims
    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Invalid user ID"
            }));
        }
    };
    
    // Get the borrowing record
    let borrowing = sqlx::query_as::<_, ItemBorrowing>(
        "SELECT * FROM item_borrowings WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(pool.get_ref())
    .await;
    
    let borrowing = match borrowing {
        Ok(Some(b)) => b,
        Ok(None) => {
            return HttpResponse::NotFound().json(serde_json::json!({
                "error": "Borrowing not found"
            }));
        },
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": e.to_string()
            }));
        }
    };
    
    // Check if borrowing is already approved
    if borrowing.status != "pending" {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Borrowing is not in pending status"
        }));
    }
    
    // Start a transaction
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to start transaction: {}", e)
            }));
        }
    };
    
    // Update the borrowing status
    let updated_borrowing = sqlx::query_as::<_, ItemBorrowing>(
        "UPDATE item_borrowings 
         SET status = 'approved', approved_by = $1 
         WHERE id = $2 
         RETURNING *"
    )
    .bind(user_id)
    .bind(id)
    .fetch_one(&mut *tx)
    .await;
    
    let updated_borrowing = match updated_borrowing {
        Ok(b) => b,
        Err(e) => {
            let _ = tx.rollback().await;
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to update borrowing: {}", e)
            }));
        }
    };
    
    // Update the item status to 'borrowed'
    let borrowed_status = sqlx::query!(
        "SELECT id FROM item_statuses WHERE name = 'borrowed'"
    )
    .fetch_one(&mut *tx)
    .await;
    
    let borrowed_status_id = match borrowed_status {
        Ok(status) => status.id,
        Err(e) => {
            let _ = tx.rollback().await;
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to get borrowed status: {}", e)
            }));
        }
    };
    
    // Update the item status
    let update_item = sqlx::query!(
        "UPDATE items SET status_id = $1 WHERE id = $2",
        borrowed_status_id,
        borrowing.item_id
    )
    .execute(&mut *tx)
    .await;
    
    match update_item {
        Ok(_) => {
            // Log the approval
            let log = sqlx::query(
                "INSERT INTO item_logs (item_id, action, note, by) 
                 VALUES ($1, $2, $3, $4)"
            )
            .bind(borrowing.item_id)
            .bind("borrowing_approved")
            .bind(format!("Borrowing approved for {} units", borrowing.quantity))
            .bind(user_id)
            .execute(&mut *tx)
            .await;
            
            match log {
                Ok(_) => {
                    // Commit the transaction
                    match tx.commit().await {
                        Ok(_) => HttpResponse::Ok().json(updated_borrowing),
                        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
                            "error": format!("Failed to commit transaction: {}", e)
                        })),
                    }
                },
                Err(e) => {
                    let _ = tx.rollback().await;
                    HttpResponse::InternalServerError().json(serde_json::json!({
                        "error": format!("Failed to log approval: {}", e)
                    }))
                }
            }
        },
        Err(e) => {
            let _ = tx.rollback().await;
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to update item status: {}", e)
            }))
        }
    }
}

#[patch("/{id}/return")]
pub async fn return_borrowing(claims: Claims, pool: web::Data<PgPool>, path: web::Path<Uuid>) -> impl Responder {
    let id = path.into_inner();
    
    // Parse user ID from claims
    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Invalid user ID"
            }));
        }
    };
    
    // Get the borrowing record
    let borrowing = sqlx::query_as::<_, ItemBorrowing>(
        "SELECT * FROM item_borrowings WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(pool.get_ref())
    .await;
    
    let borrowing = match borrowing {
        Ok(Some(b)) => b,
        Ok(None) => {
            return HttpResponse::NotFound().json(serde_json::json!({
                "error": "Borrowing not found"
            }));
        },
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": e.to_string()
            }));
        }
    };
    
    // Check if borrowing is approved
    if borrowing.status != "approved" {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Borrowing is not in approved status"
        }));
    }
    
    // Check if user is the borrower or has permission to manage borrowings
    let is_borrower = borrowing.borrower_id.to_string() == claims.sub;
    let can_manage = has_permission(&claims, pool.get_ref(), "manage_borrowings").await;
    
    if !is_borrower && !can_manage {
        return HttpResponse::Forbidden().json(serde_json::json!({
            "error": "You don't have permission to return this item"
        }));
    }
    
    // Start a transaction
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to start transaction: {}", e)
            }));
        }
    };
    
    // Update the borrowing status
    let updated_borrowing = sqlx::query_as::<_, ItemBorrowing>(
        "UPDATE item_borrowings 
         SET status = 'returned', actual_return_date = now() 
         WHERE id = $1 
         RETURNING *"
    )
    .bind(id)
    .fetch_one(&mut *tx)
    .await;
    
    let updated_borrowing = match updated_borrowing {
        Ok(b) => b,
        Err(e) => {
            let _ = tx.rollback().await;
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to update borrowing: {}", e)
            }));
        }
    };
    
    // Update the item status back to 'active'
    let active_status = sqlx::query!(
        "SELECT id FROM item_statuses WHERE name = 'active'"
    )
    .fetch_one(&mut *tx)
    .await;
    
    let available_status_id = match active_status {
        Ok(status) => status.id,
        Err(e) => {
            let _ = tx.rollback().await;
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to get active status: {}", e)
            }));
        }
    };
    
    // Update the item status
    let update_item = sqlx::query!(
        "UPDATE items SET status_id = $1 WHERE id = $2",
        available_status_id,
        borrowing.item_id
    )
    .execute(&mut *tx)
    .await;
    
    match update_item {
        Ok(_) => {
            // Log the return
            let log = sqlx::query(
                "INSERT INTO item_logs (item_id, action, note, by) 
                 VALUES ($1, $2, $3, $4)"
            )
            .bind(borrowing.item_id)
            .bind("item_returned")
            .bind(format!("Item returned: {} units", borrowing.quantity))
            .bind(user_id)
            .execute(&mut *tx)
            .await;
            
            match log {
                Ok(_) => {
                    // Commit the transaction
                    match tx.commit().await {
                        Ok(_) => HttpResponse::Ok().json(updated_borrowing),
                        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
                            "error": format!("Failed to commit transaction: {}", e)
                        })),
                    }
                },
                Err(e) => {
                    let _ = tx.rollback().await;
                    HttpResponse::InternalServerError().json(serde_json::json!({
                        "error": format!("Failed to log return: {}", e)
                    }))
                }
            }
        },
        Err(e) => {
            let _ = tx.rollback().await;
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to update item status: {}", e)
            }))
        }
    }
}

pub fn borrowings_config(cfg: &mut web::ServiceConfig) {
    cfg.service(get_borrowings)
        .service(get_borrowing_by_id)
        .service(create_borrowing)
        .service(approve_borrowing)
        .service(return_borrowing);
}

use sqlx::{
    mysql::{MySqlPoolOptions, MySqlQueryResult},
    MySqlPool,
};
use std::time::Duration;
use teloxide::types::User;

pub async fn create_pool_from_env(
) -> Result<MySqlPool, Box<dyn std::error::Error + Send + Sync>> {
    let database_url = std::env::var("DATABASE_URL")?;

    let pool = MySqlPoolOptions::new()
        .acquire_timeout(Duration::from_secs(10))
        .max_connections(10)
        .connect(&database_url)
        .await?;

    init_schema(&pool).await?;

    Ok(pool)
}

async fn init_schema(pool: &MySqlPool) -> Result<MySqlQueryResult, sqlx::Error> {
    let result = sqlx::query(
        r#"
CREATE TABLE IF NOT EXISTS telegram_users (
  telegram_id BIGINT UNSIGNED NOT NULL,
  username VARCHAR(255) NULL,
  first_name VARCHAR(255) NOT NULL,
  last_name VARCHAR(255) NULL,
  language_code VARCHAR(32) NULL,
  is_bot BOOLEAN NOT NULL,
  referred_by_telegram_id BIGINT UNSIGNED NULL,
  amount VARCHAR(64) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  last_seen_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (telegram_id)
) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci
"#,
    )
    .execute(pool)
    .await?;

    ensure_column(
        pool,
        r#"ALTER TABLE telegram_users ADD COLUMN referred_by_telegram_id BIGINT UNSIGNED NULL"#,
    )
    .await?;

    ensure_column(
        pool,
        r#"ALTER TABLE telegram_users ADD COLUMN amount VARCHAR(64) NOT NULL DEFAULT ''"#,
    )
    .await?;

    sqlx::query(r#"UPDATE telegram_users SET amount = '' WHERE amount IS NULL"#)
        .execute(pool)
        .await?;

    sqlx::query(r#"ALTER TABLE telegram_users MODIFY COLUMN amount VARCHAR(64) NOT NULL DEFAULT ''"#)
        .execute(pool)
        .await?;

    Ok(result)
}

async fn ensure_column(pool: &MySqlPool, sql: &str) -> Result<(), sqlx::Error> {
    let result = sqlx::query(sql).execute(pool).await;
    if let Err(err) = result {
        if let sqlx::Error::Database(db_err) = &err {
            let code = db_err.code().map(|c| c.to_string());
            let message = db_err.message();
            if code.as_deref() == Some("1060") || message.contains("Duplicate column name") {
                return Ok(());
            }
        }
        return Err(err);
    }
    Ok(())
}

pub async fn upsert_telegram_user(
    pool: &MySqlPool,
    user: &User,
    referred_by_telegram_id: Option<u64>,
) -> Result<(), sqlx::Error> {
    let telegram_id: u64 = user.id.0;
    let username: Option<&str> = user.username.as_deref();
    let first_name: &str = &user.first_name;
    let last_name: Option<&str> = user.last_name.as_deref();
    let language_code: Option<&str> = user.language_code.as_deref();
    let is_bot: bool = user.is_bot;

    sqlx::query(
        r#"
INSERT INTO telegram_users
  (telegram_id, username, first_name, last_name, language_code, is_bot, referred_by_telegram_id, last_seen_at)
VALUES
  (?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP) AS new
ON DUPLICATE KEY UPDATE
  username = new.username,
  first_name = new.first_name,
  last_name = new.last_name,
  language_code = new.language_code,
  is_bot = new.is_bot,
  referred_by_telegram_id = COALESCE(referred_by_telegram_id, new.referred_by_telegram_id),
  last_seen_at = CURRENT_TIMESTAMP
"#,
    )
    .bind(telegram_id)
    .bind(username)
    .bind(first_name)
    .bind(last_name)
    .bind(language_code)
    .bind(is_bot)
    .bind(referred_by_telegram_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_user_amount(pool: &MySqlPool, telegram_id: u64) -> Result<String, sqlx::Error> {
    let amount: Option<Option<String>> = sqlx::query_scalar::<_, Option<String>>(
        r#"
SELECT amount
FROM telegram_users
WHERE telegram_id = ?
"#,
    )
    .bind(telegram_id)
    .fetch_optional(pool)
    .await?;

    Ok(normalize_amount(amount.flatten().as_deref()))
}

pub async fn list_telegram_ids(pool: &MySqlPool) -> Result<Vec<u64>, sqlx::Error> {
    let ids: Vec<u64> = sqlx::query_scalar(
        r#"
SELECT telegram_id
FROM telegram_users
WHERE is_bot = FALSE
ORDER BY telegram_id
"#,
    )
    .fetch_all(pool)
    .await?;

    Ok(ids)
}

fn normalize_amount(amount: Option<&str>) -> String {
    let s = amount.unwrap_or("0.0").trim();
    if s.is_empty() {
        return "0.0".to_owned();
    }
    match s.parse::<f64>() {
        Ok(v) if v == 0.0 => "0.0".to_owned(),
        Ok(_) => s.to_owned(),
        Err(_) => s.to_owned(),
    }
}

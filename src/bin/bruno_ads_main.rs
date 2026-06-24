use env_logger::Env;
use std::time::Duration;
use teloxide::{prelude::*, types::InlineKeyboardButton, types::InlineKeyboardMarkup};
use telegram_bot::db;

fn support_username() -> String {
    std::env::var("SUPPORT_CONTACT")
        .unwrap_or_else(|_| "JoJotaroKujo".to_owned())
        .trim()
        .trim_start_matches('@')
        .to_owned()
}

fn support_url() -> Option<reqwest::Url> {
    let url = std::env::var("SUPPORT_CONTACT_URL")
        .ok()
        .or_else(|| Some(format!("https://t.me/{}", support_username())))?;
    url.parse().ok()
}

fn ads_text_default() -> String {
    "".to_owned()
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    pretty_env_logger::formatted_builder()
        .parse_env(Env::default().default_filter_or("info"))
        .target(env_logger::Target::Stderr)
        .init();

    let pool = match db::create_pool_from_env().await {
        Ok(pool) => pool,
        Err(err) => {
            log::error!("mysql init failed: {err}");
            return;
        }
    };

    let bot = Bot::from_env();
    let ids = match db::list_telegram_ids(&pool).await {
        Ok(ids) => ids,
        Err(err) => {
            log::error!("load telegram_ids failed: {err}");
            return;
        }
    };

    let text = std::env::var("ADS_TEXT").unwrap_or_else(|_| ads_text_default());
    let sleep_ms: u64 = std::env::var("ADS_SLEEP_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(350);
    let dry_run: bool = std::env::var("ADS_DRY_RUN")
        .ok()
        .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false);
    let limit: Option<usize> = std::env::var("ADS_LIMIT").ok().and_then(|v| v.parse().ok());

    let send_ids = match limit {
        Some(n) => ids.into_iter().take(n).collect::<Vec<_>>(),
        None => ids,
    };

    let markup = support_url().map(|url| {
        InlineKeyboardMarkup::new([[InlineKeyboardButton::url("联系客服".to_owned(), url)]])
    });

    log::info!(
        "ads sender start: total={} dry_run={} sleep_ms={}",
        send_ids.len(),
        dry_run,
        sleep_ms
    );
    if dry_run {
        log::warn!("ADS_DRY_RUN is enabled, no messages will be sent");
    }

    let mut ok = 0usize;
    let mut failed = 0usize;

    for (i, telegram_id) in send_ids.iter().copied().enumerate() {
        if dry_run {
            log::info!("dry_run send to telegram_id={} ({}/{})", telegram_id, i + 1, send_ids.len());
        } else {
            let chat_id = ChatId(telegram_id as i64);
            let mut req = bot.send_message(chat_id, text.clone());
            if let Some(markup) = markup.clone() {
                req = req.reply_markup(markup);
            }

            match req.await {
                Ok(_) => {
                    ok += 1;
                    log::info!("sent ok telegram_id={} ({}/{})", telegram_id, i + 1, send_ids.len());
                }
                Err(err) => {
                    failed += 1;
                    log::warn!("send failed telegram_id={telegram_id}: {err}");
                }
            }
        }

        tokio::time::sleep(Duration::from_millis(sleep_ms)).await;
    }

    log::info!("ads sender done: ok={} failed={}", ok, failed);
}

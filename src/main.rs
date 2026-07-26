use teloxide::{
    prelude::*,
    types::{BotCommand, InlineKeyboardButton, InlineKeyboardMarkup, InputFile, ParseMode, Update},
    utils::command::BotCommands,
};
use env_logger::Env;
use teloxide::dispatching::UpdateFilterExt;
use teloxide::dptree;
use telegram_bot::db;

fn support_username() -> String {
    std::env::var("SUPPORT_CONTACT")
        .unwrap_or_else(|_| "epinastinejojo_bot".to_owned())
        .trim()
        .trim_start_matches('@')
        .to_owned()
}

fn support_url_str() -> Option<String> {
    std::env::var("SUPPORT_CONTACT_URL")
        .ok()
        .or_else(|| Some(format!("https://t.me/{}", support_username())))
}

fn support_chat_id() -> Option<i64> {
    std::env::var("SUPPORT_CHAT_ID")
        .ok()
        .and_then(|value| value.trim().parse::<i64>().ok())
}

fn telegram_bot_token() -> Result<String, std::env::VarError> {
    std::env::var("TELEGRAM_BOT_TOKEN")
}
 
 #[derive(BotCommands, Clone)]
 #[command(rename_rule = "lowercase")]
 enum Command {
    Start(String),
     Help,
    Referral,
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

    let bot = match telegram_bot_token() {
        Ok(token) => Bot::new(token),
        Err(err) => {
            log::error!("telegram bot token init failed: {err}");
            return;
        }
    };
    let commands = vec![
        BotCommand::new("start", "开始"),
        BotCommand::new("help", "联系客服"),
        BotCommand::new("referral", "推广返佣"),
    ];
    if let Err(err) = bot.set_my_commands(commands).await {
        log::error!("set_my_commands failed: {err}");
    }

    let command_handler = dptree::entry()
        .filter_command::<Command>()
        .endpoint(answer);
    let tx_hash_handler = dptree::filter(|msg: Message| {
        msg.text()
            .map(|text| {
                let text = text.trim();
                !text.is_empty() && !text.starts_with('/')
            })
            .unwrap_or(false)
    })
    .endpoint(handle_transaction_hash);
    let handler = Update::filter_message()
        .branch(command_handler)
        .branch(tx_hash_handler);

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![pool])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
 }
 
async fn answer(
    bot: Bot,
    msg: Message,
    cmd: Command,
    pool: sqlx::MySqlPool,
) -> ResponseResult<()> {
     let chat_id = msg.chat.id;
    if let Some(user) = msg.from() {
        let referred_by_telegram_id: Option<u64> = match &cmd {
            Command::Start(payload) => payload
                .trim()
                .parse::<u64>()
                .ok()
                .filter(|id| *id != user.id.0),
            _ => None,
        };

        if let Err(err) = db::upsert_telegram_user(&pool, user, referred_by_telegram_id).await {
            log::error!("upsert user failed: {err}");
        }
    }

     match cmd {
        Command::Start(_payload) => {
            let address = std::env::var("TOPUP_ADDRESS")
                .unwrap_or_else(|_| "TXkESNx5J3zjtWEtYE99JFRmMxf5rgUCCi".to_owned());

            let text = format!(
                "<b>一年节点费用：100 USDT（TRC20）</b>\n\n\
<b>1、充值</b>\n\
请向以下地址充值：\n\
<code>{address}</code>\n\n\
<b>2、提交交易哈希</b>\n\
充值成功后，请将交易哈希（TxHash）发送给客服，或直接在机器人下方输入。\n\n\
<b>3、等待开通</b>\n\
客服查阅确认后，将为你开通服务。\n\
预计 <b>10 分钟</b>内节点搭建完毕，并会通知你。\n\n\
<b>4、下载v2ray客户端</b>\n\
下载链接：<a href=\"https://itlanyan.com/v2ray-clients-download/\">点击这里</a>\n\
如以上链接无法下载，或需要其他客户端（Windows、iOS、Android、macOS），请联系我"
            );

            let qr_path = std::env::var("TOPUP_QR_PATH")
                .unwrap_or_else(|_| format!("{}/7777.png", env!("CARGO_MANIFEST_DIR")));
            let photo = InputFile::file(qr_path);

            match bot
                .send_photo(chat_id, photo)
                .caption(text.clone())
                .parse_mode(ParseMode::Html)
                .await
            {
                Ok(_) => {}
                Err(err) => {
                    log::error!("send_photo failed: {err}");
                    bot.send_message(chat_id, text)
                        .parse_mode(ParseMode::Html)
                        .await?;
                }
            }
         }
         Command::Help => {
             bot.send_message(chat_id, format!("联系客服 @{}", support_username()))
                 .await?;
         }
        Command::Referral => {
            if let Some(user) = msg.from() {
                let me = bot.get_me().await?;
                if let Some(bot_username) = me.user.username {
                    let link = format!("https://t.me/{bot_username}?start={}", user.id.0);
                    let amount = match db::get_user_amount(&pool, user.id.0).await {
                        Ok(amount) => amount,
                        Err(err) => {
                            log::error!("get amount failed: {err}");
                            "0.0".to_owned()
                        }
                    };
                    let text = format!(
                        "推广返佣 20%\n可提取返佣资金：{amount} USDT\n您的专属推广链接：\n{link}\n好友通过此链接进入并完成开通后，你可获得返佣。"
                    );
                    let mut request = bot.send_message(chat_id, text);
                    if let Some(url) = support_url_str().and_then(|u| u.parse().ok()) {
                        let markup = InlineKeyboardMarkup::new([[InlineKeyboardButton::url(
                            "联系客服".to_owned(),
                            url,
                        )]]);
                        request = request.reply_markup(markup);
                    }
                    request.await?;
                } else {
                    bot.send_message(chat_id, "当前机器人未设置用户名，无法生成推广链接。")
                        .await?;
                }
            } else {
                bot.send_message(chat_id, "无法获取用户信息，暂时无法生成推广链接。")
                    .await?;
            }
        }
     }
     Ok(())
 }

async fn handle_transaction_hash(
    bot: Bot,
    msg: Message,
    pool: sqlx::MySqlPool,
) -> ResponseResult<()> {
    let chat_id = msg.chat.id;
    let Some(user) = msg.from() else {
        bot.send_message(chat_id, "无法识别发送者信息，请稍后重试。").await?;
        return Ok(());
    };
    let Some(tx_hash) = msg.text().map(str::trim).filter(|text| !text.is_empty()) else {
        return Ok(());
    };

    if let Err(err) = db::upsert_telegram_user(&pool, user, None).await {
        log::error!("upsert user failed: {err}");
    }

    if let Err(err) = db::insert_transaction_hash(&pool, user.id.0, tx_hash).await {
        log::error!("insert transaction hash failed: {err}");
        bot.send_message(chat_id, "交易哈希保存失败，请稍后重试。").await?;
        return Ok(());
    }

    let mut notified_support = false;
    if let Some(support_id) = support_chat_id() {
        let username = user
            .username
            .as_deref()
            .map(|value| format!("@{value}"))
            .unwrap_or_else(|| "-".to_owned());
        let notify_text = format!(
            "收到新的交易哈希\ntelegram_id: {}\nusername: {}\nname: {}\ntx_hash: {}",
            user.id.0,
            username,
            user.full_name(),
            tx_hash
        );

        match bot.send_message(ChatId(support_id), notify_text).await {
            Ok(_) => notified_support = true,
            Err(err) => log::error!("notify support failed: {err}"),
        }
    } else {
        log::warn!("SUPPORT_CHAT_ID is not configured, skip support notification");
    }

    let reply = if notified_support {
        "已收到你的交易哈希，并已推送给客服。"
    } else {
        "已收到你的交易哈希，客服推送暂未配置，请稍后联系客服确认。"
    };
    bot.send_message(chat_id, reply).await?;

    Ok(())
}

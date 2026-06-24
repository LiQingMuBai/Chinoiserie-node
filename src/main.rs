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
        .unwrap_or_else(|_| "JoJotaroKujo".to_owned())
        .trim()
        .trim_start_matches('@')
        .to_owned()
}

fn support_url_str() -> Option<String> {
    std::env::var("SUPPORT_CONTACT_URL")
        .ok()
        .or_else(|| Some(format!("https://t.me/{}", support_username())))
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
        .parse_env(Env::default().default_filter_or("trace"))
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
    let commands = vec![
        BotCommand::new("start", "开始"),
        BotCommand::new("help", "联系客服"),
        BotCommand::new("referral", "推广返佣"),
    ];
    if let Err(err) = bot.set_my_commands(commands).await {
        log::error!("set_my_commands failed: {err}");
    }

    let handler = Update::filter_message()
        .filter_command::<Command>()
        .endpoint(answer);

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
                .unwrap_or_else(|_| "TQo2BpJ1hwjoa4ak8WmmrgwTHiHGp47777".to_owned());

            let text = format!(
                "*一年节点费用：100 USDT（TRC20）*\n\
\n\
*1、充值*\n\
请向以下地址充值：\n\
`{address}`\n\
\n\
*2、提交交易哈希*\n\
充值成功后，请将交易哈希（`TxHash`）发送给客服，或直接在机器人下方输入。\n\
\n\
*3、等待开通*\n\
客服查阅确认后，将为你开通服务。\n\
预计 *10 分钟*内节点搭建完毕，并会通知你。
\n\
*4、下载v2ray客户端*\n\
下载链接：[点击这里](https://itlanyan.com/v2ray-clients-download/)\n\
如以上链接无法下载，或需要其他客户端（Windows、iOS、Android、macOS），请联系我\n\
"
            );

            let qr_path = std::env::var("TOPUP_QR_PATH")
                .unwrap_or_else(|_| format!("{}/7777.png", env!("CARGO_MANIFEST_DIR")));
            let photo = InputFile::file(qr_path);

            match bot
                .send_photo(chat_id, photo)
                .caption(text.clone())
                .parse_mode(ParseMode::MarkdownV2)
                .await
            {
                Ok(_) => {}
                Err(err) => {
                    log::error!("send_photo failed: {err}");
                    bot.send_message(chat_id, text)
                        .parse_mode(ParseMode::MarkdownV2)
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


## 功能

- `/start`：展示充值说明、交易哈希提交、客户端下载链接
- `/help`：联系客服（支持从环境变量配置）
- `/referral`：推广返佣 20%，展示可提取返佣金额（amount），生成专属推广链接，并带「联系客服」按钮
- 用户留存：用户触发命令时 upsert 到 `telegram_users`，并更新 `last_seen_at`
- 推广归因：用户通过推广链接进入（`/start <推广者id>`）时，把 `referred_by_telegram_id` 写入新用户（仅首次补充，不覆盖已有）
- 广告群发：从 `telegram_users.telegram_id` 读取用户列表，按间隔群发 Bruno 布鲁诺节点推广文案

## 依赖

- Rust 2021
- MySQL（数据库：`ming`）

## 环境变量

建议创建 `.env`（参考 `.env.example`）。

- `TELOXIDE_TOKEN`：机器人 Token
- `DATABASE_URL`：MySQL 连接串，数据库名需为 `ming`
  - 示例：`mysql://user:password@127.0.0.1:3306/ming`
- `TOPUP_ADDRESS`：收款地址（可选）
- `TOPUP_QR_PATH`：收款二维码图片路径（可选）

客服配置：

- `SUPPORT_CONTACT`：客服用户名（不带 @ 也可以）
- `SUPPORT_CONTACT_URL`：客服链接（可选，不填则自动用 `https://t.me/{SUPPORT_CONTACT}`）

广告群发配置（仅 `bruno_ads_main` 使用）：

- `ADS_DRY_RUN`：true/false，true 时只打印不发送（建议先用 true 验证）
- `ADS_SLEEP_MS`：每条消息间隔毫秒数（建议 >= 350）
- `ADS_LIMIT`：限制发送数量（可选）
- `ADS_TEXT`：自定义广告文案（可选）

## MySQL 表结构

启动时会自动创建/补充表结构：

- 表：`telegram_users`
- 字段：
  - `telegram_id` BIGINT UNSIGNED (PK)
  - `username`/`first_name`/`last_name`/`language_code`/`is_bot`
  - `referred_by_telegram_id` BIGINT UNSIGNED NULL
  - `amount` VARCHAR(64) NOT NULL DEFAULT ''（展示时空/0 会显示 0.0 USDT）
  - `created_at`/`last_seen_at`

## 运行机器人

```bash
cargo run
```

## 推广链接

`/referral` 会给出专属推广链接（深链）：

```
https://t.me/<bot_username>?start=<你的telegram_id>
```

新用户通过该链接进入后，会触发 `/start <推广者id>`，并将推广者 id 写入 `telegram_users.referred_by_telegram_id`。

## 广告群发（Bruno）

广告群发是一个单独的二进制入口：

```bash
cargo run --bin bruno_ads_main
```

注意：

- 若 `ADS_DRY_RUN=true`，不会发送任何消息，只会输出日志。
- Bot 只能给“曾经和机器人发过消息/点过 /start”的用户私聊发送消息；否则会失败并在日志里显示错误信息。

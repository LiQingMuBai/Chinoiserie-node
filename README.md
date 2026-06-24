# telegram-bot

A Telegram bot built with `teloxide`, featuring MySQL user persistence, referral attribution, referral commission display, and a separate bulk advertising sender.

## Features

- `/start`: Displays top-up instructions, TxHash submission flow, and client download link
- `/help`: Shows support contact (configurable via environment variables)
- `/referral`: Referral commission (20%), displays withdrawable commission amount (`amount`), generates a personal referral link, and includes a "Contact Support" inline button
- User persistence: Upserts the user into `telegram_users` on each command and updates `last_seen_at`
- Referral attribution: When a user joins via a referral deep link (`/start <referrer_id>`), the new user's `referred_by_telegram_id` is filled once (it won't overwrite existing values)
- Bulk ads sender: Reads `telegram_users.telegram_id` and sends a Bruno ads message to users with a configurable interval

## Requirements

- Rust 2021
- MySQL (database: `ming`)

## Environment Variables

Create a `.env` file (see `.env.example`).

Core:

- `TELOXIDE_TOKEN`: Telegram bot token
- `DATABASE_URL`: MySQL connection string (database name must be `ming`)
  - Example: `mysql://user:password@127.0.0.1:3306/ming`
- `TOPUP_ADDRESS`: Top-up address (optional)
- `TOPUP_QR_PATH`: QR image path (optional)

Support:

- `SUPPORT_CONTACT`: Support username (with or without `@`)
- `SUPPORT_CONTACT_URL`: Support URL (optional). If empty, it defaults to `https://t.me/{SUPPORT_CONTACT}`

Bulk ads sender (only for `bruno_ads_main`):

- `ADS_DRY_RUN`: `true`/`false`. If `true`, it prints logs but does not send any messages (recommended for a first run)
- `ADS_SLEEP_MS`: Delay in milliseconds between messages (recommended `>= 350`)
- `ADS_LIMIT`: Optional limit for how many users to send to
- `ADS_TEXT`: Optional custom ad text

## MySQL Schema

On startup, the app will create/patch the schema automatically.

- Table: `telegram_users`
- Columns:
  - `telegram_id` BIGINT UNSIGNED (PK)
  - `username` / `first_name` / `last_name` / `language_code` / `is_bot`
  - `referred_by_telegram_id` BIGINT UNSIGNED NULL
  - `amount` VARCHAR(64) NOT NULL DEFAULT '' (displayed as `0.0 USDT` when empty/blank/0)
  - `created_at` / `last_seen_at`

## Run The Bot

This repository contains multiple binaries. By default, `cargo run` starts the bot:

```bash
cargo run
```

You can also specify the binary explicitly:

```bash
cargo run --bin telegram-bot
```

## Referral Link

`/referral` returns a personal deep link:

```
https://t.me/<bot_username>?start=<your_telegram_id>
```

When a new user joins via this link, Telegram will send `/start <referrer_id>` to the bot, and the bot will store the referrer ID into `telegram_users.referred_by_telegram_id`.

## Bulk Advertising Sender (Bruno)

The bulk ads sender is a separate binary:

```bash
cargo run --bin bruno_ads_main
```

Notes:

- If `ADS_DRY_RUN=true`, no messages will be sent; only logs will be printed.
- A bot can only message users who have previously started a chat with the bot (e.g. clicked `/start`). Otherwise, Telegram will reject the send request and the error will be logged.

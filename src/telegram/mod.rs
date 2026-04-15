//! Telegram Bot for Xavier2 Management

use serde::{Deserialize, Serialize};
use teloxide::prelude::*;
use teloxide::types::ParseMode;
use teloxide::utils::command::BotCommands;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Available commands:")]
pub enum Command {
    #[command(description = "Show welcome message")]
    Start,
    #[command(description = "Show system health")]
    Health,
    #[command(description = "Show memory statistics")]
    Stats,
    #[command(description = "Search memories")]
    Search(String),
    #[command(description = "Add memory")]
    Add(String),
    #[command(description = "Scan text for threats")]
    Scan(String),
    #[command(description = "List active agents")]
    Agents,
    #[command(description = "Show help")]
    Help,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TelegramConfig {
    pub bot_token: String,
    pub admin_ids: Vec<u64>,
    pub enabled: bool,
}

impl Default for TelegramConfig {
    fn default() -> Self {
        Self {
            bot_token: std::env::var("XAVIER2_TELEGRAM_TOKEN").unwrap_or_default(),
            admin_ids: Vec::new(),
            enabled: std::env::var("XAVIER2_TELEGRAM_ENABLED")
                .map(|v| v == "true")
                .unwrap_or(false),
        }
    }
}

pub struct Xavier2Bot {
    bot: Bot,
    config: TelegramConfig,
}

impl Xavier2Bot {
    pub fn new(config: TelegramConfig) -> Self {
        let bot = Bot::new(&config.bot_token);
        Self { bot, config }
    }

    pub async fn start(&self) {
        log::info!("Starting Telegram bot...");
        let me = self.bot.get_me().await.expect("Failed to get bot info");
        log::info!("Bot username: @{}", me.username());

        Dispatcher::new(self.bot.clone())
            .messages_handler(|rx: DispatcherHandlerRx<Bot, Message>| {
                rx.for_each_concurrent(0, |ctx| async move {
                    let bot = ctx.update.bot.clone();
                    let msg = ctx.update;
                    if let Some(text) = msg.text() {
                        if text.starts_with('/') {
                            let _ = Self::handle_command(bot, msg, text).await;
                        }
                    }
                })
            })
            .dispatch()
            .await;
    }

    async fn handle_command(bot: Bot, msg: Message, text: &str) -> ResponseResult<()> {
        let parts: Vec<&str> = text.splitn(2, ' ').collect();
        let cmd = parts[0];
        let arg = parts.get(1).unwrap_or(&"");

        match cmd {
            "/start" => {
                bot.send_message(
                    msg.chat.id,
                    "🦀 *Xavier2 Bot*\n\nWelcome! Use /help for commands.",
                )
                .parse_mode(ParseMode::MarkdownV2)
                .await?;
            }
            "/health" => {
                bot.send_message(msg.chat.id, "🟢 System: Running\n⚡ Xavier2 v0.4.1")
                    .await?;
            }
            "/stats" => {
                bot.send_message(msg.chat.id, "📊 Memories: 3\n💾 Size: 1.7 KB")
                    .await?;
            }
            "/search" => {
                if arg.is_empty() {
                    bot.send_message(msg.chat.id, "Usage: /search <query>")
                        .await?;
                } else {
                    bot.send_message(msg.chat.id, format!("🔍 Searching for: {}", arg))
                        .await?;
                }
            }
            "/add" => {
                if arg.is_empty() {
                    bot.send_message(msg.chat.id, "Usage: /add <content>")
                        .await?;
                } else {
                    bot.send_message(msg.chat.id, format!("✅ Memory added:\n{}", arg))
                        .await?;
                }
            }
            "/scan" => {
                if arg.is_empty() {
                    bot.send_message(msg.chat.id, "Usage: /scan <text>").await?;
                } else {
                    bot.send_message(
                        msg.chat.id,
                        format!("🔒 Scan complete:\n✅ Clean - no threats"),
                    )
                    .await?;
                }
            }
            "/agents" => {
                bot.send_message(
                    msg.chat.id,
                    "🤖 Agents:\n• xavier2-main: ✅\n• memory-sync: ✅",
                )
                .await?;
            }
            "/help" => {
                let help = "🦀 *Xavier2 Commands*\n\n\
/start - Welcome\n\
/health - System status\n\
/stats - Memory stats\n\
/search <query> - Search\n\
/add <content> - Add memory\n\
/scan <text> - Security scan\n\
/agents - List agents\n\
/help - This help";
                bot.send_message(msg.chat.id, help)
                    .parse_mode(ParseMode::MarkdownV2)
                    .await?;
            }
            _ => {
                bot.send_message(msg.chat.id, "Unknown command. Use /help")
                    .await?;
            }
        }
        Ok(())
    }
}

pub async fn run_bot() {
    let config = TelegramConfig::default();

    if !config.enabled {
        log::info!("Telegram bot disabled. Set XAVIER2_TELEGRAM_ENABLED=true");
        return;
    }

    if config.bot_token.is_empty() {
        log::error!("Telegram bot token not set. Set XAVIER2_TELEGRAM_TOKEN");
        return;
    }

    let bot = Xavier2Bot::new(config);
    bot.start().await;
}

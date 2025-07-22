use std::fs::OpenOptions;
use std::io::prelude::*;

use dotenv::dotenv;
use serenity::all::EventHandler;
use serenity::model::prelude::*;
use serenity::{async_trait, prelude::*};

struct ROBot;

#[async_trait]
impl EventHandler for ROBot {
    async fn message(&self, _ctx: Context, msg: Message) {
        log_msg(&msg);
    }
    async fn message_update(
        &self,
        _ctx: Context,
        _old_if_available: Option<Message>,
        _new: Option<Message>,
        event: MessageUpdateEvent,
    ) {
        log_msg_event(&event);
    }
}

/// This function dumps the cache to a log file to allow for reading it later
fn log_msg(message: &Message) {
    let msg = serde_json::to_string(message).expect("Serialising message failed!");
    let path = "./dump.log";
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)
        .expect("Failed to create or open logfile");
    writeln!(file, "{msg}").expect("Failed to wirte to file!");
}
/// This function dumps the cache to a log file to allow for reading it later
fn log_msg_event(event: &MessageUpdateEvent) {
    let msg = serde_json::to_string(event).expect("Serialising message failed!");
    let path = "./dump.log";
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)
        .expect("Failed to create or open logfile");
    writeln!(file, "{msg}").expect("Failed to wirte to file!");
}

#[tokio::main]
async fn main() {
    // Load secret from .env file
    dotenv().ok();

    // Configure the client with your Discord bot token in the environment.
    let token = std::env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_MODERATION;

    // Create a new instance of the Client, logging in as a bot. This will automatically prepend
    // your bot token with "Bot ", which is a requirement by Discord for bot users.
    let mut client = Client::builder(&token, intents)
        .event_handler(ROBot)
        .await
        .expect("Err creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}

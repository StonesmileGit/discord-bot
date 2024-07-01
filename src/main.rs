use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};

use dotenv::dotenv;
use itertools::Itertools;
use serenity::all::{Cache, EventHandler};
use serenity::model::prelude::*;
use serenity::{async_trait, prelude::*};

#[derive(Clone, PartialEq, Eq)]
struct Invite {
    user: User,
    server: GuildId,
}

struct PastMessages;
struct AdminDMCache;

#[derive(Debug, PartialEq, Eq, Hash)]
struct MessageInQueue {
    user_id: UserId,
    message: String,
    channel: ChannelId,
}

impl TypeMapKey for PastMessages {
    type Value = Arc<RwLock<VecDeque<(Instant, MessageInQueue)>>>;
}
impl TypeMapKey for AdminDMCache {
    type Value = Arc<RwLock<Cache>>;
}

struct ROBot;
// Time to keep messages in the cache
const TIME_SPAN: u64 = 60;
// Number of duplicate messages needed to ban
const REPEAT_MESSAGES: usize = 10;

#[async_trait]
impl EventHandler for ROBot {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot || msg.content.is_empty() {
            return;
        }
        let past_messages = ctx
            .data
            .read()
            .await
            .get::<PastMessages>()
            .expect("Expected list of past messages in TypeMap")
            .clone();

        {
            let mut q = past_messages.write().await;

            // Remove old invites from the queue
            loop {
                if let Some(elem) = q.back() {
                    if Instant::now().duration_since(elem.0) > Duration::from_secs(TIME_SPAN) {
                        q.pop_back();
                        continue;
                    }
                }

                break;
            }
            q.push_front((
                Instant::now(),
                MessageInQueue {
                    user_id: msg.author.id,
                    message: msg.content.clone(),
                    channel: msg.channel_id,
                },
            ));
            println!(
                "Messages in queue:\n{}",
                q.iter().map(|m| m.1.message.clone()).format("\n")
            );
        }

        // We have now updated our queue, count identical invites
        let same_user_count = past_messages
            .read()
            .await
            .iter()
            .filter(|a| a.1.user_id == msg.author.id && a.1.message == msg.content)
            .unique()
            .count();

        println!("count: {same_user_count}");

        if same_user_count > REPEAT_MESSAGES {
            ban_user(ctx, msg).await;
        }
    }
}

async fn ban_user(ctx: Context, msg: Message) {
    println!("Attempting to ban user");
    // ban user
    let res = ctx
                .http
                .ban_user(
                    msg.guild_id.expect("Expected message to be sent on server"),
                    msg.author.id,
                    1,
                    Some(format!("RO-Bot Auto-Ban: Posted too many duplicate messages in a short time: {} identical messages in {}s", REPEAT_MESSAGES, TIME_SPAN).as_str()),
                )
                .await;
    if let Err(error) = res {
        println!("Banning user failed: {error:?}");
    }
}

#[allow(unused)]
async fn notify_me(ctx: Context) {
    // Send a message to me directly
    let user_id = UserId::new(277536889165316096);
    let cache = &ctx.cache;
    let http = ctx.http();
    let channel = user_id.create_dm_channel((cache, http)).await.unwrap();
    let res = channel.say(http, "Test").await;
    match res {
        Ok(_) => (),
        Err(err) => println!("{err}"),
    }
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

    {
        // Open the data lock in write mode, so keys can be inserted to it.
        let mut data = client.data.write().await;

        data.insert::<PastMessages>(Arc::new(RwLock::new(VecDeque::new())));
        data.insert::<AdminDMCache>(Arc::new(RwLock::new(Cache::new())));
    }
    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}

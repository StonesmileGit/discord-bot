use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};

use dotenv::dotenv;
use itertools::Itertools;
use serenity::all::EventHandler;
use serenity::model::prelude::*;
use serenity::{async_trait, prelude::*};

struct PastMessages;
impl TypeMapKey for PastMessages {
    type Value = Arc<RwLock<VecDeque<(Instant, Message)>>>;
}

struct ROBot;
// Time in seconds to keep messages in the list
const TIME_SPAN: u64 = 60;
// Number of duplicate messages needed to ban
const REPEAT_MESSAGES: usize = 3;

#[async_trait]
impl EventHandler for ROBot {
    async fn message(&self, ctx: Context, msg: Message) {
        check_for_duplicate_messages(&ctx, &msg).await;
    }
}

async fn check_for_duplicate_messages(ctx: &Context, new_message: &Message) {
    // No need to process the message if the user is one of our bots
    if new_message.author.bot {
        return;
    }
    let past_messages = ctx
        .data
        .read()
        .await
        .get::<PastMessages>()
        .expect("Expected list of past messages in TypeMap")
        .clone();

    // block creates scope where past_messages is locked for writing
    {
        let mut q = past_messages.write().await;

        // Remove old messages from the queue
        loop {
            if let Some(elem) = q.back() {
                if Instant::now().duration_since(elem.0) > Duration::from_secs(TIME_SPAN) {
                    q.pop_back();
                    continue;
                }
            }

            break;
        }
        q.push_front((Instant::now(), new_message.clone()));
        log::info!(
            "Messages in queue:\n{}",
            q.iter().map(|m| m.1.content.clone()).format("\n")
        );
    }

    // We have now updated our queue, count identical messages
    let items = past_messages.read().await;
    let same_user_messages: Vec<&(Instant, Message)> = items
        .iter()
        // This filter is intended to filter out messages that are the same as the new one.
        .filter(|(_, message)| compare_messages(&message, new_message))
        // Additionally filter by channels being unique, as the same message posted repeatedly
        // in the same channel is a different issue that we are not trying to address
        .unique_by(|(_, message)| message.channel_id)
        .collect();
    let same_user_count = same_user_messages.len();

    log::info!("count: {same_user_count}");

    if same_user_count >= REPEAT_MESSAGES {
        // TODO: change to conditionally enable actions below based on config
        // ban_user(ctx, new_message).await;
        jail_user(ctx, &same_user_messages).await;
        notify_me(ctx, &same_user_messages).await;
    }
}

async fn jail_user(ctx: &Context, messages: &[&(Instant, Message)]) {
    let guild_id = messages[0]
        .1
        .guild_id
        .expect("Expected the message to be sent in a server");
    let user_id = messages[0].1.author.id;
    // Add user to group @muted
    let role_id = RoleId::new(876090959589425152);
    let audit_log_reason = Some(
        "User was quarantiened for sending too many messages in a short time in different channels",
    );
    let res = ctx
        .http
        .add_member_role(guild_id, user_id, role_id, audit_log_reason)
        .await;
    if let Err(error) = res {
        log::error!("Adding role to user failed: {error:?}");
    }
    // Delete the passed messages
    for message in messages {
        let channel_id = message.1.channel_id;
        let message_id = message.1.id;
        let res = ctx
            .http
            .delete_message(channel_id, message_id, audit_log_reason)
            .await;
        if let Err(error) = res {
            log::error!("Deleting messages failed: {error:?}");
        }
    }
}

/// Returns true if messages are concidered to be identical for purposes of spam detection
#[allow(unused)]
fn compare_messages(msg1: &Message, msg2: &Message) -> bool {
    // If the messages do not have the same author, they clearly cannot be identical
    if msg1.author.id != msg2.author.id {
        return false;
    }

    if msg1.content != msg2.content {
        return false;
    }

    //if !compare_attachments(msg1, msg2) {
    //    return false;
    //}

    if !compare_embeds(msg1, msg2) {
        return false;
    }

    // We have checked for all cases where the messages could be different. Return true as the messages are the same
    true
}

fn compare_embeds(msg1: &Message, msg2: &Message) -> bool {
    if msg1.embeds.len() != msg2.embeds.len() {
        return false;
    }

    if msg1.embeds.len() == 0 {
        return true;
    }

    'outer: for embed1 in &msg1.embeds {
        for embed2 in &msg2.embeds {
            if compare_embeds_inner(embed1, embed2) {
                continue 'outer;
            }
        }
        // Found no match for the embed, return false
        return false;
    }
    // All embeds match
    true
}

fn compare_embeds_inner(embed1: &Embed, embed2: &Embed) -> bool {
    if embed1.title != embed2.title {
        return false;
    }
    if embed1.description != embed2.description {
        return false;
    }

    if embed1.kind != embed2.kind {
        return false;
    }

    if embed1.url != embed2.url {
        return false;
    }

    true
}

/// Checks if the messages have identical attachments
fn compare_attachments(msg1: &Message, msg2: &Message) -> bool {
    // If the number of attachments do not match, the messages are not identical
    if msg1.attachments.len() != msg2.attachments.len() {
        return false;
    }

    // If neither message has attachments, they are identical. Handle edge case 0 here.
    if msg1.attachments.len() == 0 {
        return true;
    }

    let matches = msg1
        .attachments
        .iter()
        .cartesian_product(msg2.attachments.iter())
        .filter(|(_att1, _att2)| {
            // TODO: Actually check if the attachments are the same
            todo!()
        })
        .count();
    return msg1.attachments.len() == matches;
}

#[allow(unused)]
async fn ban_user(ctx: &Context, msg: &Message) {
    println!("Attempting to ban user");
    // ban user
    let res = ctx
                .http
                .ban_user(
                    msg.guild_id.expect("Expected offending message to be sent on server"),
                    msg.author.id,
                    // TODO: Is this the best way, or should the offending messages be deleted in a more controlled way?
                    1,
                    Some(format!("RO-Bot Auto-Ban: Posted too many duplicate messages in a short time: {} identical messages in {}s", REPEAT_MESSAGES, TIME_SPAN).as_str()),
                )
                .await;
    if let Err(error) = res {
        log::error!("Banning user failed: {error:?}");
    }
}

// TODO: Something like this can be used, but instead post in specified channel (#bot-moderator)
#[allow(unused)]
async fn notify_me(ctx: &Context, messages: &[&(Instant, Message)]) {
    // Send a message to me directly
    let user_id = UserId::new(277536889165316096);
    let cache = &ctx.cache;
    let http = ctx.http();
    let channel = user_id.create_dm_channel((cache, http)).await.unwrap();
    //let res = channel.say(http, "I did a thing!").await;
    let res = channel
        .say(
            http,
            format!(
                "User {} posted too quickly!",
                messages[0].1.author.display_name()
            ),
        )
        .await;
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
    }
    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}

use dotenv::dotenv;
use serenity::all::{Cache, Http, UserId};
use std::sync::Arc;

async fn notify_me(cache: &Arc<Cache>, http: &Http) {
    // Send a message to me directly
    let user_id = UserId::new(277536889165316096);
    // let cache = &ctx.cache;
    // let http = ctx.http();
    let channel = user_id.create_dm_channel((cache, http)).await.unwrap();
    let res = channel.say(http, "Test").await;
    match res {
        Ok(_) => (),
        Err(err) => println!("{err}"),
    }
}

#[allow(unused)]
async fn debug_me(cache: &Arc<Cache>, http: &Http) {
    // Send a message to me directly
    let user_id = UserId::new(277536889165316096);
    // let cache = &ctx.cache;
    // let http = ctx.http();
    let user_name = user_id.to_user((cache, http)).await.unwrap().name;
    println!("User: {user_name}");
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    // Configure the client with your Discord bot token in the environment.
    let token = std::env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let cache = Arc::new(Cache::new());
    let http = Http::new(&token);
    notify_me(&cache, &http).await;
}

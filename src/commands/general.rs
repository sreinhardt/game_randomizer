use serenity::prelude::*;
use serenity::client::Context;
use serenity::model::channel::{MessageType, Message};
use serenity::framework::standard::{
    CommandResult,
    macros::{group, command}
};

use crate::ShardManagerContainer;

#[group]
#[commands(ping, quit)]
pub struct General;

// PING_COMMAND
#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    if msg.kind != MessageType::Regular {
        return Err( "Invalid message type".into() );
    }
    msg.reply(ctx, "Pong!").await?;

    Ok(())
}
#[command]
#[owners_only]
async fn quit(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;

    if let Some(manager) = data.get::<ShardManagerContainer>() {
        msg.reply(ctx, "Shutting down!").await?;
        manager.lock().await.shutdown_all().await;
    } else {
        msg.reply(ctx, "There was a problem getting the shard manager").await?;

        return Ok(());
    }

    Ok(())
}

use std::fs;
use std::fs::File;
use std::path::Path;
use std::io::prelude::*;
//use serenity::prelude::*;
use serenity::client::Context;
use serenity::model::channel::{MessageType, Message};
use serenity::framework::standard::{
    CommandResult,
    macros::{group, command}
};
use serenity::utils::MessageBuilder;

use crate::ShardManagerContainer;
use crate::commands::{GameSuggestions, PlayerContainer};

#[group]
#[commands(ping, quit, save, get_ids)]
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
async fn get_ids(ctx: &Context, msg: &Message) -> CommandResult {
    let mut response = MessageBuilder::new();
    
    if let Some(guild) = msg.guild(&ctx.cache).await {
        response.push_line(format!{"Guild(name:{}, id:{})", guild.name, guild.id});
        response.push_line("Members:");
        let _: Vec<u8> = guild.members(&ctx.http, None, None).await?.iter()
            .map(|user| {
                if let Some(nick) = &user.nick {
                    response.push_line(format!{"{} ({}, {})", nick, user.user.id, user.user.name});
                } else {
                    response.push_line(format!{"({}, {})", user.user.id, user.user.name});
                }
                0
            }).collect();
    } else {
        response.push_line("msg.guild not found!");
    }
    let dm = msg.author.create_dm_channel(&ctx).await?;
    dm.send_message(&ctx.http, |m| m.content(response.build()) ).await?;
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
#[command]
#[owners_only]
async fn save(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;
    let storage = match data.get::<crate::config::Config>() {
        Some(c) => &c.storage,
        None => {
            msg.reply(ctx, "No stored configuration with save path.").await?;
            return Ok(()); // todo error
        }
    };
    let storage = Path::new(&storage);
    match fs::create_dir_all(storage) {
        Ok(_) => {},
        Err(_) => {
            msg.reply(ctx, "Failed opening storage path.").await?;
            return Ok(());
        }
    };
    match File::create(storage.join("suggestions.json")) {
        Err(_) => {
            msg.reply(ctx, "Failure opening suggestions file").await?;
            return Ok(());
        },
        Ok(p) => {
            let inner = data.get::<GameSuggestions>().expect("no suggestions read data");
            match serde_json::to_writer(p, inner) {
                Ok(_) => {},
                Err(_) => {
                    msg.reply(ctx, "Error serializing game suggestions.").await?;
                    return Ok(());
                }
            };
        }
    };
    match File::create(storage.join("players.json")) {
        Err(_) => {
            msg.reply(ctx, "Failure opening suggestions file").await?;
            return Ok(());
        },
        Ok(p) => {
            let inner = data.get::<PlayerContainer>().expect("no players read data");
            match serde_json::to_writer(p, inner) {
                Ok(_) => {},
                Err(_) => {
                    msg.reply(ctx, "Error serializing players.").await?;
                    return Ok(());
                }
            };
        }
    };
    msg.reply(ctx, "Saving successful!").await?;
    Ok(())
}

use std::fs;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use serde::ser::{Serialize as SerSerialize, SerializeTupleVariant, Serializer};

use serenity::prelude::*;
use serenity::prelude::SerenityError;
use serenity::client::Context;
use serenity::model::id::GuildId;
use serenity::model::channel::{ ChannelType,  MessageType};
use serenity::utils::EmbedMessageBuilding;
use serenity::{
    async_trait,
    client::bridge::gateway::{GatewayIntents, ShardId, ShardManager},
    framework::standard::{
        buckets::{LimitedFor, RevertBucket},
        help_commands,
        macros::{check, command, group, help, hook},
        Args,
        CommandGroup,
        CommandOptions,
        CommandResult,
        DispatchError,
        HelpOptions,
        Reason,
        StandardFramework,
    },
    http::Http,
    model::{
        channel::{Channel, Message},
        gateway::Ready,
        id::UserId,
        permissions::Permissions,
    },
    utils::{
        content_safe, ContentSafeOptions,
        MessageBuilder
    }
};
use rsteam::SteamID;


use crate::endpoints::steam;

#[derive(Serialize,Deserialize)]
pub enum Player {
    Steam(UserId, u64),
}
impl Player {
    pub fn discord(&self) -> &UserId {
        match self {
            Player::Steam(user, _) => &user
        }
    }
    pub fn steam(&self) -> SteamID {
        match self {
            Player::Steam(_, user) => {
                let id:SteamID = (*user).into();
                id
            }
        }
    }
}


#[derive(Deserialize,Serialize)]
pub struct PlayerContainer;
impl TypeMapKey for PlayerContainer {
    type Value = HashMap<GuildId, Vec<Player>>;
}
impl PlayerContainer {
    pub fn new() -> HashMap<GuildId, Vec<Player>> {
        HashMap::new()
    }
}

#[group]
#[commands(add_steam_id)]
pub struct Players;

#[command]
async fn add_steam_id(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    // https://www.ubisoft.com/en-gb/help/article/finding-your-steam-id/000060565
    let gid = match msg.guild_id {
        Some(g) => g,
        None => {
            msg.reply(ctx, "Cannot add players to non-guild channel.").await?;
            return Ok(()); // todo error
        }
    };
    let id: u64 = match args.single_quoted::<String>() {
        Ok(id) => match id.parse::<u64>() {
            Ok(i) => i,
            Err(_) => {
                msg.reply(ctx, "Failed parsing steam id as u64").await?;
                return Ok(());
            }
        },
        Err(_) => {
            msg.reply(ctx, "Please provide a steam id.").await?;
            return Ok(());
        },
    };
    let sid: SteamID = id.into();
    {
        let mut wlock = ctx.data.write().await;
        let mut winner = wlock.get_mut::<PlayerContainer>().expect("no player data stored");
        if let Some(existing) = winner.get_mut(&gid) {
            for player in existing.iter_mut() {
                if *player.discord() == msg.author.id {
                    if player.steam() == sid {
                        msg.reply(ctx, "Your discord and steam users match.").await?;
                    } else {
                        *player = Player::Steam(msg.author.id, id);
                        msg.reply(ctx, "Updated your steam id.").await?;
                    }
                    return Ok(());
                }
            }
            // no matches
            existing.push(Player::Steam(msg.author.id, id));
            msg.reply(ctx, "Added steam id to your user.").await?;
        } else { // no gid or user
            winner.insert(gid, vec![ Player::Steam(msg.author.id, id) ]);
            msg.reply(ctx, "Added steam id to your user.").await?;
        }
    }
    Ok(())
}
    

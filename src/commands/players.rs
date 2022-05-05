use std::fs;
use std::cmp::PartialEq;
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
use rsteam::{
    SteamID,
    player_service::{OwnedGames, OwnedGame}
};


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
#[commands(add_steam_id, find_common_games)]
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
    let sid: SteamID = match args.single_quoted::<String>() {
        Ok(id) => match id.parse::<u64>() {
            Ok(i) => i.into(),
            Err(_) => {
                let rlock = ctx.data.read().await;
                let steam_inner = rlock.get::<steam::Client>().expect("no steam client found");
                let u = match (**steam_inner).lock().await.resolve_vanity_user(&id).await {// is string
                    Ok(u) => u,
                    Err(_) => {
                        msg.reply(ctx, "Invalid steam id and vanity user provided.").await?;
                        return Ok(());
                    }
                };
                u
            }
        },
        Err(_) => {
            msg.reply(ctx, "Please provide a steam id or user.").await?;
            return Ok(());
        },
    };
    let id:u64 = (&sid).into();
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


fn owned_game_eq(this:&OwnedGame, other: &OwnedGame) -> bool {
    this.appid == other.appid
}
fn find_common(left: &OwnedGames, right: &OwnedGames) -> OwnedGames {
    let mut mutual = OwnedGames{
        game_count: 0,
        games: Vec::new()
    };
    for lgame in left.games.iter() {
        //if right.games.contains(lgame) {
        if right.games.iter().any(|g| owned_game_eq(g, lgame)) {
            let name = match lgame.name.as_deref() {
                Some(n) => Some(n.to_owned()),
                None => None
            };
            let img_icon_url = match lgame.img_icon_url.as_deref() {
                Some(i) => Some(i.to_owned()),
                None => None
            };
            let img_logo_url = match lgame.img_logo_url.as_deref() {
                Some(i) => Some(i.to_owned()),
                None => None
            };
            mutual.games.push(
                OwnedGame {
                    appid: lgame.appid,
                    name: name,
                    playtime_forever: lgame.playtime_forever,
                    img_icon_url: img_icon_url,
                    img_logo_url: img_logo_url,
                    playtime_windows_forever: lgame.playtime_windows_forever,
                    playtime_mac_forever: lgame.playtime_mac_forever,
                    playtime_linux_forever: lgame.playtime_linux_forever
                }
            );
        }
    }
    mutual.game_count = mutual.games.len() as u32;
    mutual
}
#[command]
async fn find_common_games(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let gid = match msg.guild_id {
        Some(g) => g,
        None => {
            msg.reply(ctx, "Cannot add players to non-guild channel.").await?;
            return Ok(()); // todo error
        }
    };
    let guild_users = match gid.members(ctx, None, None).await {
        Ok(u) => u,
        Err(_) => {
            msg.reply(ctx, "Failed gathering guild users.").await?;
            return Ok(());
        }
    };
    let mut users: Vec<UserId> = Vec::new();
    for user in args.iter::<String>() {
        // discord name -> discord id
        let user = match user {
            Ok(u) => u,
            Err(_) => {break;}
        };
        for g_user in guild_users.iter() {
            if let Some(nick) = &g_user.nick {
                if *nick == user {
                    users.push(g_user.user.id);
                    break;
                } else {
                }
            }
            // didnt match or no nick
            if g_user.user.name == user {
                users.push(g_user.user.id);
                break;
            }
        }
    }
    if users.len() < 2 {
        msg.reply(ctx, "Not enough discord names found to find common games.").await?;
        return Ok(());
    }
    let mut ids: Vec<SteamID> = Vec::new();
    // match discord names and store associated steam id
    { // playercontainer read lock
        let rlock = ctx.data.read().await;
        let hash = rlock.get::<PlayerContainer>().expect("no player container found");
        let players = match hash.get(&gid) {
            Some(p) => p,
            None => {
                msg.reply(ctx, "No player to steam mappings found for guild.").await?;
                return Ok(());
            }
        };

        for d_user in users.iter() {
            for player in players {
                if player.discord() == d_user {
                    ids.push(player.steam());
                }
            }
        }
    }
    if ids.len() < 2 {
        msg.reply(ctx, "Not enough names match discord users to find common games.").await?;
        return Ok(());
    }
    let mut owned_games: Vec<OwnedGames> = Vec::new();
    // get all steam games for each steam id
    { // steam read lock
        let rlock = ctx.data.read().await;
        let steam_inner = rlock.get::<steam::Client>().expect("no steam client found");
        for id in ids.iter() {
            match (**steam_inner).lock().await.user_owned_games(&id).await {
                Ok(g) => owned_games.push(g),
                Err(_) => {
                    msg.reply(ctx, format!{"Could not find games for: {}", id}).await?;
                    //return Ok(());
                }
            };
        }
    }
    // find common across all games
    let common = if let Some(first) = owned_games.pop() { // grab a users games as acc
        owned_games.iter().fold(first, |acc, right| find_common(&acc, right))
    } else {
        msg.reply(ctx, "Failure processing common games.").await?;
        return Ok(());
    };
    if common.game_count == 0 || common.games.is_empty() {
        msg.reply(ctx, "There are no shared games between requested players.").await?;
        return Ok(());
    }
    //convert to names - link
    let mut games = Vec::new();
    { // steam read lock
        let rlock = ctx.data.read().await;
        let steam_inner = rlock.get::<steam::Client>().expect("no steam client found");
        let steam_lock = (**steam_inner).lock().await;

        for game in common.games.iter() {
            let name = match &game.name {
                Some(n) => n.clone(),
                None => {// appid to name
                    match steam_lock.game_by_id(game.appid).await {
                        Ok(app) => app.name,
                        Err(()) => String::from("** NNF **"),
                    }
                }
            };
            games.push(format!{"{} - https://store.steampowered.com/app/{}/\r\n", name, game.appid});
        }
    }
    // separate code blocks into <2k messages
    let mut game_block = String::new();
    let mut count = 0;
    for game in games.iter() {
        if game_block.len() + game.len() <= 1950 {
            game_block.push_str(game);
        } else {
            let response = MessageBuilder::new()
                .push_line(&format!{"Common games {}", count})
                .push_codeblock_safe(game_block, None)
                .build();
            msg.reply(ctx, response).await?;
            game_block = String::new();
            count += 1;
        }
    }
    if game_block.len() != 0 {
        let response = MessageBuilder::new()
            .push_line(&format!{"Common games {}", count})
            .push_codeblock_safe(game_block, None)
            .build();
        msg.reply(ctx, response).await?;
        game_block = String::new();
    }
    Ok(())
}

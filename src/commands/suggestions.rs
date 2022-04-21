use std::fmt;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

use serenity::prelude::*;
use serenity::client::Context;
use serenity::model::id::GuildId;

use serenity::{
    async_trait,
    framework::standard::{
        macros::{command, group},
        Args,
        CommandResult
    },
    model::{
        channel::Message,
        id::UserId,
    },
    utils::{
        MessageBuilder
    }
};
use crate::endpoints::steam;

#[derive(Deserialize,Serialize)]
pub enum Suggestion {
    Steam(UserId, steam::App),
    PlainText(UserId, TextSuggestion)
}
impl<'a> Suggestion {
    pub fn title(&'a self) -> &'a str {
        use self::Suggestion::*;
        match self {
            Steam(_, app) => app.name.as_ref(),
            PlainText(_,app) => app.title.as_ref()
        }
    }
    pub fn user(&self) -> UserId {
        use self::Suggestion::*;
        match self {
            Steam(user, _) => *user,
            PlainText(user, _) => *user
        }
    }
}
impl PartialEq for Suggestion {
    fn eq(&self, other: &Suggestion) -> bool {
        use self::Suggestion::*;
        match self {
            Steam(_, s_app) => match other {
                Steam(_, o_app) => s_app.id == o_app.id,
                PlainText(_, o_app) => s_app.name == o_app.title,
            },
            PlainText(_, s_app) => match other {
                Steam(_, o_app) => s_app.title == o_app.name,
                PlainText(_, o_app) => s_app.title == o_app.title,
            },
        }
    }
}
impl Default for Suggestion {
    fn default() -> Self {
        Suggestion::PlainText(UserId::default(), TextSuggestion::default())
    }
}

#[derive(Debug,Hash,Deserialize,Serialize)]
pub struct TextSuggestion {
    pub title: String,
    pub genre: Option<String>,
    pub url: Option<String>,
}
impl Default for TextSuggestion {
    fn default() -> Self {
        TextSuggestion {
            title: String::new(),
            genre: None, 
            url: None
        }
    }
}
impl fmt::Display for TextSuggestion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut rst = write!(f, "Title: {}", self.title);
        if let Some(genre) = &self.genre {
            rst = write!(f, " | Genre: {}", genre);
        }
        if let Some(url) =&self.url {
            rst = write!(f, " | Url: {}", url);
        }
        rst
    }
}

#[derive(Deserialize,Serialize)]
pub struct GameSuggestions;
impl TypeMapKey for GameSuggestions {
    type Value = HashMap<GuildId, Vec<Suggestion>>;
}
impl GameSuggestions {
    pub fn new() -> HashMap<GuildId, Vec<Suggestion>> {
        HashMap::new()
    }
}

#[group]
#[commands(add_suggestion, list_suggestions, remove_suggestion)]
pub struct Suggestions;
// ~add_game title genre url
#[command]
#[aliases("suggest")]
async fn add_suggestion(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    
    match args.single_quoted::<String>() {
        Ok(t) => {
            match t.to_ascii_lowercase().trim() {
                "plain" => add_text_suggestion(ctx, msg, args).await,
                "steam" => add_steam_suggestion(ctx, msg, args).await,
                _ => {
                    msg.reply(ctx, "Invalid suggestion type. Try 'plain' or 'steam'.").await?;
                    Ok(())
                }
            }
        },
        Err(_) => { 
            println!{"Failed parsing add_suggestion"};
            Ok(())
        },
    }
}

#[command]
#[aliases("suggestions")]
async fn list_suggestions(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let gid = match msg.guild_id {
        Some(g) => g,
        None => {
            msg.reply(ctx, "Cannot add suggestion to non-guild channel.").await?;
            return Ok(()); // todo error
        }
    };
    let mut response = MessageBuilder::new();
    response.push_line("Currently suggested games");
    let games = {
        use self::Suggestion::*;
        let rlock = ctx.data.read().await;
        let inner = rlock.get::<GameSuggestions>().expect("no suggestions read data");
        let mut game_block = String::new();
        if let Some(existing) = inner.get(&gid) {
            let _ = existing.iter().map(|s|{
                match s {
                    PlainText(_, app) => {
                        game_block.push_str("Title: ");
                        game_block.push_str(&app.title);
                        if let Some(genre) = &app.genre {
                            game_block.push_str(" Genre: ");
                            game_block.push_str(genre);
                        }
                        if let Some(url) = &app.url {
                            game_block.push_str("Url: ");
                            game_block.push_str(url);
                        }
                    },
                    Steam(_, app) => {
                        game_block.push_str("Steam: ");
                        game_block.push_str(&app.name);
                        game_block.push_str(" - ");
                        game_block.push_str(&app.url());
                    }
                };
                game_block.push_str("\r\n");
            }).collect::<()>();
        };
        game_block
    };
    response.push_codeblock(games, None);
    msg.reply(ctx, response.build()).await?;
    Ok(())
}

#[command]
async fn remove_suggestion(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {

    match args.single_quoted::<String>() {
        Ok(t) => {
            match t.to_ascii_lowercase().trim() {
                "plain" => remove_text_suggestion(ctx, msg, args).await,
                "steam" => remove_steam_suggestion(ctx, msg, args).await,
                _ => {
                    msg.reply(ctx, "Invalid suggestion type. Try 'plain' or 'steam'.").await?;
                    Ok(())
                }
            }
        },
        Err(_) => { 
            println!{"Failed parsing remove_suggestion"};
            Ok(())
        },
    }
}

async fn remove_text_suggestion(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let gid = match msg.guild_id {
        Some(g) => g,
        None => {
            msg.reply(ctx, "Cannot add suggestion to non-guild channel.").await?;
            return Ok(()); // todo error
        }
    };
    
    let mut suggestion = TextSuggestion::default();
    let mut response = MessageBuilder::new();

    match args.single_quoted::<String>() {
        Ok(t) => {
            suggestion.title = t.trim().to_string();
        },
        Err(_) => {
            msg.reply(ctx, "Invalid command, no title provided").await?;
            return Ok(());
        },
    };
    match args.single_quoted::<String>() {
        Ok(g) => {
            suggestion.genre = Some(g.clone());
        },
        Err(_) => {},
    };
    match args.single_quoted::<String>() {
        Ok(u) => {
            suggestion.url = Some(u.clone());
        },
        Err(_) => {},
    };
    let mut idx = std::usize::MAX;
    let suggestion = Suggestion::PlainText(msg.author.id, suggestion);
    let tmp = suggestion.title().to_ascii_lowercase();
    { // read lock
        use self::Suggestion::*;
        let rlock = ctx.data.read().await;
        let inner = rlock.get::<GameSuggestions>().expect("no suggestions read data");
        if let Some(existing) = inner.get(&gid) {
            idx = match (*existing).iter()
                .map(|sug| sug.title() )
                .position(|s| s.to_ascii_lowercase() == tmp) {
                    Some(i) => i,
                    None => std::usize::MAX,
                };
        };
    }
    { // write lock
        if idx == std::usize::MAX {
            response.push("No suggestion with that title.");
        } else {
            let mut wlock = ctx.data.write().await;
            let mut winner = wlock.get_mut::<GameSuggestions>().expect("no suggestions write data");
            if let Some(existing) = winner.get_mut(&gid) {
                if existing[idx].user() == suggestion.user() {
                    let _ = existing.swap_remove(idx);
                    response.push("Removed suggestion: ")
                        .push_line(suggestion.title());
                } else {
                    response.push_line("Found suggestion but cannot be removed by you.");
                }
            };
        }
    }
    msg.reply(ctx, response.build()).await?;
    Ok(())
}
async fn remove_steam_suggestion(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let gid = match msg.guild_id {
        Some(g) => g,
        None => {
            msg.reply(ctx, "Cannot add suggestion to non-guild channel.").await?;
            return Ok(()); // todo error
        }
    };
    
    let mut response = MessageBuilder::new();
    let suggestion = match args.single_quoted::<String>() {
        Ok(t) => t.trim().to_string(),
        Err(_) => {
            msg.reply(ctx, "No id or name provided").await?;
            return Ok(());
        },
    };
    let suggestion = { // read-lock to find game from steam apps
        let rlock = ctx.data.read().await;
        let steam_inner = rlock.get::<steam::Client>().expect("no global steam::Client");
        let app = match suggestion.parse::<u32>() {
            Ok(id) => { // parsed as int, use as id
                match (**steam_inner).lock().await.game_by_id(id).await {
                    Ok(a) => a,
                    Err(_) => {
                        msg.reply(ctx, "Invalid Steam Id requested.").await?;
                        return Ok(());
                    }
                }
            },
            Err(_) => { // wasnt an integer, treat as title
                match (**steam_inner).lock().await.game_by_name(&suggestion).await {
                    Ok(a) => a,
                    Err(_) => {
                        msg.reply(ctx, "Invalid Steam Name requested.").await?;
                        return Ok(());
                    }
                }
            }
        };
        Suggestion::Steam(
            msg.author.id,
            app
        )
    };

    let mut idx = std::usize::MAX;
    let tmp = suggestion.title().to_ascii_lowercase();
    { // read lock
        let rlock = ctx.data.read().await;
        let inner = rlock.get::<GameSuggestions>().expect("no suggestions read data");
        if let Some(existing) = inner.get(&gid) {
            idx = match (*existing).iter()
                .map(|sug| sug.title() )
                .position(|s| s.to_ascii_lowercase() == tmp) {
                    Some(i) => i,
                    None => std::usize::MAX,
                };
        };
    }
    { // write lock
        if idx == std::usize::MAX {
            response.push("No suggestion with that title.");
        } else {
            let mut wlock = ctx.data.write().await;
            let mut winner = wlock.get_mut::<GameSuggestions>().expect("no suggestions write data");
            if let Some(existing) = winner.get_mut(&gid) {
                if existing[idx].user() == suggestion.user() {
                    let _ = existing.swap_remove(idx);
                    response.push("Removed suggestion: ")
                        .push_line(suggestion.title());
                } else {
                    response.push_line("Found suggestion but cannot be removed by you.");
                }
            };
        }
    }
    msg.reply(ctx, response.build()).await?;
    Ok(())
}


async fn add_text_suggestion(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let mut suggestion = TextSuggestion::default();
    let mut response = String::new(); //TODO switch to messagebuilder
    
    match args.single_quoted::<String>() {
        Ok(t) => {
            suggestion.title = t.trim().to_string();
            response.push_str("Added: ");
            response.push_str(&suggestion.title);
        },
        Err(_) => {},
    };
    match args.single_quoted::<String>() {
        Ok(g) => {
            response.push_str(" genre: "); 
            response.push_str(&g);
            suggestion.genre = Some(g.clone());
        },
        Err(_) => {},
    };
    match args.single_quoted::<String>() {
        Ok(u) => {
            response.push_str(" url: ");
            response.push_str(&u);
            suggestion.url = Some(u.clone());
        },
        Err(_) => {},
    };
    let gid = match msg.guild_id {
        Some(g) => g,
        None => {
            msg.reply(ctx, "Cannot add suggestion to non-guild channel.").await?;
            return Ok(()); // todo error
        }
    };
    {
        // await any other writers first!
        let rlock = ctx.data.read().await;
        let inner = rlock.get::<GameSuggestions>().expect("no suggestions read data");
        if let Some(existing) = inner.get(&gid) { // have existing suggestions
            let tmp = suggestion.title.to_ascii_lowercase();
            if (*existing).iter().any(|s| s.title().to_ascii_lowercase() == tmp) {
                msg.reply(ctx, "This game has been suggested already, thanks!").await?;
                return Ok(());
            }
        }
    }
    // add to suggestions 
    {
        let mut wlock = ctx.data.write().await;
        let mut inner = wlock.get_mut::<GameSuggestions>().expect("no suggestions write data");
        let suggestion = Suggestion::PlainText(msg.author.id, suggestion);
        if let Some(existing) = inner.get_mut(&gid) { // add to existing suggestions
            existing.push(suggestion);
        } else {
            let suggestions = vec![ suggestion ];
            inner.insert(gid, suggestions);
        }
    }
    msg.reply(ctx, response).await?;
    Ok(())
}
async fn add_steam_suggestion(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    // suggest steam 01234
    // suggest steam "some title"
    // "suggest steam" is already removed
    let gid = match msg.guild_id {
        Some(g) => g,
        None => {
            msg.reply(ctx, "Cannot add suggestion to non-guild channel.").await?;
            return Ok(()); // todo error
        }
    };
    let suggestion = match args.single_quoted::<String>() {
        Ok(t) => t.trim().to_string(),
        Err(_) => {
            msg.reply(ctx, "No id or name provided").await?;
            return Ok(());
        },
    };
    let app = { // read-lock to find game from steam apps and see about matches
        let rlock = ctx.data.read().await;
        let steam_inner = rlock.get::<steam::Client>().expect("no global steam::Client");
        let app = match suggestion.parse::<u32>() {
            Ok(id) => { // parsed as int, use as id
                match (**steam_inner).lock().await.game_by_id(id).await {
                    Ok(a) => a,
                    Err(_) => {
                        msg.reply(ctx, "Invalid Steam Id requested.").await?;
                        return Ok(());
                    }
                }
            },
            Err(_) => { // wasnt an integer, treat as title
                match (**steam_inner).lock().await.game_by_name(&suggestion).await {
                    Ok(a) => a,
                    Err(_) => {
                        msg.reply(ctx, "Invalid Steam Name requested.").await?;
                        return Ok(());
                    }
                }
            }
        };
        let app = Suggestion::Steam(
            msg.author.id,
            app
        );
        // await any other writers first!
        let inner = rlock.get::<GameSuggestions>().expect("no suggestions read data");
        if let Some(existing) = inner.get(&gid) {
            if (*existing).iter().any(|sug| *sug == app) {
                msg.reply(ctx, "This game has been suggested already, thanks!").await?;
                return Ok(());
            }
        }
        app
    };
    // add to suggestions 
    {
        let mut wlock = ctx.data.write().await;
        let mut inner = wlock.get_mut::<GameSuggestions>().expect("no suggestions write data");
        if let Some(existing) = inner.get_mut(&gid) {
            existing.push(app);
        } else {
            let apps = vec![ app ];
            inner.insert(gid, apps);
        }
    }
    msg.reply(ctx, "added game").await?;
    Ok(())
}

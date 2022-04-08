use std::fmt;
use std::sync::Arc;
use tokio::sync::RwLock;

use serenity::prelude::*;
use serenity::prelude::SerenityError;
use serenity::client::Context;
use serenity::model::channel::{ ChannelType,  MessageType};

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

#[derive(Debug,PartialEq,Hash)]
pub struct Suggestion {
    pub user: UserId,
    pub title: String,
    pub genre: Option<String>,
    pub url: Option<String>,
}
impl Default for Suggestion {
    fn default() -> Self {
        Suggestion {
            user: UserId(0),
            title: String::new(),
            genre: None, 
            url: None
        }
    }
}
impl fmt::Display for Suggestion {
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
pub struct GameSuggestions;
impl TypeMapKey for GameSuggestions {
    type Value = Vec<Suggestion>;
}

#[group]
#[commands(add_suggestions, list_suggestions, remove_suggestion)]
pub struct Suggestions;
// ~add_game title genre url
#[command]
#[aliases("suggest")]
async fn add_suggestions(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let mut suggestion = Suggestion::default();
    let mut response = String::new();
    
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
    {
        // await any other writers first!
        let rlock = ctx.data.read().await;
        let inner = rlock.get::<GameSuggestions>().expect("no suggestions read data");
        let tmp = suggestion.title.to_ascii_lowercase();
        if (*inner).iter().any(|s| s.title.to_ascii_lowercase() == tmp) {
            msg.reply(ctx, "This game has been suggested already, thanks!").await?;
            return Ok(());
        }
    }
    suggestion.user = msg.author.id;
    // add to suggestions 
    {
        let mut wlock = ctx.data.write().await;
        let mut inner = wlock.get_mut::<GameSuggestions>().expect("no suggestions write data");
        inner.push(suggestion);
    }
    msg.reply(ctx, response).await?;
    Ok(())
}
#[command]
#[aliases("suggestions")]
async fn list_suggestions(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let mut response = MessageBuilder::new();
    response.push_line("Currently suggested games");
    {
        let rlock = ctx.data.read().await;
        let inner = rlock.get::<GameSuggestions>().expect("no suggestions read data");
        for suggest in inner.iter() {
            response.push_bold_safe("Title: ")
                .push(&suggest.title);
            if let Some(genre) = &suggest.genre {
                response.push_bold_safe(" Genre: ")
                    .push(genre);
            }
            if let Some(url) = &suggest.url {
                response.push_bold_safe("Url: ")
                    .push(url);
            }
            response.push_line("");
        }
    }
    msg.reply(ctx, response.build()).await?;
    Ok(())
}
#[command]
async fn remove_suggestion(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let mut suggestion = Suggestion::default();
    let mut response = MessageBuilder::new();
    
    suggestion.user = msg.author.id;
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
    let tmp = suggestion.title.to_ascii_lowercase();
    { // read lock
        let rlock = ctx.data.read().await;
        let inner = rlock.get::<GameSuggestions>().expect("no suggestions read data");
        
        match (*inner).iter()
            .position(|s| s.title.to_ascii_lowercase() == tmp) {
            Some(i) => {
                if (*inner)[i].user == suggestion.user {
                    idx = i;
                }
            },
            None => {
                let _ = response.push("No suggestion with that title.");
            },
        };
    }
    { // write lock
        if idx == std::usize::MAX {
            response.push_bold("Cannot remove suggestion (Not Owner): ");
        } else {
            let mut wlock = ctx.data.write().await;
            let mut winner = wlock.get_mut::<GameSuggestions>().expect("no suggestions write data");
            let s = winner.swap_remove(idx);
            response.push("Removed suggestion: ");
        }
    }
    response.push_line(suggestion.title);
    msg.reply(ctx, response.build()).await?;
    Ok(())
}

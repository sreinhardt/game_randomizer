use std::{
    env,
    collections::HashSet,
    sync::Arc
};

use tokio::sync::RwLock;
use clap::{App as ClapApp, Arg, crate_version, SubCommand, ArgMatches};

use serenity::{
    prelude::*,
    model::{
        channel::Message,
        prelude::*,
    },
    async_trait,
    client::{
        Client, Context, EventHandler,
        bridge::gateway::ShardManager
    },
    http::Http,
    framework::standard::{
        StandardFramework,
        CommandResult,
        macros::{
            command,
            group
        }
    }
};

mod config;
mod events;
mod commands;
mod endpoints;

use crate::commands::{GENERAL_GROUP, SUGGESTIONS_GROUP, GameSuggestions};
use crate::events::Handler;
use crate::endpoints::steam;
use crate::config::Config;

pub struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

#[tokio::main]
async fn main() {
    let config = match Config::from_file("./config.toml") {
        Ok(c) => c,
        Err(_) => panic!{"Unable to read config"}
    };
    
    // access bot owners to restrict commands
    let http = Http::new_with_token(&config.discord);
    let (owners, _bot_id) = match http.get_current_application_info().await {
        Ok(info) => {
            let mut owners = HashSet::new();
            owners.insert(info.owner.id);
            (owners, info.id)
        },
        Err(why) => panic!("Could not access application info: {:?}", why),
    };
    // setup command framework
    let framework = StandardFramework::new()
        .configure(|c| c.owners(owners).prefix("~"))
        .group(&GENERAL_GROUP)
        .group(&SUGGESTIONS_GROUP);
    // Login with a bot token from the environment
    let mut client = Client::builder(config.discord)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("Error creating client");
    // add shared data
    let mut steamclient = steam::Client::with_api_key(&config.steam);
    steamclient.fill_app_list().await.expect("Could not fill steam app list");
    {
        let mut data = client.data.write().await;
        data.insert::<GameSuggestions>(GameSuggestions::new());
        data.insert::<ShardManagerContainer>(client.shard_manager.clone());
        data.insert::<steam::Client>(Arc::new(Mutex::new(steamclient)));
    }
    // spawn shard manager threads
    let shard_manager = client.shard_manager.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.expect("Could not register ctrl+c handler");
        shard_manager.lock().await.shutdown_all().await;
    });
    // start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }
}


/*
~add_suggestion steam 730
~add_suggestion steam 220
~add_suggestion steam 440

~remove_suggestion steam 730
~suggestions
*/

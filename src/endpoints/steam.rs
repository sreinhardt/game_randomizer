use std::sync::Arc;
use std::ops::Deref;
use std::fmt;
use rsteam;
use serenity::prelude::*;

pub struct App {
    pub id: u32,
    pub name: String,
}
impl App {
    pub fn url(&self) -> String {
        let url = format!("https://store.steampowered.com/app/{}/", self.id);
        url
    }
}
impl fmt::Display for App {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!{f, "[{}](https://store.steampowered.com/app/{}/)", self.name, self.id.to_string()}
    }
}
impl From<&rsteam::steam_apps::App> for App {
    fn from(app: &rsteam::steam_apps::App) -> Self {
        App{
            id: app.id,
            name: app.name.clone()
        }
    }
}

pub struct Apps (Vec<App>);
impl Apps {
    pub fn new(apps: Vec<App>) -> Self {
        Apps( apps )
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn find_by_id(&self, id: u32) -> Result<App, ()> {
        match self.0.iter().find(|a| a.id == id) {
            Some(app) => Ok( App{
                                id: app.id,
                                name: app.name.clone()
                        }),
            None => Err( () ), // TODO: ERROR NOT FOUND
        }
    }
    pub fn find_by_name(&self, name: &str) -> Result<App, ()> {
        match self.0.iter().find(|a| a.name == name) {
            Some(app) => Ok( App{
                                id: app.id,
                                name: app.name.clone()
                        }),
            None => Err( () ), // TODO: ERROR NOT FOUND
        }
    }
}
impl Default for Apps {
    fn default() -> Self {
        Apps( Vec::with_capacity(5000) )
    }
}
impl From<Vec<rsteam::steam_apps::App>> for Apps {
    fn from(apps: Vec<rsteam::steam_apps::App>) -> Self {
        let apps: Vec<App> = apps.iter().map(|a| a.into()).collect();
        Apps(apps)
    }
}
impl fmt::Display for Apps {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let rst = write!{f, "Application List"};
        for app in self.0.iter() {
            let rst = match write!{f, "{}\n", app} {
                r@Ok(_) => r,
                e@Err(_) => return e,
            };
        }
        rst
    }
}

pub struct Client {
    pub client: rsteam::SteamClient,
    pub apps: Apps,
}
impl Client {
    pub fn with_api_key(key: &str) -> Self {
        Client {
            client: rsteam::SteamClient::with_api_key(key),
            apps: Apps::default()
        }
    }
    pub async fn fill_app_list(&mut self) -> Result<(), ()> {
        match self.client.get_app_list().await {
            Ok(apps) => {
                let apps:Apps = apps.into();
                self.apps = apps;
                Ok(())
            },
            Err(_) => {
                Err(()) // TODO: ERROR FAILED GATHER
            }
        }
    }
    pub async fn game_by_id(&self, id:u32) -> Result<App, ()> {
        if self.apps.is_empty() {
            return Err(()); // TODO: ERROR LIST EMPTY
        }
        self.apps.find_by_id(id)
    }
    pub async fn game_by_name(&self, name:&str) -> Result<App, ()> {
        if self.apps.is_empty() {
            return Err(()); // TODO: ERROR LIST EMPTY
        }
        self.apps.find_by_name(name)
    }
}
impl Default for Client {
    fn default() -> Self {
        Client {
            client: rsteam::SteamClient::new(),
            .. Default::default()
        }
    }
}

impl TypeMapKey for Client {
    type Value = Arc<Mutex<Client>>;
}

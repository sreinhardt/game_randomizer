use serenity::async_trait;
use serenity::prelude::*;
use serenity::{
    model::{
        event::ResumedEvent,
        gateway::Ready,
    }
};

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is ready and connected!", ready.user.name);
    }
    async fn resume(&self, _: Context, resume: ResumedEvent) {
        println!("Resuming events: {:?}", resume.trace);
    }
}

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use serenity::{
    prelude::*,
    model::{
        id::{ChannelId, MessageId}
    }
};

pub struct StrawPollKey;

impl TypeMapKey for StrawPollKey {
  type Value = Arc<Mutex<StrawPollMap>>;
}

pub type StrawPollMap = HashMap<(ChannelId, MessageId), StrawPoll>;
pub struct StrawPoll {
  pub question: String,
  pub answers: Vec<String>,
  pub answerers: Vec<usize>,
}
impl StrawPoll {
    pub fn set_question<Q>(&mut self, question: Q) where Q: Into<String> {
        self.question = question.into();
    }
    pub fn add_answer<A>(&mut self, answer: A) where A: Into<String> {
        self.answers.push( answer.into() );
    }
    pub fn remove_answer(&mut self, answer: usize) -> Option<()> {
        if self.answers.len() <= answer  {
            return None;
        }
        self.answers.remove(answer);
        Some( () )
    }
}

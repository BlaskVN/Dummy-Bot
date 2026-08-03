pub mod ban;
pub mod kick;
pub mod purge;

use crate::{Data, Error};

pub fn all() -> Vec<poise::Command<Data, Error>> {
    vec![kick::kick(), ban::ban(), purge::purge()]
}

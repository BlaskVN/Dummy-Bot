pub mod activity;
pub mod configuration;
pub mod donation;
pub mod general;
pub mod moderation;
pub mod presence;
pub mod valorant;
pub mod voice;
pub mod word_puzzle;

use crate::{Data, Error};

pub fn all() -> Vec<poise::Command<Data, Error>> {
    let mut commands = general::all();
    commands.push(activity::activity());
    commands.extend(moderation::all());
    commands.extend(configuration::all());
    commands.extend(voice::all());
    commands.push(donation::donation());
    commands.push(presence::presence());
    commands.push(word_puzzle::word_puzzle());
    commands.push(valorant::valorant());
    commands
}

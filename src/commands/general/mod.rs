pub mod botinfo;
pub mod donate;
pub mod ping;
pub mod serverinfo;

use crate::{Data, Error};

pub fn all() -> Vec<poise::Command<Data, Error>> {
    vec![
        ping::ping(),
        botinfo::botinfo(),
        serverinfo::serverinfo(),
        donate::donate(),
    ]
}

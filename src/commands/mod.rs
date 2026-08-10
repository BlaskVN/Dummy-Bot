pub mod activity;
pub mod configuration;
pub mod donation;
pub mod general;
pub mod moderation;
pub mod presence;
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
    commands
}

#[cfg(test)]
mod tests {
    use super::all;

    #[test]
    fn registers_representative_slash_commands_without_prefix_actions() {
        fn inspect(command: &poise::Command<crate::Data, crate::Error>) {
            let description = command.description.as_deref().unwrap_or_default();
            assert!(
                !description.is_empty() && description != "A slash command",
                "/{} has a missing or default description",
                command.name
            );
            assert!(
                command.slash_action.is_some(),
                "{} is not slash-enabled",
                command.name
            );
            assert!(
                command.prefix_action.is_none(),
                "{} still has prefix dispatch",
                command.name
            );
            for parameter in &command.parameters {
                let description = parameter.description.as_deref().unwrap_or_default();
                assert!(
                    !description.is_empty() && description != "A slash command parameter",
                    "/{} parameter {} has a missing or default description",
                    command.name,
                    parameter.name
                );
            }
            for subcommand in &command.subcommands {
                inspect(subcommand);
            }
        }

        let commands = all();
        for command in &commands {
            inspect(command);
        }
        let names = commands
            .iter()
            .map(|command| command.name.as_str())
            .collect::<Vec<_>>();
        for representative in ["ping", "ban", "settings", "presence", "connect"] {
            assert!(names.contains(&representative), "missing /{representative}");
        }
        assert!(!names.contains(&"setprefix"));
        assert!(!names.contains(&"valorant"));
    }
}

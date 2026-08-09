use crate::{Data, database};
use poise::serenity_prelude as serenity;

pub async fn handle_guild_delete(incomplete: &serenity::UnavailableGuild, data: &Data) {
    if !is_permanent_removal(incomplete.unavailable) {
        return;
    }
    match database::delete_guild_data(&data.db_pool, incomplete.id).await {
        Ok(()) => {
            tracing::info!(guild = %incomplete.id, "Deleted data after permanent bot removal")
        }
        Err(error) => {
            tracing::error!(guild = %incomplete.id, %error, "Failed to delete data after permanent bot removal")
        }
    }
}

fn is_permanent_removal(unavailable: bool) -> bool {
    !unavailable
}

#[cfg(test)]
mod tests {
    use super::is_permanent_removal;

    #[test]
    fn preserves_data_during_temporary_unavailability() {
        assert!(!is_permanent_removal(true));
        assert!(is_permanent_removal(false));
    }
}

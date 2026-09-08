use anyhow::{Context, Result};
use poise::serenity_prelude::{GuildId, UserId};
use sqlx::SqlitePool;

use super::riot_api::RiotRegion;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkedRiotAccount {
    pub user_id: UserId,
    pub puuid: String,
    pub game_name: String,
    pub tag_line: String,
    pub region: RiotRegion,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(sqlx::FromRow)]
struct LinkedRiotAccountRow {
    user_id: String,
    puuid: String,
    game_name: String,
    tag_line: String,
    region: String,
    created_at: String,
    updated_at: String,
}

/// Retrieve a user's global linked Riot account.
pub async fn get_linked_account(
    pool: &SqlitePool,
    user_id: UserId,
) -> Result<Option<LinkedRiotAccount>> {
    let row = sqlx::query_as::<_, LinkedRiotAccountRow>(
        "SELECT user_id, puuid, game_name, tag_line, region, created_at, updated_at
         FROM valorant_linked_account
         WHERE user_id = ?",
    )
    .bind(user_id.to_string())
    .fetch_optional(pool)
    .await
    .context("Failed to query valorant_linked_account")?;

    match row {
        Some(r) => {
            let region = RiotRegion::try_parse(&r.region).unwrap_or(RiotRegion::Ap);
            Ok(Some(LinkedRiotAccount {
                user_id,
                puuid: r.puuid,
                game_name: r.game_name,
                tag_line: r.tag_line,
                region,
                created_at: r.created_at,
                updated_at: r.updated_at,
            }))
        }
        None => Ok(None),
    }
}

/// Link or update a user's global Riot account.
pub async fn set_linked_account(
    pool: &SqlitePool,
    user_id: UserId,
    puuid: &str,
    game_name: &str,
    tag_line: &str,
    region: RiotRegion,
) -> Result<LinkedRiotAccount> {
    let user_str = user_id.to_string();
    let region_str = region.as_str();

    sqlx::query(
        "INSERT INTO valorant_linked_account (user_id, puuid, game_name, tag_line, region)
         VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(user_id) DO UPDATE SET
             puuid = excluded.puuid,
             game_name = excluded.game_name,
             tag_line = excluded.tag_line,
             region = excluded.region,
             updated_at = CURRENT_TIMESTAMP",
    )
    .bind(&user_str)
    .bind(puuid)
    .bind(game_name)
    .bind(tag_line)
    .bind(region_str)
    .execute(pool)
    .await
    .context("Failed to save valorant_linked_account")?;

    get_linked_account(pool, user_id)
        .await?
        .context("Failed to reload newly saved linked account")
}

/// Unlink a user's global Riot account and remove their visibility preferences across guilds.
pub async fn remove_linked_account(pool: &SqlitePool, user_id: UserId) -> Result<bool> {
    let user_str = user_id.to_string();
    let mut tx = pool.begin().await?;

    sqlx::query("DELETE FROM valorant_guild_visibility WHERE user_id = ?")
        .bind(&user_str)
        .execute(&mut *tx)
        .await
        .context("Failed to clear valorant_guild_visibility on unlink")?;

    let rows = sqlx::query("DELETE FROM valorant_linked_account WHERE user_id = ?")
        .bind(&user_str)
        .execute(&mut *tx)
        .await
        .context("Failed to delete valorant_linked_account")?
        .rows_affected();

    tx.commit().await?;
    Ok(rows > 0)
}

/// Check if a user has explicitly enabled Guild Profile Visibility in a specific guild.
/// Per CONTEXT.md: Linking alone leaves the profile hidden in every Guild (defaults to false).
pub async fn get_guild_visibility(
    pool: &SqlitePool,
    guild_id: GuildId,
    user_id: UserId,
) -> Result<bool> {
    let row = sqlx::query_scalar::<_, i64>(
        "SELECT visible FROM valorant_guild_visibility WHERE guild_id = ? AND user_id = ?",
    )
    .bind(guild_id.to_string())
    .bind(user_id.to_string())
    .fetch_optional(pool)
    .await
    .context("Failed to query valorant_guild_visibility")?;

    Ok(row.map(|v| v == 1).unwrap_or(false))
}

/// Set a user's Guild Profile Visibility consent for a specific guild.
pub async fn set_guild_visibility(
    pool: &SqlitePool,
    guild_id: GuildId,
    user_id: UserId,
    visible: bool,
) -> Result<()> {
    let val = if visible { 1 } else { 0 };
    sqlx::query(
        "INSERT INTO valorant_guild_visibility (guild_id, user_id, visible)
         VALUES (?, ?, ?)
         ON CONFLICT(guild_id, user_id) DO UPDATE SET
             visible = excluded.visible,
             updated_at = CURRENT_TIMESTAMP",
    )
    .bind(guild_id.to_string())
    .bind(user_id.to_string())
    .bind(val)
    .execute(pool)
    .await
    .context("Failed to set valorant_guild_visibility")?;

    Ok(())
}

/// List all linked accounts in a guild that have enabled Guild Profile Visibility.
pub async fn list_guild_visible_accounts(
    pool: &SqlitePool,
    guild_id: GuildId,
) -> Result<Vec<LinkedRiotAccount>> {
    let rows = sqlx::query_as::<_, LinkedRiotAccountRow>(
        "SELECT a.user_id, a.puuid, a.game_name, a.tag_line, a.region, a.created_at, a.updated_at
         FROM valorant_linked_account a
         INNER JOIN valorant_guild_visibility v ON a.user_id = v.user_id
         WHERE v.guild_id = ? AND v.visible = 1",
    )
    .bind(guild_id.to_string())
    .fetch_all(pool)
    .await
    .context("Failed to query guild visible valorant accounts")?;

    let accounts = rows
        .into_iter()
        .filter_map(|r| {
            let user_id = r.user_id.parse::<u64>().ok().map(UserId::new)?;
            let region = RiotRegion::try_parse(&r.region).unwrap_or(RiotRegion::Ap);
            Some(LinkedRiotAccount {
                user_id,
                puuid: r.puuid,
                game_name: r.game_name,
                tag_line: r.tag_line,
                region,
                created_at: r.created_at,
                updated_at: r.updated_at,
            })
        })
        .collect();

    Ok(accounts)
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_test_pool() -> (SqlitePool, std::path::PathBuf) {
        let directory = std::env::temp_dir().join(format!(
            "dummy-bot-valorant-storage-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let url = format!("sqlite:{}/bot.db?mode=rwc", directory.display());
        let pool = crate::database::init_db(&url, &directory).await.unwrap();
        (pool, directory)
    }

    #[tokio::test]
    async fn linked_account_crud_and_visibility_isolation() {
        let (pool, dir) = setup_test_pool().await;
        let user1 = UserId::new(1001);
        let user2 = UserId::new(1002);
        let guild1 = GuildId::new(5001);
        let guild2 = GuildId::new(5002);

        // Initially no linked account
        assert!(get_linked_account(&pool, user1).await.unwrap().is_none());

        // Link user1
        let acc1 = set_linked_account(&pool, user1, "puuid-1001", "Jett", "WIND", RiotRegion::Ap)
            .await
            .unwrap();
        assert_eq!(acc1.user_id, user1);
        assert_eq!(acc1.puuid, "puuid-1001");
        assert_eq!(acc1.game_name, "Jett");
        assert_eq!(acc1.tag_line, "WIND");
        assert_eq!(acc1.region, RiotRegion::Ap);

        // Visibility is false by default in all guilds
        assert!(!get_guild_visibility(&pool, guild1, user1).await.unwrap());
        assert!(!get_guild_visibility(&pool, guild2, user1).await.unwrap());
        assert!(
            list_guild_visible_accounts(&pool, guild1)
                .await
                .unwrap()
                .is_empty()
        );

        // Enable visibility in guild1 only
        set_guild_visibility(&pool, guild1, user1, true)
            .await
            .unwrap();
        assert!(get_guild_visibility(&pool, guild1, user1).await.unwrap());
        assert!(!get_guild_visibility(&pool, guild2, user1).await.unwrap());

        let visible_guild1 = list_guild_visible_accounts(&pool, guild1).await.unwrap();
        assert_eq!(visible_guild1.len(), 1);
        assert_eq!(visible_guild1[0].user_id, user1);
        assert!(
            list_guild_visible_accounts(&pool, guild2)
                .await
                .unwrap()
                .is_empty()
        );

        // Link user2 and enable visibility in guild1 as well
        set_linked_account(&pool, user2, "puuid-1002", "Sova", "DART", RiotRegion::Na)
            .await
            .unwrap();
        set_guild_visibility(&pool, guild1, user2, true)
            .await
            .unwrap();

        let visible_both = list_guild_visible_accounts(&pool, guild1).await.unwrap();
        assert_eq!(visible_both.len(), 2);

        // Disable user1 visibility
        set_guild_visibility(&pool, guild1, user1, false)
            .await
            .unwrap();
        assert!(!get_guild_visibility(&pool, guild1, user1).await.unwrap());
        let visible_after_disable = list_guild_visible_accounts(&pool, guild1).await.unwrap();
        assert_eq!(visible_after_disable.len(), 1);
        assert_eq!(visible_after_disable[0].user_id, user2);

        // Unlink user2 removes account and visibility
        assert!(remove_linked_account(&pool, user2).await.unwrap());
        assert!(get_linked_account(&pool, user2).await.unwrap().is_none());
        assert!(
            list_guild_visible_accounts(&pool, guild1)
                .await
                .unwrap()
                .is_empty()
        );

        pool.close().await;
        let _ = tokio::fs::remove_dir_all(&dir).await;
    }
}

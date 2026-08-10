use anyhow::Result;
use poise::serenity_prelude::{GuildId, UserId};
use sqlx::SqlitePool;

pub async fn is_opted_out(pool: &SqlitePool, guild_id: GuildId, user_id: UserId) -> Result<bool> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM activity_opt_out WHERE guild_id = ? AND user_id = ?)",
    )
    .bind(guild_id.to_string())
    .bind(user_id.to_string())
    .fetch_one(pool)
    .await?)
}

pub async fn opt_out(pool: &SqlitePool, guild_id: GuildId, user_id: UserId) -> Result<()> {
    let mut transaction = pool.begin().await?;
    sqlx::query(
        "INSERT INTO activity_opt_out (guild_id, user_id) VALUES (?, ?) ON CONFLICT DO NOTHING",
    )
    .bind(guild_id.to_string())
    .bind(user_id.to_string())
    .execute(&mut *transaction)
    .await?;
    for table in [
        "activity_attendance_interval",
        "activity_attendance",
        "activity_completion",
        "activity_member_game_aggregate",
        "activity_member_aggregate",
        "activity_reward_grant",
    ] {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DELETE FROM {table} WHERE guild_id = ? AND user_id = ?"
        )))
        .bind(guild_id.to_string())
        .bind(user_id.to_string())
        .execute(&mut *transaction)
        .await?;
    }
    transaction.commit().await?;
    Ok(())
}

pub async fn opt_in(pool: &SqlitePool, guild_id: GuildId, user_id: UserId) -> Result<()> {
    sqlx::query("DELETE FROM activity_opt_out WHERE guild_id = ? AND user_id = ?")
        .bind(guild_id.to_string())
        .bind(user_id.to_string())
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{is_opted_out, opt_in, opt_out};
    use crate::database::init_db;
    use poise::serenity_prelude::{GuildId, UserId};

    #[tokio::test]
    async fn deletes_one_guild_and_reenters_empty_idempotently() {
        let directory =
            std::env::temp_dir().join(format!("dummy-bot-opt-out-test-{}", std::process::id()));
        let pool = init_db(
            &format!("sqlite:{}/bot.db?mode=rwc", directory.display()),
            &directory,
        )
        .await
        .unwrap();
        for guild in [1, 2] {
            sqlx::query("INSERT INTO activity_member_game_aggregate (guild_id, user_id, game_key, play_minutes, session_credits) VALUES (?, '3', 'game', 60, 1)")
                .bind(guild.to_string()).execute(&pool).await.unwrap();
            sqlx::query("INSERT INTO activity_member_aggregate (guild_id, user_id, play_minutes, session_credits) VALUES (?, '3', 60, 1)")
                .bind(guild.to_string()).execute(&pool).await.unwrap();
            sqlx::query("INSERT INTO activity_completion (guild_id, source_key, user_id, game_key, play_minutes, session_credit) VALUES (?, 'source', '3', 'game', 60, 1)")
                .bind(guild.to_string()).execute(&pool).await.unwrap();
        }
        opt_out(&pool, GuildId::new(1), UserId::new(3))
            .await
            .unwrap();
        opt_out(&pool, GuildId::new(1), UserId::new(3))
            .await
            .unwrap();
        assert!(
            is_opted_out(&pool, GuildId::new(1), UserId::new(3))
                .await
                .unwrap()
        );
        let other: i64 = sqlx::query_scalar("SELECT play_minutes FROM activity_member_aggregate WHERE guild_id = '2' AND user_id = '3'")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(other, 60);
        opt_in(&pool, GuildId::new(1), UserId::new(3))
            .await
            .unwrap();
        opt_in(&pool, GuildId::new(1), UserId::new(3))
            .await
            .unwrap();
        assert!(
            !is_opted_out(&pool, GuildId::new(1), UserId::new(3))
                .await
                .unwrap()
        );
        let restored: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM activity_member_aggregate WHERE guild_id = '1' AND user_id = '3'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(restored, 0);
        pool.close().await;
        std::fs::remove_dir_all(directory).unwrap();
    }
}

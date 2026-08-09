use poise::serenity_prelude::{GuildId, UserId};
use rust_discord_bot::automod::{
    ExecutionMetadata, maybe_open_suggestion, observer_enabled, record_execution,
    set_observer_enabled,
};
use rust_discord_bot::database::{delete_guild_data, init_db, load_donation_config};
use rust_discord_bot::message_log_health::{MessageLogHealth, reconcile};
use rust_discord_bot::moderation_cases::{
    ModerationAction, create_case, get_case, list_cases, void_case,
};

#[tokio::test]
async fn isolates_v1_1_data_on_fresh_and_upgraded_databases() {
    exercise(false).await;
    exercise(true).await;
}

async fn exercise(upgrade_from_v1: bool) {
    let directory = std::env::temp_dir().join(format!(
        "dummy-bot-v1-1-isolation-{}-{upgrade_from_v1}",
        std::process::id()
    ));
    let url = format!("sqlite:{}/bot.db?mode=rwc", directory.display());
    let pool = if upgrade_from_v1 {
        std::fs::create_dir_all(&directory).unwrap();
        let pool = sqlx::SqlitePool::connect(&url).await.unwrap();
        sqlx::raw_sql(include_str!("../migrations/0001_initial.sql"))
            .execute(&pool)
            .await
            .unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();
        pool
    } else {
        init_db(&url, &directory).await.unwrap()
    };
    let first = GuildId::new(1);
    let second = GuildId::new(2);
    sqlx::raw_sql(
        "INSERT INTO guild_timezone (guild_id, iana_name) VALUES ('1', 'UTC'), ('2', 'Asia/Bangkok');
         INSERT INTO moderation_channel_config (guild_id, channel_id) VALUES ('1', '11'), ('2', '22');
         INSERT INTO message_log_config (guild_id, log_channel_id, enabled) VALUES ('1', '11', 1), ('2', '22', 1);
         INSERT INTO donation_config (id, message) VALUES (1, 'global');",
    )
    .execute(&pool)
    .await
    .unwrap();
    create_case(
        &pool,
        first,
        ModerationAction::Warn,
        UserId::new(3),
        UserId::new(4),
        "first",
        None,
    )
    .await
    .unwrap();
    create_case(
        &pool,
        second,
        ModerationAction::Warn,
        UserId::new(3),
        UserId::new(4),
        "second",
        None,
    )
    .await
    .unwrap();
    set_observer_enabled(&pool, first, true).await.unwrap();
    set_observer_enabled(&pool, second, false).await.unwrap();
    for message in 1..=3 {
        let metadata = ExecutionMetadata {
            guild_id: first,
            user_id: 3,
            rule_id: 9,
            action_type: 1,
            channel_id: Some(11),
            message_id: Some(message),
            alert_message_id: None,
        };
        record_execution(&pool, &metadata, 100 + message as i64)
            .await
            .unwrap();
    }
    assert!(
        maybe_open_suggestion(&pool, first, 3, 9, 103)
            .await
            .unwrap()
            .is_some()
    );
    assert_eq!(
        reconcile(&pool, first, false).await.unwrap(),
        (MessageLogHealth::Degraded, true)
    );
    assert_eq!(
        reconcile(&pool, second, true).await.unwrap(),
        (MessageLogHealth::Healthy, false)
    );

    assert_eq!(
        get_case(&pool, first, 1).await.unwrap().unwrap().reason,
        "first"
    );
    assert_eq!(
        get_case(&pool, second, 1).await.unwrap().unwrap().reason,
        "second"
    );
    assert_eq!(
        list_cases(&pool, first, None, 0, 10).await.unwrap().len(),
        1
    );
    assert!(
        void_case(&pool, first, 1, UserId::new(4), "mistake")
            .await
            .unwrap()
    );
    assert_eq!(
        get_case(&pool, second, 1).await.unwrap().unwrap().status,
        "active"
    );
    sqlx::query("UPDATE guild_timezone SET iana_name = 'Europe/London' WHERE guild_id = '1'")
        .execute(&pool)
        .await
        .unwrap();
    let second_timezone: String =
        sqlx::query_scalar("SELECT iana_name FROM guild_timezone WHERE guild_id = '2'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(second_timezone, "Asia/Bangkok");
    assert!(observer_enabled(&pool, first).await.unwrap());
    assert!(!observer_enabled(&pool, second).await.unwrap());

    delete_guild_data(&pool, first).await.unwrap();
    assert!(get_case(&pool, first, 1).await.unwrap().is_none());
    assert!(get_case(&pool, second, 1).await.unwrap().is_some());
    assert_eq!(
        load_donation_config(&pool)
            .await
            .unwrap()
            .unwrap()
            .message
            .as_deref(),
        Some("global")
    );
    pool.close().await;
    std::fs::remove_dir_all(directory).unwrap();
}

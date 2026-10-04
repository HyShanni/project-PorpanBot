#![allow(deprecated)]

mod commands;
mod utils;
mod handler;
mod types;
mod api;
pub mod db;
pub mod services;

use std::env;
use dotenvy::dotenv;
use serenity::prelude::GatewayIntents;
use songbird::SerenityInit;
use tracing::{error, info};
use sqlx::postgres::PgPoolOptions;

use crate::types::Data;
use crate::commands::{
    music::*,
    moderation::*,
    utility::*,
    admin::*,
};

#[tokio::main]
async fn main() {
    let start_time = std::time::Instant::now();
    tracing_subscriber::fmt::init();
    dotenv().ok();

    let token = env::var("DISCORD_TOKEN").expect("Expected DISCORD_TOKEN in environment");
    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| env::var("SUPABASE_DATABASE_URL").expect("Expected DATABASE_URL"));

    let pool = match PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            eprintln!("CRITICAL ERROR: Failed to connect to database! URL: {}", database_url);
            eprintln!("Error details: {:?}", e);
            std::process::exit(1);
        }
    };
    info!("Connected to Supabase PostgreSQL");
    info!("Booting Khivella Core v1.0.1 - Access and Banner fixes applied");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS khivella_memory (
            user_id TEXT PRIMARY KEY,
            username TEXT NOT NULL,
            favorite_game TEXT,
            favorite_food TEXT,
            about_user TEXT,
            relationship_score INT DEFAULT 0,
            last_interaction TIMESTAMPTZ DEFAULT NOW()
        );"
    ).execute(&pool).await.unwrap_or_else(|e| {
        error!("Failed to initialize khivella_memory table: {:?}", e);
        Default::default()
    });

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS khivella_access (
            discord_id TEXT PRIMARY KEY,
            role_name TEXT NOT NULL,
            added_at TIMESTAMPTZ DEFAULT NOW()
        );"
    ).execute(&pool).await.unwrap_or_else(|e| {
        error!("Failed to initialize khivella_access table: {:?}", e);
        Default::default()
    });

    // Grant access to Supabase API roles and reload schema cache
    let _ = sqlx::query("GRANT ALL ON khivella_access TO anon, authenticated;").execute(&pool).await;
    let _ = sqlx::query("NOTIFY pgrst, 'reload schema';").execute(&pool).await;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS khivella_audit_logs (
            id SERIAL PRIMARY KEY,
            event_type TEXT NOT NULL,
            user_id TEXT,
            username TEXT,
            details TEXT,
            created_at TIMESTAMPTZ DEFAULT NOW()
        );"
    ).execute(&pool).await.unwrap_or_else(|e| {
        error!("Failed to initialize khivella_audit_logs table: {:?}", e);
        Default::default()
    });

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS khivella_autoreplies (
            id SERIAL PRIMARY KEY,
            guild_id TEXT NOT NULL,
            trigger TEXT NOT NULL,
            response TEXT,
            media_url TEXT,
            use_container BOOLEAN DEFAULT false,
            UNIQUE(guild_id, trigger)
        );"
    ).execute(&pool).await.unwrap_or_else(|e| {
        error!("Failed to initialize khivella_autoreplies table: {:?}", e);
        Default::default()
    });

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS khivella_booster (
            guild_id TEXT PRIMARY KEY,
            channel_id TEXT,
            background_url TEXT,
            style TEXT DEFAULT 'plain text',
            text TEXT DEFAULT 'Thank you {username} for boosting the server!'
        );"
    ).execute(&pool).await.unwrap_or_else(|e| {
        error!("Failed to initialize khivella_booster table: {:?}", e);
        Default::default()
    });

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS khivella_sticky (
            guild_id TEXT NOT NULL,
            channel_id TEXT NOT NULL,
            message TEXT NOT NULL,
            last_message_id TEXT,
            UNIQUE(guild_id, channel_id)
        );"
    ).execute(&pool).await.unwrap_or_else(|e| {
        error!("Failed to initialize khivella_sticky table: {:?}", e);
        Default::default()
    });

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS khivella_galleries (
            guild_id TEXT NOT NULL,
            channel_id TEXT NOT NULL,
            emojis TEXT NOT NULL,
            UNIQUE(guild_id, channel_id)
        );"
    ).execute(&pool).await.unwrap_or_else(|e| {
        error!("Failed to initialize khivella_galleries table: {:?}", e);
        Default::default()
    });

    let intents = GatewayIntents::non_privileged() 
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::GUILD_MESSAGES;

    let chatbot_state = std::sync::Arc::new(tokio::sync::RwLock::new(true));
    let chat_history = std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));
    let clan_data = std::sync::Arc::new(tokio::sync::RwLock::new(String::new()));
    let api_chatbot_state = chatbot_state.clone();

    // Background task to fetch clan data
    let fetch_clan_data = clan_data.clone();
    tokio::spawn(async move {
        let client = reqwest::Client::new();
        loop {
            match client.get("https://4funclan.site/api/clan-data").send().await {
                Ok(res) => {
                    if let Ok(json) = res.json::<serde_json::Value>().await {
                        if json["success"].as_bool().unwrap_or(false) {
                            let mut lock = fetch_clan_data.write().await;
                            *lock = serde_json::to_string(&json["data"]).unwrap_or_default();
                            info!("Successfully fetched and updated clan data from website");
                        }
                    }
                }
                Err(e) => error!("Failed to fetch clan data: {}", e),
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await; // 1 jam
        }
    });

    let framework_pool = pool.clone();
    let api_pool = pool.clone();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                join(), leave(), play(), pause(), resume(), skip(), stop(), queue(), nowplaying(), volume(),
                kick(), ban(), unban(), purge(), timeout(), warn(), strike(),
                lock(), unlock(), slowmode(), chatbot(), status(), autoreply(),
                booster(), sticky(), restart(), checknames(), autothread(),
                ping(), userinfo(), serverinfo(), avatar(), help(),
                grab(), report(), stats(), about(), profile(), members(),
            ],
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some("ff".into()),
                ..Default::default()
            },
            event_handler: |ctx, event, framework, data| {
                Box::pin(handler::event_handler(ctx, event, framework, data))
            },
            owners: std::collections::HashSet::from([serenity::model::id::UserId::new(494169184175915019)]),
            ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                
                use serenity::all::{ActivityData, OnlineStatus};
                ctx.set_presence(Some(ActivityData::playing("StayWith4Fun")), OnlineStatus::Online);

                Ok(Data {
                    chatbot_enabled: chatbot_state,
                    db_pool: framework_pool,
                    chat_history,
                    clan_data,
                    start_time,
                })
            })
        })
        .build();

    let mut client = serenity::Client::builder(&token, intents)
        .framework(framework)
        .register_songbird()
        .await
        .expect("Error creating client");

    let cache = client.cache.clone();
    let http = client.http.clone();
    let api_task = tokio::spawn(async move {
        api::start_api_server(api_chatbot_state, cache, http, start_time, api_pool).await;
    });

    if let Err(why) = client.start().await {
        error!("CRITICAL ERROR: Bot failed to connect to Discord! Error: {:?}", why);
        eprintln!("CRITICAL ERROR: Bot failed to connect to Discord! Error: {:?}", why);
        eprintln!("Keeping the process alive so the API server can still respond...");
        let _ = api_task.await;
    }
}

use axum::{
    routing::{get, post},
    Router,
    extract::State,
    Json,
};
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;
use tokio::sync::RwLock;
use std::time::Instant;
use sysinfo::System;
use serenity::all::{Cache, Http};
use serenity::model::id::ChannelId;

#[derive(Serialize)]
struct GuildInfo {
    id: String,
    name: String,
    member_count: u64,
}

#[derive(Serialize)]
struct SystemInfo {
    uptime_seconds: u64,
    ram_used_mb: u64,
    ram_total_mb: u64,
    cpu_cores: usize,
    os_name: String,
}

#[derive(Serialize)]
struct StatusResponse {
    status: String,
    chatbot_enabled: bool,
    guilds: Vec<GuildInfo>,
    total_members: u64,
    system: SystemInfo,
}

#[derive(Clone)]
pub struct ApiState {
    pub chatbot_enabled: Arc<RwLock<bool>>,
    pub discord_cache: Arc<Cache>,
    pub discord_http: Arc<Http>,
    pub start_time: Instant,
    pub db_pool: sqlx::PgPool,
}

async fn get_status(State(state): State<ApiState>) -> Json<StatusResponse> {
    let enabled = *state.chatbot_enabled.read().await;
    
    let mut guilds_info = Vec::new();
    let mut total_members = 0;
    
    let guild_ids = state.discord_cache.guilds();
    for guild_id in guild_ids {
        if let Some(guild) = state.discord_cache.guild(guild_id) {
            let count = guild.member_count as u64;
            guilds_info.push(GuildInfo {
                id: guild.id.to_string(),
                name: guild.name.clone(),
                member_count: count,
            });
            total_members += count;
        }
    }

    let mut sys = System::new_all();
    sys.refresh_all();
    
    let uptime = state.start_time.elapsed().as_secs();
    
    let system_info = SystemInfo {
        uptime_seconds: uptime,
        ram_used_mb: sys.used_memory() / 1048576,
        ram_total_mb: sys.total_memory() / 1048576,
        cpu_cores: sys.cpus().len(),
        os_name: System::name().unwrap_or_else(|| "Unknown OS".to_string()),
    };

    Json(StatusResponse {
        status: "online".to_string(),
        chatbot_enabled: enabled,
        guilds: guilds_info,
        total_members,
        system: system_info,
    })
}

#[derive(Deserialize)]
struct ToggleRequest {
    enabled: bool,
}

async fn toggle_chatbot(
    State(state): State<ApiState>,
    Json(payload): Json<ToggleRequest>,
) -> Json<StatusResponse> {
    *state.chatbot_enabled.write().await = payload.enabled;
    get_status(State(state)).await
}

#[derive(Deserialize)]
struct SendMessageRequest {
    channel_id: String,
    content: String,
}

#[derive(Serialize)]
struct SendMessageResponse {
    success: bool,
    error: Option<String>,
}

async fn send_message(
    State(state): State<ApiState>,
    Json(payload): Json<SendMessageRequest>,
) -> Json<SendMessageResponse> {
    if let Ok(ch_id) = payload.channel_id.parse::<u64>() {
        let channel_id = ChannelId::new(ch_id);
        let builder = serenity::builder::CreateMessage::new().content(payload.content);
        
        match channel_id.send_message(&state.discord_http, builder).await {
            Ok(_) => Json(SendMessageResponse { success: true, error: None }),
            Err(e) => Json(SendMessageResponse { success: false, error: Some(e.to_string()) }),
        }
    } else {
        Json(SendMessageResponse { success: false, error: Some("Invalid channel ID format".to_string()) })
    }
}

#[derive(Deserialize)]
struct SendEmbedField {
    name: String,
    value: String,
    inline: bool,
}

#[derive(Deserialize)]
struct SendEmbedRequest {
    channel_id: String,
    title: Option<String>,
    url: Option<String>,
    description: Option<String>,
    color: Option<String>,
    image_url: Option<String>,
    thumbnail_url: Option<String>,
    author_name: Option<String>,
    author_url: Option<String>,
    author_icon: Option<String>,
    footer_text: Option<String>,
    footer_icon: Option<String>,
    fields: Option<Vec<SendEmbedField>>,
}

async fn send_embed_message(
    State(state): State<ApiState>,
    Json(payload): Json<SendEmbedRequest>,
) -> Json<SendMessageResponse> {
    if let Ok(ch_id) = payload.channel_id.parse::<u64>() {
        let channel_id = ChannelId::new(ch_id);
        
        let mut embed = serenity::builder::CreateEmbed::new();
        
        if let Some(t) = &payload.title { embed = embed.title(t); }
        if let Some(u) = &payload.url { embed = embed.url(u); }
        if let Some(d) = &payload.description { embed = embed.description(d); }
        
        if let Some(c_hex) = &payload.color {
            let clean_hex = c_hex.trim_start_matches('#');
            if let Ok(color_val) = u32::from_str_radix(clean_hex, 16) {
                embed = embed.color(color_val);
            }
        }
        
        if let Some(i) = &payload.image_url { embed = embed.image(i); }
        if let Some(t) = &payload.thumbnail_url { embed = embed.thumbnail(t); }
        
        if let Some(a_name) = &payload.author_name {
            let mut author = serenity::builder::CreateEmbedAuthor::new(a_name);
            if let Some(a_url) = &payload.author_url {
                author = author.url(a_url);
            }
            if let Some(a_icon) = &payload.author_icon {
                author = author.icon_url(a_icon);
            }
            embed = embed.author(author);
        }
        
        if let Some(f_text) = &payload.footer_text {
            let mut footer = serenity::builder::CreateEmbedFooter::new(f_text);
            if let Some(f_icon) = &payload.footer_icon {
                footer = footer.icon_url(f_icon);
            }
            embed = embed.footer(footer);
        }

        if let Some(fields) = &payload.fields {
            for f in fields {
                embed = embed.field(&f.name, &f.value, f.inline);
            }
        }

        let builder = serenity::builder::CreateMessage::new().embed(embed);
        
        match channel_id.send_message(&state.discord_http, builder).await {
            Ok(_) => Json(SendMessageResponse { success: true, error: None }),
            Err(e) => Json(SendMessageResponse { success: false, error: Some(e.to_string()) }),
        }
    } else {
        Json(SendMessageResponse { success: false, error: Some("Invalid channel ID format".to_string()) })
    }
}

#[derive(Deserialize)]
struct ModActionRequest {
    guild_id: String,
    target_user_id: String,
    moderator_id: String,
    action: String,
    reason: String,
    duration_minutes: Option<i64>,
}

async fn remote_mod_action(
    State(state): State<ApiState>,
    Json(payload): Json<ModActionRequest>,
) -> Json<SendMessageResponse> {
    use serenity::model::id::{GuildId, UserId};
    
    let guild_id = match payload.guild_id.parse::<u64>() {
        Ok(id) => GuildId::new(id),
        Err(_) => return Json(SendMessageResponse { success: false, error: Some("Invalid guild ID".to_string()) }),
    };
    
    let target_user_id = match payload.target_user_id.parse::<u64>() {
        Ok(id) => UserId::new(id),
        Err(_) => return Json(SendMessageResponse { success: false, error: Some("Invalid target user ID".to_string()) }),
    };
    
    let mod_service = crate::services::moderation::ModerationService::new(state.db_pool.clone());
    
    match payload.action.as_str() {
        "warn" => {
            if let Err(e) = mod_service.warn_user(&payload.guild_id, &payload.target_user_id, &payload.moderator_id, &payload.reason, None).await {
                return Json(SendMessageResponse { success: false, error: Some(e.to_string()) });
            }
        },
        "strike" => {
            if let Err(e) = mod_service.strike_user(&payload.guild_id, &payload.target_user_id, &payload.moderator_id, &payload.reason, None).await {
                return Json(SendMessageResponse { success: false, error: Some(e.to_string()) });
            }
        },
        "kick" => {
            if let Err(e) = guild_id.kick_with_reason(&state.discord_http, target_user_id, &payload.reason).await {
                return Json(SendMessageResponse { success: false, error: Some(e.to_string()) });
            }
        },
        "ban" => {
            if let Err(e) = guild_id.ban_with_reason(&state.discord_http, target_user_id, 0, &payload.reason).await {
                return Json(SendMessageResponse { success: false, error: Some(e.to_string()) });
            }
        },
        "timeout" => {
            let mins = payload.duration_minutes.unwrap_or(10);
            let timestamp = serenity::model::Timestamp::from_unix_timestamp(serenity::model::Timestamp::now().unix_timestamp() + (mins * 60)).unwrap();
            let builder = serenity::builder::EditMember::new().disable_communication_until(timestamp.to_string());
            if let Err(e) = guild_id.edit_member(&state.discord_http, target_user_id, builder).await {
                return Json(SendMessageResponse { success: false, error: Some(e.to_string()) });
            }
        },
        _ => return Json(SendMessageResponse { success: false, error: Some("Unknown action".to_string()) }),
    }
    
    Json(SendMessageResponse { success: true, error: None })
}

pub async fn start_api_server(chatbot_state: Arc<RwLock<bool>>, discord_cache: Arc<Cache>, discord_http: Arc<Http>, start_time: Instant, db_pool: sqlx::PgPool) {
    let api_state = ApiState {
        chatbot_enabled: chatbot_state,
        discord_cache,
        discord_http,
        start_time,
        db_pool,
    };

    let app = Router::new()
        .route("/api/status", get(get_status))
        .route("/api/chatbot/toggle", post(toggle_chatbot))
        .route("/api/message/send", post(send_message))
        .route("/api/message/embed", post(send_embed_message))
        .route("/api/moderation/action", post(remote_mod_action))
        .layer(CorsLayer::permissive())
        .with_state(api_state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port);
    
    let listener = match tokio::net::TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to bind API server to {}: {}", addr, e);
            return;
        }
    };
    println!("API Server running on {}", addr);
    
    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("API server error: {}", e);
    }
}

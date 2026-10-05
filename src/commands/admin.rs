use crate::types::{Context, Error};
use crate::utils::embeds::send_embed;
use serenity::model::{
    channel::{PermissionOverwriteType, PermissionOverwrite},
    permissions::Permissions,
};
use serenity::builder::EditChannel;

#[poise::command(slash_command, prefix_command, required_permissions = "MANAGE_CHANNELS", category = "Admin")]
pub async fn lock(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let everyone_role_id = serenity::model::id::RoleId::new(guild_id.get());
    
    let channel = match ctx.channel_id().to_channel(ctx.http()).await {
        Ok(c) => c.guild(),
        Err(_) => None,
    };
    
    if let Some(mut c) = channel {
        let overwrite = PermissionOverwrite {
            allow: Permissions::empty(),
            deny: Permissions::SEND_MESSAGES,
            kind: PermissionOverwriteType::Role(everyone_role_id),
        };
        
        let builder = EditChannel::new().permissions(vec![overwrite]);
        if let Err(e) = c.edit(ctx.http(), builder).await {
            send_embed(ctx, "Error", &format!("Failed to lock channel: {}", e), 0xED4245).await?;
            return Ok(());
        }
    }

    send_embed(ctx, "Admin: Lock", "This channel has been locked.", 0x2b2d31).await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command, required_permissions = "MANAGE_CHANNELS", category = "Admin")]
pub async fn unlock(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let everyone_role_id = serenity::model::id::RoleId::new(guild_id.get());
    
    let channel = match ctx.channel_id().to_channel(ctx.http()).await {
        Ok(c) => c.guild(),
        Err(_) => None,
    };
    
    if let Some(c) = channel {
        if let Err(e) = c.delete_permission(ctx.http(), PermissionOverwriteType::Role(everyone_role_id)).await {
            send_embed(ctx, "Error", &format!("Failed to unlock channel: {}", e), 0xED4245).await?;
            return Ok(());
        }
    }

    send_embed(ctx, "Admin: Unlock", "This channel has been unlocked.", 0x2b2d31).await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command, required_permissions = "MANAGE_CHANNELS", category = "Admin")]
pub async fn slowmode(
    ctx: Context<'_>, 
    #[description = "Duration in seconds (0 to disable)"] seconds: u16
) -> Result<(), Error> {
    if seconds > 21600 {
        send_embed(ctx, "Error", "Slowmode cannot exceed 21600 seconds (6 hours).", 0xED4245).await?;
        return Ok(());
    }

    let channel = match ctx.channel_id().to_channel(ctx.http()).await {
        Ok(c) => c.guild(),
        Err(_) => None,
    };
    
    if let Some(mut c) = channel {
        let builder = EditChannel::new().rate_limit_per_user(seconds);
        if let Err(e) = c.edit(ctx.http(), builder).await {
            send_embed(ctx, "Error", &format!("Failed to set slowmode: {}", e), 0xED4245).await?;
            return Ok(());
        }
    }

    let status_text = if seconds == 0 {
        "Slowmode has been disabled.".to_string()
    } else {
        format!("Slowmode set to {} seconds.", seconds)
    };

    send_embed(ctx, "Admin: Slowmode", &status_text, 0x2b2d31).await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command, check = "crate::utils::checks::is_staff", category = "Admin")]
pub async fn chatbot(
    ctx: Context<'_>, 
    #[description = "Action: 'enable' or 'disable'"] action: String
) -> Result<(), Error> {
    let action = action.to_lowercase();
    
    if action == "enable" || action == "on" {
        *ctx.data().chatbot_enabled.write().await = true;
        send_embed(ctx, "Chatbot AI", "Chatbot AI has been **ENABLED**.\nThe bot will now reply to tags and replies.", 0x2b2d31).await?;
    } else if action == "disable" || action == "off" {
        *ctx.data().chatbot_enabled.write().await = false;
        send_embed(ctx, "Chatbot AI", "Chatbot AI has been **DISABLED**.", 0x2b2d31).await?;
    } else {
        send_embed(ctx, "Error", "Usage: `/chatbot enable` or `/chatbot disable`", 0xED4245).await?;
    }
    
    Ok(())
}

#[poise::command(slash_command, prefix_command, check = "crate::utils::checks::is_staff", category = "Admin")]
pub async fn status(
    ctx: Context<'_>, 
    #[description = "Type: 'playing', 'watching', 'listening', 'competing'"] activity_type: String,
    #[rest]
    #[description = "The status message"] message: String,
) -> Result<(), Error> {
    use serenity::all::ActivityData;
    use serenity::all::OnlineStatus;

    let activity = match activity_type.to_lowercase().as_str() {
        "watching" => Some(ActivityData::watching(&message)),
        "listening" => Some(ActivityData::listening(&message)),
        "competing" => Some(ActivityData::competing(&message)),
        _ => Some(ActivityData::playing(&message)),
    };

    ctx.serenity_context().set_presence(activity, OnlineStatus::Online);
    send_embed(ctx, "Admin: Status", &format!("Status updated successfully."), 0x2b2d31).await?;
    
    Ok(())
}

use sqlx::Row;

#[poise::command(slash_command, prefix_command, category = "Admin", required_permissions = "MANAGE_GUILD", check = "crate::utils::checks::is_staff", subcommands("add_autoreply", "list_autoreplies", "remove_autoreply"))]
pub async fn autoreply(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

#[poise::command(slash_command, prefix_command, rename = "add")]
pub async fn add_autoreply(
    ctx: Context<'_>,
    #[description = "The text that triggers the auto-reply."] trigger: String,
    #[description = "The response text."] response: Option<String>,
    #[description = "An image to attach to the response."] media: Option<serenity::model::channel::Attachment>,
    #[description = "Use Advanced Components V2 Container style?"] use_container: Option<bool>,
) -> Result<(), Error> {
    if response.is_none() && media.is_none() {
        send_embed(ctx, "Error", "You must provide either a response or a media attachment.", 0xED4245).await?;
        return Ok(());
    }

    let guild_id = match ctx.guild_id() {
        Some(id) => id.to_string(),
        None => {
            send_embed(ctx, "Error", "This command can only be used in a server.", 0xED4245).await?;
            return Ok(());
        }
    };
    
    let media_url = media.map(|m| m.url.clone());
    let use_container = use_container.unwrap_or(false);

    let db_pool = &ctx.data().db_pool;

    let res = sqlx::query(
        "INSERT INTO khivella_autoreplies (guild_id, trigger, response, media_url, use_container)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (guild_id, trigger) DO UPDATE
         SET response = EXCLUDED.response, media_url = EXCLUDED.media_url, use_container = EXCLUDED.use_container"
    )
    .bind(&guild_id)
    .bind(&trigger.to_lowercase())
    .bind(&response)
    .bind(&media_url)
    .bind(&use_container)
    .execute(db_pool)
    .await;

    match res {
        Ok(_) => send_embed(ctx, "Success", &format!("Auto-reply added for trigger: `{}`", trigger), 0x2b2d31).await?,
        Err(e) => send_embed(ctx, "Error", &format!("Failed to save to database: {}", e), 0xED4245).await?,
    }

    Ok(())
}

#[poise::command(slash_command, prefix_command, rename = "list")]
pub async fn list_autoreplies(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = match ctx.guild_id() {
        Some(id) => id.to_string(),
        None => {
            send_embed(ctx, "Error", "This command can only be used in a server.", 0xED4245).await?;
            return Ok(());
        }
    };
    
    let db_pool = &ctx.data().db_pool;

    let replies = sqlx::query(
        "SELECT trigger, response, media_url FROM khivella_autoreplies WHERE guild_id = $1"
    )
    .bind(&guild_id)
    .fetch_all(db_pool)
    .await;

    match replies {
        Ok(rows) => {
            if rows.is_empty() {
                send_embed(ctx, "Auto-replies", "No auto-replies found for this server.", 0x2b2d31).await?;
            } else {
                let mut content = String::new();
                for r in rows {
                    let trigger_text: String = r.get("trigger");
                    let mut details = String::new();
                    
                    if let Ok(resp) = r.try_get::<String, _>("response") {
                        if !resp.is_empty() {
                            details.push_str(&format!("Response: {}\n", resp));
                        }
                    }
                    if let Ok(m) = r.try_get::<String, _>("media_url") {
                        if !m.is_empty() {
                            details.push_str(&format!("Media: {}\n", m));
                        }
                    }
                    content.push_str(&format!("**Trigger:** `{}`\n{}\n", trigger_text, details));
                }
                send_embed(ctx, "Auto-replies", &content, 0x2b2d31).await?;
            }
        }
        Err(e) => {
            send_embed(ctx, "Error", &format!("Failed to fetch from database: {}", e), 0xED4245).await?;
        }
    }
    
    Ok(())
}

#[poise::command(slash_command, prefix_command, rename = "remove")]
pub async fn remove_autoreply(
    ctx: Context<'_>,
    #[description = "The trigger content to remove."] trigger: String,
) -> Result<(), Error> {
    let guild_id = match ctx.guild_id() {
        Some(id) => id.to_string(),
        None => {
            send_embed(ctx, "Error", "This command can only be used in a server.", 0xED4245).await?;
            return Ok(());
        }
    };
    let db_pool = &ctx.data().db_pool;

    let res = sqlx::query(
        "DELETE FROM khivella_autoreplies WHERE guild_id = $1 AND trigger = $2"
    )
    .bind(&guild_id)
    .bind(&trigger.to_lowercase())
    .execute(db_pool)
    .await;

    match res {
        Ok(result) => {
            if result.rows_affected() > 0 {
                send_embed(ctx, "Success", &format!("Auto-reply removed for trigger: `{}`", trigger), 0x2b2d31).await?;
            } else {
                send_embed(ctx, "Not Found", &format!("No auto-reply found for trigger: `{}`", trigger), 0xED4245).await?;
            }
        }
        Err(e) => {
            send_embed(ctx, "Error", &format!("Failed to delete from database: {}", e), 0xED4245).await?;
        }
    }

    Ok(())
}

#[poise::command(slash_command, prefix_command, category = "Admin", required_permissions = "MANAGE_GUILD", check = "crate::utils::checks::is_staff", subcommands("booster_background", "booster_channel", "booster_style", "booster_test", "booster_text"))]
pub async fn booster(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

#[poise::command(slash_command, prefix_command, rename = "background")]
pub async fn booster_background(
    ctx: Context<'_>,
    #[description = "Direct URL to the background image"] url: String,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().to_string();
    let db_pool = &ctx.data().db_pool;

    sqlx::query(
        "INSERT INTO khivella_booster (guild_id, background_url) VALUES ($1, $2)
         ON CONFLICT (guild_id) DO UPDATE SET background_url = EXCLUDED.background_url"
    )
    .bind(&guild_id).bind(&url).execute(db_pool).await?;

    send_embed(ctx, "Booster System", "Booster banner background URL updated.", 0x2b2d31).await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command, rename = "channel")]
pub async fn booster_channel(
    ctx: Context<'_>,
    #[description = "Channel where booster messages are sent"] channel: serenity::model::channel::Channel,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().to_string();
    let db_pool = &ctx.data().db_pool;

    sqlx::query(
        "INSERT INTO khivella_booster (guild_id, channel_id) VALUES ($1, $2)
         ON CONFLICT (guild_id) DO UPDATE SET channel_id = EXCLUDED.channel_id"
    )
    .bind(&guild_id).bind(&channel.id().to_string()).execute(db_pool).await?;

    send_embed(ctx, "Booster System", &format!("Booster channel set to <#{}>", channel.id()), 0x2b2d31).await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command, rename = "style")]
pub async fn booster_style(
    ctx: Context<'_>,
    #[description = "Choose the message style (banner card or plain text)"] style: String,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().to_string();
    let db_pool = &ctx.data().db_pool;

    sqlx::query(
        "INSERT INTO khivella_booster (guild_id, style) VALUES ($1, $2)
         ON CONFLICT (guild_id) DO UPDATE SET style = EXCLUDED.style"
    )
    .bind(&guild_id).bind(&style).execute(db_pool).await?;

    send_embed(ctx, "Booster System", &format!("Booster style set to: {}", style), 0x2b2d31).await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command, rename = "text")]
pub async fn booster_text(
    ctx: Context<'_>,
    #[description = "Booster text. Placeholders: {username}, {guildName}, etc."] text: String,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().to_string();
    let db_pool = &ctx.data().db_pool;

    sqlx::query(
        "INSERT INTO khivella_booster (guild_id, text) VALUES ($1, $2)
         ON CONFLICT (guild_id) DO UPDATE SET text = EXCLUDED.text"
    )
    .bind(&guild_id).bind(&text).execute(db_pool).await?;

    send_embed(ctx, "Booster System", "Booster message text updated.", 0x2b2d31).await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command, rename = "test")]
pub async fn booster_test(
    ctx: Context<'_>,
    #[description = "User to test with"] user: Option<serenity::model::user::User>,
) -> Result<(), Error> {
    let target = user.unwrap_or_else(|| ctx.author().clone());
    let guild_id = ctx.guild_id().unwrap().to_string();
    let db_pool = &ctx.data().db_pool;

    let row = sqlx::query("SELECT channel_id, style, text, background_url FROM khivella_booster WHERE guild_id = $1")
        .bind(&guild_id).fetch_optional(db_pool).await?;

    if let Some(r) = row {
        let channel_id: Option<String> = r.try_get("channel_id").unwrap_or(None);
        let text: Option<String> = r.try_get("text").unwrap_or(None);
        
        let mut msg_text = text.unwrap_or_else(|| "Thank you {username} for boosting the server!".to_string());
        msg_text = msg_text.replace("{username}", &target.name);
        if let Some(guild) = ctx.partial_guild().await {
            msg_text = msg_text.replace("{guildName}", &guild.name);
        }

        if let Some(cid) = channel_id {
            if let Ok(id) = cid.parse::<u64>() {
                let channel = serenity::model::id::ChannelId::new(id);
                let _ = channel.send_message(&ctx.http(), serenity::builder::CreateMessage::new().content(msg_text)).await;
            }
        }
        send_embed(ctx, "Booster System", "Test message sent.", 0x2b2d31).await?;
    } else {
        send_embed(ctx, "Error", "Booster system not configured yet.", 0xED4245).await?;
    }

    Ok(())
}

#[poise::command(slash_command, prefix_command, category = "Admin", required_permissions = "MANAGE_MESSAGES", check = "crate::utils::checks::is_staff", subcommands("sticky_set", "sticky_remove", "sticky_list"))]
pub async fn sticky(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

#[poise::command(slash_command, prefix_command, rename = "set")]
pub async fn sticky_set(
    ctx: Context<'_>,
    #[description = "The content of the sticky message."] message: String,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().to_string();
    let channel_id = ctx.channel_id().to_string();
    let db_pool = &ctx.data().db_pool;

    let sent_msg = ctx.channel_id().send_message(&ctx.http(), serenity::builder::CreateMessage::new().content(&message)).await?;
    let msg_id = sent_msg.id.to_string();

    sqlx::query(
        "INSERT INTO khivella_sticky (guild_id, channel_id, message, last_message_id) VALUES ($1, $2, $3, $4)
         ON CONFLICT (guild_id, channel_id) DO UPDATE SET message = EXCLUDED.message, last_message_id = EXCLUDED.last_message_id"
    )
    .bind(&guild_id).bind(&channel_id).bind(&message).bind(&msg_id).execute(db_pool).await?;

    send_embed(ctx, "Sticky Message", "Sticky message set for this channel.", 0x2b2d31).await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command, rename = "remove")]
pub async fn sticky_remove(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().to_string();
    let channel_id = ctx.channel_id().to_string();
    let db_pool = &ctx.data().db_pool;

    let res = sqlx::query("DELETE FROM khivella_sticky WHERE guild_id = $1 AND channel_id = $2")
        .bind(&guild_id).bind(&channel_id).execute(db_pool).await?;

    if res.rows_affected() > 0 {
        send_embed(ctx, "Sticky Message", "Sticky message removed from this channel.", 0x2b2d31).await?;
    } else {
        send_embed(ctx, "Error", "No sticky message found in this channel.", 0xED4245).await?;
    }
    Ok(())
}

#[poise::command(slash_command, prefix_command, rename = "list")]
pub async fn sticky_list(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().to_string();
    let db_pool = &ctx.data().db_pool;

    let rows = sqlx::query("SELECT channel_id, message FROM khivella_sticky WHERE guild_id = $1")
        .bind(&guild_id).fetch_all(db_pool).await?;

    if rows.is_empty() {
        send_embed(ctx, "Sticky Messages", "No sticky messages in this server.", 0x2b2d31).await?;
    } else {
        let mut content = String::new();
        for r in rows {
            let cid: String = r.get("channel_id");
            let msg: String = r.get("message");
            let display_msg = if msg.len() > 50 {
                format!("{}...", &msg[..47])
            } else {
                msg
            };
            content.push_str(&format!("<#{}>: `{}`\n", cid, display_msg));
        }
        send_embed(ctx, "Sticky Messages", &content, 0x2b2d31).await?;
    }
    Ok(())
}

#[poise::command(slash_command, prefix_command, category = "Admin", required_permissions = "ADMINISTRATOR", check = "crate::utils::checks::is_staff")]
pub async fn restart(ctx: Context<'_>) -> Result<(), Error> {
    let msg = "Memulai proses *reboot* sistem secara paksa. Porpan akan offline sejenak dan secara otomatis menyala kembali melalui protokol *auto-recovery* Railway.\n\nHarap tunggu beberapa saat...";
    send_embed(ctx, "System Reboot Initiated", msg, 0xef4444).await?;
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    std::process::exit(1);
}

#[derive(serde::Serialize)]
struct RobloxUserRequest {
    userIds: Vec<u64>,
    #[serde(rename = "excludeBannedUsers")]
    exclude_banned_users: bool,
}

#[poise::command(slash_command, prefix_command, category = "Admin", check = "crate::utils::checks::is_staff", aliases("checkname"))]
pub async fn checknames(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?;
    
    let clan_data_str = ctx.data().clan_data.read().await.clone();
    if clan_data_str.is_empty() {
        send_embed(ctx, "Error", "Clan data is empty or not yet loaded from the website. Please try again later.", 0xED4245).await?;
        return Ok(());
    }
    
    let members: serde_json::Value = match serde_json::from_str(&clan_data_str) {
        Ok(v) => v,
        Err(_) => {
            send_embed(ctx, "Error", "Failed to parse clan data.", 0xED4245).await?;
            return Ok(());
        }
    };
    
    let mut user_map = std::collections::HashMap::new();
    let mut user_ids = Vec::new();
    
    let members_arr = members.as_array().or_else(|| members["members"].as_array());
    
    if let Some(arr) = members_arr {
        for member in arr {
            if let Some(profile_url) = member["robloxProfile"].as_str() {
                if let Some(id_str) = profile_url.split("users/").nth(1).and_then(|s| s.split('/').next()) {
                    if let Ok(id) = id_str.parse::<u64>() {
                        user_ids.push(id);
                        user_map.insert(id, member.clone());
                    }
                }
            }
        }
    }
    
    if user_ids.is_empty() {
        send_embed(ctx, "Check Names", "No Roblox profiles found in clan data.", 0x2b2d31).await?;
        return Ok(());
    }
    
    let client = reqwest::Client::new();
    let mut offenders: Vec<String> = Vec::new();
    let mut checked = 0;
    
    for chunk in user_ids.chunks(100) {
        let req_body = RobloxUserRequest {
            userIds: chunk.to_vec(),
            exclude_banned_users: false,
        };
        
        match client.post("https://users.roblox.com/v1/users")
            .json(&req_body)
            .send()
            .await {
            Ok(res) => {
                if let Ok(json) = res.json::<serde_json::Value>().await {
                    if let Some(users) = json["data"].as_array() {
                        for user in users {
                            if let Some(roblox_id) = user["id"].as_u64() {
                                if let Some(member) = user_map.get(&roblox_id) {
                                    let roblox_display_name = user["displayName"].as_str().unwrap_or("").to_lowercase();
                                    let roblox_username = user["name"].as_str().unwrap_or("").to_lowercase();
                                    
                                    let json_name = member["name"].as_str().unwrap_or("");
                                    let json_username = member["username"].as_str().unwrap_or("");
                                    
                                    let mut detail = String::new();
                                    if roblox_display_name != json_name.to_lowercase() {
                                        detail = format!("`{}` -> `{}`", json_name, user["displayName"].as_str().unwrap_or(""));
                                    } else if roblox_username != json_username.to_lowercase() {
                                        detail = format!("`@{}` -> `@{}`", json_username, user["name"].as_str().unwrap_or(""));
                                    }
                                    
                                    if !detail.is_empty() {
                                        if let Some(discord_id) = member["socials"]["discordId"].as_str() {
                                            offenders.push(format!("<@{}>: {}", discord_id, detail));
                                        }
                                    }
                                    checked += 1;
                                }
                            }
                        }
                    }
                }
            },
            Err(_) => {
                send_embed(ctx, "Error", "Failed to fetch from Roblox API.", 0xED4245).await?;
                return Ok(());
            }
        }
    }
    
    if offenders.is_empty() {
        send_embed(ctx, "Check Names", &format!("Checked {} accounts. All names are matching!", checked), 0x00FF00).await?;
    } else {
        let mut msg_content = "**UNAUTHORIZED NAME CHANGE DETECTED**\n\nThe following users have changed their Roblox Name without prior notice:\n\n".to_string();
        
        for offender in &offenders {
            msg_content.push_str(offender);
            msg_content.push('\n');
        }
        
        msg_content.push_str("\n**ACTION REQUIRED**\nPlease open a ticket within 24 hours to clarify this name change, or you will be kicked.");
        
        // Send to specific channel 1556213720173125642
        let target_channel_id = serenity::model::id::ChannelId::new(1556213720173125642);
        let _ = target_channel_id.send_message(ctx.http(), serenity::builder::CreateMessage::new().content(&msg_content)).await;
        
        send_embed(ctx, "Check Names", &format!("Found {} mismatches! Warning has been sent to the target channel.", offenders.len()), 0xFFD700).await?;
    }
    
    Ok(())
}

#[poise::command(slash_command, prefix_command, check = "crate::utils::checks::is_staff", category = "Admin", subcommands("autothread_set", "autothread_remove", "autothread_list"), rename = "autothread")]
pub async fn autothread(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

#[poise::command(slash_command, prefix_command, rename = "set", category = "Admin")]
pub async fn autothread_set(
    ctx: Context<'_>, 
    #[description = "Target channel"] channel: serenity::model::channel::Channel,
    #[description = "Emojis to react with (separated by space)"] #[rest] emojis: String
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let channel_id = channel.id();
    let db_pool = &ctx.data().db_pool;

    let res = sqlx::query("INSERT INTO khivella_galleries (guild_id, channel_id, emojis) VALUES ($1, $2, $3) ON CONFLICT (guild_id, channel_id) DO UPDATE SET emojis = $3")
        .bind(guild_id.to_string())
        .bind(channel_id.to_string())
        .bind(emojis.clone())
        .execute(db_pool)
        .await;

    match res {
        Ok(_) => {
            send_embed(ctx, "Auto-Thread Configured", &format!("Channel <#{}> has been set up for auto-threads.\n**Auto-Reacts:** {}", channel_id, emojis), 0x2ecc71).await?;
        },
        Err(e) => {
            send_embed(ctx, "Error", &format!("Failed to configure auto-thread: {}", e), 0xED4245).await?;
        }
    }
    Ok(())
}

#[poise::command(slash_command, prefix_command, rename = "remove", category = "Admin")]
pub async fn autothread_remove(
    ctx: Context<'_>, 
    #[description = "Target channel"] channel: serenity::model::channel::Channel
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let channel_id = channel.id();
    let db_pool = &ctx.data().db_pool;

    let res = sqlx::query("DELETE FROM khivella_galleries WHERE guild_id = $1 AND channel_id = $2")
        .bind(guild_id.to_string())
        .bind(channel_id.to_string())
        .execute(db_pool)
        .await;

    match res {
        Ok(_) => {
            send_embed(ctx, "Auto-Thread Removed", &format!("Channel <#{}> is no longer using auto-threads.", channel_id), 0x2ecc71).await?;
        },
        Err(e) => {
            send_embed(ctx, "Error", &format!("Failed to remove auto-thread: {}", e), 0xED4245).await?;
        }
    }
    Ok(())
}

#[poise::command(slash_command, prefix_command, rename = "list", category = "Admin")]
pub async fn autothread_list(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let db_pool = &ctx.data().db_pool;

    let rows = sqlx::query("SELECT channel_id, emojis FROM khivella_galleries WHERE guild_id = $1")
        .bind(guild_id.to_string())
        .fetch_all(db_pool)
        .await;

    match rows {
        Ok(results) => {
            if results.is_empty() {
                send_embed(ctx, "Configured Auto-Threads", "No auto-thread channels have been set up yet.", 0x3498db).await?;
                return Ok(());
            }

            let mut desc = String::new();
            use sqlx::Row;
            for r in results {
                let cid: String = r.get("channel_id");
                let emojis: String = r.get("emojis");
                desc.push_str(&format!("• <#{}> => {}\n", cid, emojis));
            }
            send_embed(ctx, "Configured Auto-Threads", &desc, 0x3498db).await?;
        },
        Err(e) => {
            send_embed(ctx, "Error", &format!("Failed to fetch auto-threads: {}", e), 0xED4245).await?;
        }
    }
    Ok(())
}

#[poise::command(slash_command, prefix_command, check = "crate::utils::checks::is_staff", category = "Admin", subcommands("absen_manage_open", "absen_manage_close", "absen_manage_list", "absen_manage_clear"), rename = "absen_manage")]
pub async fn absen_manage(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

#[poise::command(slash_command, prefix_command, rename = "open", category = "Admin")]
pub async fn absen_manage_open(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let db_pool = &ctx.data().db_pool;

    let res = sqlx::query("INSERT INTO khivella_absen_state (guild_id, is_open) VALUES ($1, TRUE) ON CONFLICT (guild_id) DO UPDATE SET is_open = TRUE")
        .bind(guild_id.to_string())
        .execute(db_pool)
        .await;

    if res.is_ok() {
        send_embed(ctx, "Absen Dibuka", "Sistem absen bulanan sekarang **DIBUKA**. Member sudah bisa memakai command `/absen`.", 0x2ecc71).await?;
    } else {
        send_embed(ctx, "Error", "Gagal membuka sistem absen.", 0xED4245).await?;
    }
    Ok(())
}

#[poise::command(slash_command, prefix_command, rename = "close", category = "Admin")]
pub async fn absen_manage_close(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let db_pool = &ctx.data().db_pool;

    let res = sqlx::query("INSERT INTO khivella_absen_state (guild_id, is_open) VALUES ($1, FALSE) ON CONFLICT (guild_id) DO UPDATE SET is_open = FALSE")
        .bind(guild_id.to_string())
        .execute(db_pool)
        .await;

    if res.is_ok() {
        send_embed(ctx, "Absen Ditutup", "Sistem absen bulanan sekarang **DITUTUP**. Member sudah tidak bisa absen.", 0xED4245).await?;
    } else {
        send_embed(ctx, "Error", "Gagal menutup sistem absen.", 0xED4245).await?;
    }
    Ok(())
}

#[poise::command(slash_command, prefix_command, rename = "list", category = "Admin")]
pub async fn absen_manage_list(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let db_pool = &ctx.data().db_pool;

    let rows = sqlx::query("SELECT discord_username, roblox_name, timestamp FROM khivella_absen_records WHERE guild_id = $1 ORDER BY timestamp ASC")
        .bind(guild_id.to_string())
        .fetch_all(db_pool)
        .await;

    match rows {
        Ok(results) => {
            if results.is_empty() {
                send_embed(ctx, "Data Absen", "Belum ada member yang melakukan absen.", 0x3498db).await?;
                return Ok(());
            }

            let mut desc = String::new();
            use sqlx::Row;
            for (i, r) in results.iter().enumerate() {
                let discord_username: String = r.get("discord_username");
                let roblox_name: String = r.get("roblox_name");
                
                let line = format!("{}. **{}** (`@{}`)\n", i + 1, roblox_name, discord_username);
                
                // Keep it under embed limits
                if desc.len() + line.len() > 3900 {
                    desc.push_str("...dan lainnya.");
                    break;
                }
                desc.push_str(&line);
            }
            send_embed(ctx, format!("Data Absen (Total: {})", results.len()).as_str(), &desc, 0x3498db).await?;
        },
        Err(e) => {
            send_embed(ctx, "Error", &format!("Gagal mengambil data absen: {}", e), 0xED4245).await?;
        }
    }
    Ok(())
}

#[poise::command(slash_command, prefix_command, rename = "clear", category = "Admin")]
pub async fn absen_manage_clear(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let db_pool = &ctx.data().db_pool;

    let res = sqlx::query("DELETE FROM khivella_absen_records WHERE guild_id = $1")
        .bind(guild_id.to_string())
        .execute(db_pool)
        .await;

    if res.is_ok() {
        send_embed(ctx, "Data Absen Dihapus", "Semua data absen bulan ini berhasil dibersihkan! Sistem siap digunakan untuk bulan depan.", 0x2ecc71).await?;
    } else {
        send_embed(ctx, "Error", "Gagal membersihkan data absen.", 0xED4245).await?;
    }
    Ok(())
}

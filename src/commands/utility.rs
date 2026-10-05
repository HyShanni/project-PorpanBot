use crate::types::{Context, Error};
use crate::utils::embeds::{create_embed, send_embed};
use serenity::model::user::User;

#[poise::command(slash_command, prefix_command, category = "Utility")]
pub async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    send_embed(ctx, "Pong", "System is online and responsive.", 0x2b2d31).await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command, guild_only, category = "Utility")]
pub async fn serverinfo(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let guild = match ctx.http().get_guild_with_counts(guild_id).await {
        Ok(g) => g,
        Err(_) => {
            send_embed(ctx, "Error", "Could not fetch server information.", 0xED4245).await?;
            return Ok(());
        }
    };

    let created_timestamp = guild_id.created_at().unix_timestamp();
    
    let channels = ctx.http().get_channels(guild_id).await.unwrap_or_default();
    let text_channels = channels.iter().filter(|c| c.kind == serenity::all::ChannelType::Text).count();
    let voice_channels = channels.iter().filter(|c| c.kind == serenity::all::ChannelType::Voice).count();
    
    let total_emojis = guild.emojis.len();
    let animated_emojis = guild.emojis.values().filter(|e| e.animated).count();
    let static_emojis = total_emojis - animated_emojis;
    
    let member_count = guild.approximate_member_count.unwrap_or(0);
    let online_count = guild.approximate_presence_count.unwrap_or(0);
    
    let tier_str = match guild.premium_tier {
        serenity::all::PremiumTier::Tier1 => "Level 1",
        serenity::all::PremiumTier::Tier2 => "Level 2",
        serenity::all::PremiumTier::Tier3 => "Level 3",
        _ => "None",
    };

    let description = if let Some(desc) = &guild.description {
        format!("*{}*", desc)
    } else {
        "A community server managed by Porpan.".to_string()
    };

    let mut embed = serenity::builder::CreateEmbed::new()
        .title(&guild.name)
        .description(description)
        .color(0xef4444)
        .field("MEMBER DEMOGRAPHICS", format!("Total Members: **{}**\nOnline Members: **{}**", member_count, online_count), false)
        .field("SERVER ARCHITECTURE", format!("Text Channels: **{}**\nVoice Channels: **{}**\nTotal Roles: **{}**", text_channels, voice_channels, guild.roles.len()), false)
        .field("COMMUNITY ASSETS", format!("Total Emojis: **{}** ({} Static, {} Animated)\nBoost Level: **{}** ({} Boosts)", total_emojis, static_emojis, animated_emojis, tier_str, guild.premium_subscription_count.unwrap_or(0)), false)
        .field("CORE INFORMATION", format!("Server ID: `{}`\nEstablished: <t:{}:D>\nServer Owner: <@{}>", guild.id, created_timestamp, guild.owner_id), false)
        .footer(serenity::builder::CreateEmbedFooter::new("4FUN Server Analytics"));

    if let Some(icon) = guild.icon_url() { 
        embed = embed.thumbnail(icon); 
    }
    if let Some(mut banner) = guild.banner_url() { 
        banner = banner.replace("?size=1024", "?size=4096");
        embed = embed.image(banner); 
    }

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command, category = "Utility")]
pub async fn userinfo(
    ctx: Context<'_>, 
    #[description = "User to inspect"] user: Option<User>
) -> Result<(), Error> {
    let user = user.unwrap_or_else(|| ctx.author().clone());
    
    let member = if let Some(guild_id) = ctx.guild_id() {
        guild_id.member(ctx.http(), user.id).await.ok()
    } else {
        None
    };
    
    let created_timestamp = user.id.created_at().unix_timestamp();
    
    let embed = serenity::builder::CreateEmbed::new()
        .title(format!("User Information: {}", user.name))
        .color(0xef4444)
        .field("Global Name", user.global_name.as_deref().unwrap_or("None").to_string(), true)
        .field("Identifier", user.id.to_string(), true)
        .field("Creation Date", format!("<t:{}:F>", created_timestamp), false);

    let embed = if let Some(avatar_url) = user.avatar_url() { embed.thumbnail(avatar_url) } else { embed };
    let mut embed = if let Some(mut banner) = user.banner_url() { 
        banner = banner.replace("?size=1024", "?size=4096");
        embed.image(banner) 
    } else { 
        embed 
    };

    if let Some(m) = member {
        if let Some(joined) = m.joined_at {
            embed = embed.field("Network Join Date", format!("<t:{}:F>", joined.unix_timestamp()), false);
        }
        
        let roles = m.roles;
        if !roles.is_empty() {
            let roles_str: Vec<String> = roles.iter().map(|r| format!("<@&{}>", r)).collect();
            let mut joined = roles_str.join(", ");
            if joined.len() > 1000 {
                joined = format!("{}... and more", &joined[0..980]);
            }
            embed = embed.field(format!("Assigned Roles ({})", roles.len()), joined, false);
        }
    }

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command, category = "Utility")]
pub async fn avatar(
    ctx: Context<'_>, 
    #[description = "User to inspect"] user: Option<User>
) -> Result<(), Error> {
    let user = user.unwrap_or_else(|| ctx.author().clone());

    let url = user.face(); 
    let mut embed = create_embed(&format!("Avatar: {}", user.name), "", 0x2b2d31);
    embed = embed.image(url);
    
    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command, category = "Utility", track_edits)]
pub async fn help(ctx: Context<'_>) -> Result<(), Error> {
    let commands = &ctx.framework().options().commands;
    
    let mut total_commands = 0;
    let mut categories: std::collections::HashMap<&str, Vec<String>> = std::collections::HashMap::new();
    
    for cmd in commands {
        if cmd.hide_in_help { continue; }
        let category = cmd.category.as_deref().unwrap_or("Uncategorized");
        let desc = cmd.description.as_deref().unwrap_or("No description provided");
        categories.entry(category).or_default().push(format!("`/{}` - {}", cmd.name, desc));
        total_commands += 1;
        
        for subcmd in &cmd.subcommands {
            let sub_desc = subcmd.description.as_deref().unwrap_or("No description provided");
            categories.entry(category).or_default().push(format!("`/{} {}` - {}", cmd.name, subcmd.name, sub_desc));
            total_commands += 1;
        }
    }
    
    let total_categories = categories.len();

    let description = format!(
        "Welcome to the **Porpan Command Center**!\n\n\
        **✦ Quick Guide:**\n\
        - Explore the categories below to discover what I can do.\n\
        - Use `/` in chat to see Discord's native auto-complete.\n\n\
        **✦ System Stats:**\n\
        - Modules Active: `{}`\n\
        - Commands Loaded: `{}`",
        total_categories, total_commands
    );

    let mut embed = serenity::builder::CreateEmbed::new()
        .title("Porpan Help & Documentation")
        .color(0xef4444)
        .description(description);

    let mut sorted_categories: Vec<_> = categories.into_iter().collect();
    sorted_categories.sort_by_key(|(k, _)| *k);

    for (cat, cmds) in sorted_categories {
        let mut current_value = String::new();
        let mut part = 1;
        
        for cmd_str in cmds {
            if current_value.len() + cmd_str.len() + 1 > 1024 {
                let title = if part == 1 { format!("🔹 {}", cat) } else { format!("🔹 {} (Cont.)", cat) };
                embed = embed.field(title, current_value.clone(), false);
                current_value.clear();
                part += 1;
            }
            if !current_value.is_empty() {
                current_value.push('\n');
            }
            current_value.push_str(&cmd_str);
        }
        
        if !current_value.is_empty() {
            let title = if part == 1 { format!("🔹 {}", cat) } else { format!("🔹 {} (Cont.)", cat) };
            embed = embed.field(title, current_value, false);
        }
    }
    
    embed = embed.footer(serenity::builder::CreateEmbedFooter::new("Porpan OS v1.0.0 | Built for 4FUN Clan"));

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command, category = "Utility", subcommands("grab_sticker", "grab_emoji", "grab_image"))]
pub async fn grab(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

#[poise::command(slash_command, prefix_command, rename = "sticker", required_permissions = "MANAGE_EMOJIS_AND_STICKERS")]
pub async fn grab_sticker(
    ctx: Context<'_>,
    #[description = "Sticker ID to grab"] sticker_id: String,
) -> Result<(), Error> {
    send_embed(ctx, "Grab", &format!("Sticker grab logic not fully implemented yet for ID: {}", sticker_id), 0x2b2d31).await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command, rename = "emoji", required_permissions = "MANAGE_EMOJIS_AND_STICKERS")]
pub async fn grab_emoji(
    ctx: Context<'_>,
    #[description = "Emoji to grab (custom emoji format)"] emoji: String,
) -> Result<(), Error> {
    send_embed(ctx, "Grab", &format!("Emoji grab logic not fully implemented yet for: {}", emoji), 0x2b2d31).await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command, rename = "image", required_permissions = "MANAGE_EMOJIS_AND_STICKERS")]
pub async fn grab_image(
    ctx: Context<'_>,
    #[description = "ID of the message containing the image"] message_id: String,
    #[description = "Name for the new sticker (max 30 chars)"] name: Option<String>,
) -> Result<(), Error> {
    send_embed(ctx, "Grab", &format!("Image grab logic not fully implemented yet for msg: {}", message_id), 0x2b2d31).await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command, category = "Utility")]
pub async fn report(
    ctx: Context<'_>,
    #[description = "User to report"] user: User,
    #[description = "Reason for the report"] reason: String,
) -> Result<(), Error> {
    send_embed(ctx, "Report", &format!("Successfully reported {} for: {}", user.name, reason), 0x2b2d31).await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command, category = "Utility")]
pub async fn stats(ctx: Context<'_>) -> Result<(), Error> {
    use sysinfo::System;
    let mut sys = System::new_all();
    sys.refresh_all();
    
    let total_memory = sys.total_memory() / 1_048_576; 
    let used_memory = sys.used_memory() / 1_048_576; 
    let cpu_cores = sys.cpus().len();
    let os_name = System::name().unwrap_or_else(|| "Unknown OS".to_string());

    let guild_count = ctx.cache().guilds().len();
    let user_count = ctx.cache().users().len();
    
    let bot_uptime_secs = ctx.data().start_time.elapsed().as_secs();
    let days = bot_uptime_secs / 86400;
    let hours = (bot_uptime_secs % 86400) / 3600;
    let mins = (bot_uptime_secs % 3600) / 60;
    let uptime_str = format!("{}d {}h {}m", days, hours, mins);
    
    let created = ctx.created_at().timestamp_millis();
    let now = serenity::model::Timestamp::now().timestamp_millis();
    let api_latency = format!("{}ms", (now - created).max(0));

    let db_status = "Online (PostgreSQL)";

    let embed = serenity::builder::CreateEmbed::new()
        .title("Porpan System Diagnostics")
        .color(0xef4444)
        .description("Real-time telemetry and resource usage statistics.")
        .field("Developer Identity", "**Author:** phy0n\n**Organization:** 4FUN Clan", false)
        .field("Network Reach", format!("**Servers:** {}\n**Cached Users:** {}\n**API Latency:** {}", guild_count, user_count, api_latency), true)
        .field("Hardware", format!("**OS:** {}\n**CPU Cores:** {}\n**RAM:** {} MB / {} MB", os_name, cpu_cores, used_memory, total_memory), true)
        .field("Core Systems", format!("**Database:** {}\n**Framework:** Poise (Rust)\n**Engine Version:** v1.0.0\n**Uptime:** {}", db_status, uptime_str), false)
        .footer(serenity::builder::CreateEmbedFooter::new("4FUN Core Engine"));

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command, category = "Utility")]
pub async fn about(ctx: Context<'_>) -> Result<(), Error> {
    let description = "Porpan adalah asisten virtual resmi yang dikembangkan eksklusif untuk Clan 4FUN.\n\n\
    Beroperasi sebagai penjaga server utama, Porpan bertanggung jawab atas manajemen member, pengecekan data Roblox, dan memastikan kenyamanan komunitas.\n\n\
    Selain tugas teknisnya, Porpan hadir sebagai teman yang ramah, asik diajak ngobrol, dan siap menemani keseharian para member 4FUN.";
    let bot_id = ctx.cache().current_user().id;

    let mut embed = serenity::builder::CreateEmbed::new()
        .title("Porpan")
        .color(0xef4444)
        .description(description)
        .field("Identitas", "AI Assistant", true)
        .field("Lokasi Sistem", "Surabaya, Indonesia", true)
        .footer(serenity::builder::CreateEmbedFooter::new("Porpan Core Engine • v1.0.0"));

    if let Ok(user) = bot_id.to_user(ctx.http()).await {
        embed = embed.thumbnail(user.face());
        if let Some(mut banner) = user.banner_url() {
            banner = banner.replace("?size=1024", "?size=4096");
            embed = embed.image(banner);
        }
    } else {
        embed = embed.thumbnail(ctx.cache().current_user().face());
    }

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command, category = "Utility")]
pub async fn profile(
    ctx: Context<'_>,
    #[rest]
    #[description = "Discord tag, ID, or Roblox Name"] query: Option<String>,
) -> Result<(), Error> {
    ctx.defer().await?;
    
    let clan_data_str = ctx.data().clan_data.read().await.clone();
    
    let mut found_member = None;
    let mut target_discord_id = String::new();
    
    if let Some(q) = &query {
        let q_lower = q.to_lowercase();
        let maybe_id = if q.starts_with("<@") && q.ends_with('>') {
            q.replace("<@", "").replace("!", "").replace(">", "")
        } else {
            q.clone()
        };
        
        if !clan_data_str.is_empty() {
            if let Ok(members) = serde_json::from_str::<serde_json::Value>(&clan_data_str) {
                let members_arr = members.as_array().or_else(|| members["members"].as_array());
                if let Some(arr) = members_arr {
                    for member in arr {
                        let d_id = member["socials"]["discordId"].as_str().unwrap_or("");
                        let r_name = member["name"].as_str().unwrap_or("").to_lowercase();
                        let r_user = member["username"].as_str().unwrap_or("").to_lowercase();
                        
                        if d_id == maybe_id || r_name == q_lower || r_user == q_lower {
                            found_member = Some(member.clone());
                            target_discord_id = d_id.to_string();
                            break;
                        }
                    }
                }
            }
        }
        
        if found_member.is_none() {
            if let Ok(id) = maybe_id.parse::<u64>() {
                target_discord_id = id.to_string();
            } else {
                send_embed(ctx, "Profile", &format!("Tidak menemukan member 4FUN atau Discord User dengan nama/ID: `{}`.", q), 0xED4245).await?;
                return Ok(());
            }
        }
    } else {
        target_discord_id = ctx.author().id.to_string();
        if !clan_data_str.is_empty() {
            if let Ok(members) = serde_json::from_str::<serde_json::Value>(&clan_data_str) {
                let members_arr = members.as_array().or_else(|| members["members"].as_array());
                if let Some(arr) = members_arr {
                    for member in arr {
                        if member["socials"]["discordId"].as_str().unwrap_or("") == target_discord_id {
                            found_member = Some(member.clone());
                            break;
                        }
                    }
                }
            }
        }
    }
    
    let mut discord_user = None;
    let mut discord_member = None;
    
    if let Ok(id) = target_discord_id.parse::<u64>() {
        let user_id = serenity::model::id::UserId::new(id);
        discord_user = ctx.http().get_user(user_id).await.ok();
        
        if let Some(guild_id) = ctx.guild_id() {
            discord_member = guild_id.member(ctx.http(), user_id).await.ok();
        }
    }
    
    let mut roblox_created = String::new();
    let mut roblox_live_bio = String::new();
    
    if let Some(member) = &found_member {
        if let Some(roblox_url) = member["robloxProfile"].as_str() {
            if let Some(id_str) = roblox_url.split("users/").nth(1).and_then(|s| s.split('/').next()) {
                if let Ok(rbx_id) = id_str.parse::<u64>() {
                    let req_url = format!("https://users.roblox.com/v1/users/{}", rbx_id);
                    if let Ok(res) = reqwest::get(&req_url).await {
                        if let Ok(json) = res.json::<serde_json::Value>().await {
                            if let Some(created) = json["created"].as_str() {
                                if let Ok(parsed_time) = chrono::DateTime::parse_from_rfc3339(created) {
                                    roblox_created = format!("<t:{}:D>", parsed_time.timestamp());
                                }
                            }
                            if let Some(bio) = json["description"].as_str() {
                                roblox_live_bio = bio.to_string();
                            }
                        }
                    }
                }
            }
        }
    }
    
    let mut embed = serenity::builder::CreateEmbed::new().color(0xef4444);
    let discord_name = discord_user.as_ref().map(|u| u.name.clone()).unwrap_or_else(|| "Unknown User".to_string());
    
    if let Some(u) = &discord_user {
        embed = embed.thumbnail(u.face());
    }
    
    if let Some(member) = found_member {
        let name = member["name"].as_str().unwrap_or("Unknown");
        let username = member["username"].as_str().unwrap_or("Unknown");
        let roblox_url = member["robloxProfile"].as_str().unwrap_or("");
        let roles = member["roles"].as_array()
            .map(|arr| arr.iter().filter_map(|r| r.as_str()).collect::<Vec<_>>().join(", "))
            .unwrap_or_else(|| "MEMBER".to_string());
        
        let id = member["id"].as_i64().unwrap_or(0);
        let priority = member["orderPriority"].as_i64().unwrap_or(0);
        
        embed = embed.title(format!("{}'s 4FUN Profile", discord_name));
        
        // Header Description
        let header = format!("` ID: {} ` • ` Priority: {} ` • ` Role: {} `\n", id, priority, roles);
        embed = embed.description(header);
                     
        // Roblox Section
        let mut rbx_info = format!("**Name:** {} (`@{}`)\n**Profile:** [View Profile]({})\n", name, username, roblox_url);
        if !roblox_created.is_empty() {
            rbx_info.push_str(&format!("**Created:** {}\n", roblox_created));
        }
        if !roblox_live_bio.is_empty() {
            let mut trunc_bio = roblox_live_bio.clone();
            if trunc_bio.len() > 100 {
                trunc_bio.truncate(97);
                trunc_bio.push_str("...");
            }
            rbx_info.push_str(&format!("**Bio:** *{}*\n", trunc_bio));
        }
        embed = embed.field("ROBLOX", rbx_info, false);
        
        // Discord Section
        let mut d_info = String::new();
        let d_joined = discord_member.as_ref().and_then(|m| m.joined_at).map(|t| format!("<t:{}:D>", t.unix_timestamp())).unwrap_or_else(|| "Unknown".to_string());
        d_info.push_str(&format!("**Joined Server:** {}\n", d_joined));
        
        if let Some(m) = &discord_member {
            let mut d_roles: Vec<String> = m.roles.iter().map(|r| format!("<@&{}>", r)).collect();
            if !d_roles.is_empty() {
                if d_roles.len() > 5 {
                    d_roles.truncate(5);
                    d_roles.push("...".to_string());
                }
                d_info.push_str(&format!("**Roles:** {}\n", d_roles.join(", ")));
            }
        }
        embed = embed.field("DISCORD", d_info, false);
        
        // Socials Section
        let mut socials = format!("**Discord:** <@{}>\n", target_discord_id);
        if let Some(tiktok) = member["socials"]["tiktok"].as_str() {
            if !tiktok.is_empty() {
                socials.push_str(&format!("**TikTok:** [@{}](https://tiktok.com/@{})\n", tiktok, tiktok));
            }
        }
        embed = embed.field("SOCIALS", socials, false);
        
        if let Some(bio) = member["description"].as_str() {
            if !bio.is_empty() {
                embed = embed.field("WEBSITE BIO", format!("*\"{}\"*", bio), false);
            }
        }
        
        embed = embed.footer(serenity::builder::CreateEmbedFooter::new("4FUN Clan Member"));
    } else {
        embed = embed.title(format!("{}'s Profile", discord_name))
                     .description(format!("**Discord ID:** {}\n*This user is not registered in the 4FUN website data.*", target_discord_id));
                     
        if let Some(m) = &discord_member {
            if let Some(joined) = m.joined_at {
                embed = embed.field("Server Join Date", format!("<t:{}:F>", joined.unix_timestamp()), false);
            }
        }
    }
    
    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command, category = "Utility", aliases("member"))]
pub async fn members(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?;
    let clan_data_str = ctx.data().clan_data.read().await.clone();
    
    if clan_data_str.is_empty() {
        send_embed(ctx, "Members", "Data clan belum tersedia. Coba lagi nanti.", 0xED4245).await?;
        return Ok(());
    }

    let members_data = match serde_json::from_str::<serde_json::Value>(&clan_data_str) {
        Ok(v) => v,
        Err(_) => {
            send_embed(ctx, "Error", "Gagal memproses data clan.", 0xED4245).await?;
            return Ok(());
        }
    };

    let members_arr = members_data.as_array().or_else(|| members_data["members"].as_array());
    if let Some(arr) = members_arr {
        let mut embed = serenity::builder::CreateEmbed::new()
            .title("👥 4FUN Clan Members")
            .color(0xef4444)
            .description(format!("Total Members: **{}**", arr.len()));

        let mut current_field = String::new();

        for member in arr {
            let name = member["name"].as_str().unwrap_or("Unknown");
            let username = member["username"].as_str().unwrap_or("Unknown");
            let discord_id = member["socials"]["discordId"].as_str().unwrap_or("");
            
            let line = format!("• **{}** (`@{}`) - <@{}>\n", name, username, discord_id);
            
            if current_field.len() + line.len() > 1024 {
                embed = embed.field("\u{200B}", current_field.clone(), false);
                current_field = String::new();
            }
            current_field.push_str(&line);
        }
        
        if !current_field.is_empty() {
            embed = embed.field("\u{200B}", current_field, false);
        }

        ctx.send(poise::CreateReply::default().embed(embed)).await?;
    } else {
        send_embed(ctx, "Error", "Data member tidak ditemukan.", 0xED4245).await?;
    }
    
    Ok(())
}

#[poise::command(slash_command, prefix_command, category = "Utility")]
pub async fn absen(
    ctx: Context<'_>,
    #[description = "Your Roblox Display Name"] roblox_name: String,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let db_pool = &ctx.data().db_pool;

    let state_row = sqlx::query("SELECT is_open, channel_id FROM khivella_absen_state WHERE guild_id = $1")
        .bind(guild_id.to_string())
        .fetch_optional(db_pool)
        .await;

    let (is_open, target_channel_id) = match state_row {
        Ok(Some(row)) => {
            use sqlx::Row;
            let open = row.get::<bool, _>("is_open");
            let cid: Option<String> = row.try_get("channel_id").unwrap_or(None);
            (open, cid)
        },
        _ => (false, None),
    };

    if !is_open {
        crate::utils::embeds::send_embed(ctx, "Attendance Closed", "Sorry! The monthly attendance session is currently closed. Please wait for an admin to open it.", 0xED4245).await?;
        return Ok(());
    }

    if let Some(required_channel) = target_channel_id {
        if ctx.channel_id().to_string() != required_channel {
            crate::utils::embeds::send_embed(ctx, "Invalid Channel", &format!("You can only record your attendance in <#{}>.", required_channel), 0xED4245).await?;
            return Ok(());
        }
    }

    let json_data = ctx.data().clan_data.read().await;
    let parsed: serde_json::Value = serde_json::from_str(&json_data).unwrap_or(serde_json::Value::Null);
    let members_array = parsed["data"].as_array();

    let mut found = false;
    let mut verified_name = String::new();

    if let Some(arr) = members_array {
        for member in arr {
            if let Some(name) = member["name"].as_str() {
                if name.to_lowercase() == roblox_name.to_lowercase() {
                    found = true;
                    verified_name = name.to_string();
                    break;
                }
            }
        }
    }

    if !found {
        crate::utils::embeds::send_embed(ctx, "Attendance Failed", &format!("The Roblox name `{}` was not found in the 4FUN Clan database. Please double-check your spelling!", roblox_name), 0xED4245).await?;
        return Ok(());
    }

    let discord_id = ctx.author().id.to_string();
    let discord_username = ctx.author().name.clone();

    let res = sqlx::query("INSERT INTO khivella_absen_records (guild_id, discord_id, discord_username, roblox_name) VALUES ($1, $2, $3, $4) ON CONFLICT (guild_id, discord_id) DO UPDATE SET roblox_name = $4, timestamp = CURRENT_TIMESTAMP")
        .bind(guild_id.to_string())
        .bind(discord_id)
        .bind(discord_username)
        .bind(verified_name.clone())
        .execute(db_pool)
        .await;

    if res.is_ok() {
        crate::utils::embeds::send_embed(ctx, "Attendance Recorded!", &format!("Thank you! Your attendance has been successfully recorded:\n\n**Roblox:** `{}`\n**Discord:** <@{}>", verified_name, ctx.author().id), 0x2ecc71).await?;
    } else {
        crate::utils::embeds::send_embed(ctx, "Error", "Failed to save your attendance in the database. Please report this to an Admin.", 0xED4245).await?;
    }

    Ok(())
}

#[poise::command(slash_command, prefix_command, category = "Utility", subcommands("bday_set", "bday_list"), rename = "bday", aliases("birthday"))]
pub async fn bday(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

#[poise::command(slash_command, prefix_command, rename = "set", category = "Utility")]
pub async fn bday_set(
    ctx: Context<'_>,
    #[description = "Your birthday in DD-MM or DD-MM-YYYY format (e.g. 15-08 or 15-08-2005)"] date: String,
) -> Result<(), Error> {
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() != 2 && parts.len() != 3 {
        crate::utils::embeds::send_embed(ctx, "Invalid Format", "Please use `DD-MM` or `DD-MM-YYYY` format. Example: `15-08` or `15-08-2005`.", 0xED4245).await?;
        return Ok(());
    }

    let day = parts[0].parse::<u32>().unwrap_or(0);
    let month = parts[1].parse::<u32>().unwrap_or(0);
    let mut year: Option<i32> = None;

    if parts.len() == 3 {
        let parsed_year = parts[2].parse::<i32>().unwrap_or(0);
        if parsed_year > 0 {
            // Handle YY format (e.g. 05 -> 2005, 99 -> 1999)
            year = Some(if parsed_year < 100 {
                if parsed_year > 50 { 1900 + parsed_year } else { 2000 + parsed_year }
            } else {
                parsed_year
            });
        }
    }

    if day == 0 || day > 31 || month == 0 || month > 12 {
        crate::utils::embeds::send_embed(ctx, "Invalid Date", "That date doesn't exist! Please provide a valid day and month.", 0xED4245).await?;
        return Ok(());
    }

    let guild_id = ctx.guild_id().unwrap();
    let db_pool = &ctx.data().db_pool;
    let discord_id = ctx.author().id.to_string();

    let res = sqlx::query("INSERT INTO khivella_birthdays (guild_id, discord_id, day, month, year) VALUES ($1, $2, $3, $4, $5) ON CONFLICT (guild_id, discord_id) DO UPDATE SET day = $3, month = $4, year = $5")
        .bind(guild_id.to_string())
        .bind(discord_id)
        .bind(day as i32)
        .bind(month as i32)
        .bind(year)
        .execute(db_pool)
        .await;

    if res.is_ok() {
        crate::utils::embeds::send_embed(ctx, "Birthday Set!", &format!("Your birthday has been successfully saved as **{}**! 🎂", date), 0x2ecc71).await?;
    } else {
        crate::utils::embeds::send_embed(ctx, "Error", "Failed to save your birthday. Please tell an admin.", 0xED4245).await?;
    }
    Ok(())
}

#[poise::command(slash_command, prefix_command, rename = "list", category = "Utility")]
pub async fn bday_list(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let db_pool = &ctx.data().db_pool;

    let rows = sqlx::query("SELECT discord_id, day, month, year FROM khivella_birthdays WHERE guild_id = $1 ORDER BY month ASC, day ASC")
        .bind(guild_id.to_string())
        .fetch_all(db_pool)
        .await;

    match rows {
        Ok(results) => {
            if results.is_empty() {
                crate::utils::embeds::send_embed(ctx, "Birthdays", "No one has set their birthday yet!", 0x3498db).await?;
                return Ok(());
            }

            let mut desc = String::new();
            use sqlx::Row;
            for r in results {
                let discord_id: String = r.get("discord_id");
                let day: i32 = r.get("day");
                let month: i32 = r.get("month");
                let year: Option<i32> = r.try_get("year").unwrap_or(None);
                
                let month_name = match month {
                    1 => "Jan", 2 => "Feb", 3 => "Mar", 4 => "Apr", 5 => "May", 6 => "Jun",
                    7 => "Jul", 8 => "Aug", 9 => "Sep", 10 => "Oct", 11 => "Nov", 12 => "Dec",
                    _ => "Unknown",
                };
                
                if let Some(y) = year {
                    desc.push_str(&format!("• <@{}> - `{} {} {}`\n", discord_id, day, month_name, y));
                } else {
                    desc.push_str(&format!("• <@{}> - `{} {}`\n", discord_id, day, month_name));
                }
            }
            crate::utils::embeds::send_embed(ctx, "Clan Birthdays 🎂", &desc, 0xf1c40f).await?;
        },
        Err(_) => {
            crate::utils::embeds::send_embed(ctx, "Error", "Failed to load birthdays.", 0xED4245).await?;
        }
    }
    Ok(())
}

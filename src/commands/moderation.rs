use crate::types::{Context, Error};
use crate::utils::embeds::send_embed;
use serenity::model::user::User;
use serenity::builder::GetMessages;
use crate::services::moderation::ModerationService;

#[poise::command(slash_command, prefix_command, default_member_permissions = "KICK_MEMBERS", required_permissions = "KICK_MEMBERS", category = "Moderation")]
pub async fn kick(
    ctx: Context<'_>, 
    #[description = "User to kick"] user: User,
    #[description = "Reason for kicking"] reason: Option<String>
) -> Result<(), Error> {
    let reason = reason.unwrap_or_else(|| "No reason provided.".to_string());

    if let Some(guild_id) = ctx.guild_id() {
        match guild_id.kick_with_reason(ctx.http(), user.id, &reason).await {
            Ok(_) => {
                send_embed(ctx, "Moderation: Kick", &format!("User {} has been kicked.\nReason: {}", user.name, reason), 0x2b2d31).await?;
            },
            Err(e) => {
                send_embed(ctx, "Error", &format!("Failed to kick user: {}", e), 0xED4245).await?;
            }
        }
    }
    Ok(())
}

#[poise::command(slash_command, prefix_command, default_member_permissions = "BAN_MEMBERS", required_permissions = "BAN_MEMBERS", category = "Moderation")]
pub async fn ban(
    ctx: Context<'_>, 
    #[description = "User to ban"] user: User,
    #[description = "Reason for banning"] reason: Option<String>
) -> Result<(), Error> {
    let reason = reason.unwrap_or_else(|| "No reason provided.".to_string());

    if let Some(guild_id) = ctx.guild_id() {
        match guild_id.ban_with_reason(ctx.http(), user.id, 0, &reason).await {
            Ok(_) => {
                send_embed(ctx, "Moderation: Ban", &format!("User {} has been banned.\nReason: {}", user.name, reason), 0x2b2d31).await?;
            },
            Err(e) => {
                send_embed(ctx, "Error", &format!("Failed to ban user: {}", e), 0xED4245).await?;
            }
        }
    }
    Ok(())
}

#[poise::command(slash_command, prefix_command, default_member_permissions = "BAN_MEMBERS", required_permissions = "BAN_MEMBERS", category = "Moderation")]
pub async fn unban(
    ctx: Context<'_>, 
    #[description = "ID of user to unban"] user_id: u64
) -> Result<(), Error> {
    if let Some(guild_id) = ctx.guild_id() {
        let uid = serenity::model::id::UserId::new(user_id);
        match guild_id.unban(ctx.http(), uid).await {
            Ok(_) => {
                send_embed(ctx, "Moderation: Unban", &format!("User ID {} has been unbanned.", user_id), 0x2b2d31).await?;
            },
            Err(e) => {
                send_embed(ctx, "Error", &format!("Failed to unban user: {}", e), 0xED4245).await?;
            }
        }
    }
    Ok(())
}

#[poise::command(slash_command, prefix_command, default_member_permissions = "MANAGE_MESSAGES", required_permissions = "MANAGE_MESSAGES", category = "Moderation")]
pub async fn purge(
    ctx: Context<'_>, 
    #[description = "Number of messages to delete (1-100)"] amount: u8
) -> Result<(), Error> {
    if amount == 0 || amount > 100 {
        send_embed(ctx, "Error", "Please provide a number between 1 and 100.", 0xED4245).await?;
        return Ok(());
    }

    let channel = ctx.channel_id();
    let builder = GetMessages::new().limit(amount);
    
    let messages = match channel.messages(ctx.http(), builder).await {
        Ok(msgs) => msgs,
        Err(e) => {
            send_embed(ctx, "Error", &format!("Failed to fetch messages: {}", e), 0xED4245).await?;
            return Ok(());
        }
    };
    
    if messages.is_empty() {
        send_embed(ctx, "Error", "No messages to delete.", 0xED4245).await?;
        return Ok(());
    }

    let message_ids: Vec<serenity::model::id::MessageId> = messages.iter().map(|m| m.id).collect();
    
    if let Err(e) = channel.delete_messages(ctx.http(), message_ids).await {
        send_embed(ctx, "Error", &format!("Failed to delete messages: {}", e), 0xED4245).await?;
        return Ok(());
    }

    let reply = ctx.send(poise::CreateReply::default().content(format!("✅ Deleted {} messages.", amount)).ephemeral(true)).await?;
    if ctx.prefix() != "/" {
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        let _ = reply.delete(ctx).await;
    }

    Ok(())
}

#[poise::command(slash_command, prefix_command, default_member_permissions = "MODERATE_MEMBERS", required_permissions = "MODERATE_MEMBERS", category = "Moderation")]
pub async fn timeout(
    ctx: Context<'_>, 
    #[description = "User to timeout"] user: User,
    #[description = "Duration in minutes"] minutes: i64
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    
    let timestamp = serenity::model::Timestamp::from_unix_timestamp(serenity::model::Timestamp::now().unix_timestamp() + (minutes * 60)).unwrap();
    
    let builder = serenity::builder::EditMember::new().disable_communication_until(timestamp.to_string());

    match guild_id.edit_member(ctx.http(), user.id, builder).await {
        Ok(_) => {
            send_embed(ctx, "Moderation: Timeout", &format!("User {} has been timed out for {} minutes.", user.name, minutes), 0x2b2d31).await?;
        },
        Err(e) => {
            send_embed(ctx, "Error", &format!("Failed to timeout user: {}", e), 0xED4245).await?;
        }
    }
    
    Ok(())
}

#[poise::command(slash_command, prefix_command, default_member_permissions = "MODERATE_MEMBERS", required_permissions = "MODERATE_MEMBERS", category = "Moderation")]
pub async fn warn(
    ctx: Context<'_>, 
    #[description = "User to warn"] user: User,
    #[description = "Reason for warning"] reason: String,
    #[description = "Evidence URL"] evidence: Option<String>
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let mod_service = ModerationService::new(ctx.data().db_pool.clone());
    
    match mod_service.warn_user(&guild_id.to_string(), &user.id.to_string(), &ctx.author().id.to_string(), &reason, evidence).await {
        Ok(case) => {
            send_embed(ctx, "Moderation: Warn", &format!("User {} has been warned.\nReason: {}\nCase ID: {}", user.name, reason, case.id), 0x2b2d31).await?;
        },
        Err(e) => {
            send_embed(ctx, "Error", &format!("Failed to record warning: {}", e), 0xED4245).await?;
        }
    }
    
    Ok(())
}

#[poise::command(slash_command, prefix_command, default_member_permissions = "MODERATE_MEMBERS", required_permissions = "MODERATE_MEMBERS", category = "Moderation")]
pub async fn strike(
    ctx: Context<'_>, 
    #[description = "User to strike"] user: User,
    #[description = "Reason for strike"] reason: String,
    #[description = "Evidence URL"] evidence: Option<String>
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let mod_service = ModerationService::new(ctx.data().db_pool.clone());
    
    match mod_service.strike_user(&guild_id.to_string(), &user.id.to_string(), &ctx.author().id.to_string(), &reason, evidence).await {
        Ok((case, active_strikes)) => {
            let mut desc = format!("User {} has been given a strike.\nReason: {}\nActive Strikes: {}\nCase ID: {}", user.name, reason, active_strikes, case.id);
            if let Ok(settings) = mod_service.get_settings(&guild_id.to_string()).await {
                if active_strikes >= settings.strike_ban_threshold {
                    let _ = guild_id.ban_with_reason(ctx.http(), user.id, 0, &format!("Reached strike threshold ({} strikes)", active_strikes)).await;
                    desc.push_str("\n\n🚨 **Action taken:** User has been BANNED due to strike threshold.");
                } else if active_strikes >= settings.strike_kick_threshold {
                    let _ = guild_id.kick_with_reason(ctx.http(), user.id, &format!("Reached strike threshold ({} strikes)", active_strikes)).await;
                    desc.push_str("\n\n⚠️ **Action taken:** User has been KICKED due to strike threshold.");
                }
            }

            send_embed(ctx, "Moderation: Strike", &desc, 0x2b2d31).await?;
        },
        Err(e) => {
            send_embed(ctx, "Error", &format!("Failed to record strike: {}", e), 0xED4245).await?;
        }
    }
    
    Ok(())
}

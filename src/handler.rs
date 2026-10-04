use serenity::client::Context as SerenityContext;
use crate::types::{Data, Error};

pub async fn event_handler(
    ctx: &SerenityContext,
    event: &serenity::all::FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    if let serenity::all::FullEvent::Message { new_message: msg } = event {
        if msg.author.bot {
            return Ok(());
        }

        let db_pool = &data.db_pool;

        // --- GALLERY & FORUM SYSTEM ---
        if let Some(guild_id) = msg.guild_id {
            let row = sqlx::query("SELECT emojis FROM khivella_galleries WHERE guild_id = $1 AND channel_id = $2")
                .bind(guild_id.to_string())
                .bind(msg.channel_id.to_string())
                .fetch_optional(db_pool)
                .await;

            if let Ok(Some(r)) = row {
                use sqlx::Row;
                let emojis_str: String = r.get("emojis");

                if msg.attachments.is_empty() {
                    let _ = msg.delete(&ctx.http).await;
                    return Ok(());
                } else {
                    let thread_name = format!("Comment on {}'s post", msg.author.name);
                    let builder = serenity::builder::CreateThread::new(thread_name);
                    let _ = msg.channel_id.create_thread_from_message(&ctx.http, msg.id, builder).await;
                    
                    // Auto React
                    for emoji_part in emojis_str.split_whitespace() {
                        let reaction = if emoji_part.starts_with("<:") || emoji_part.starts_with("<a:") {
                            if let Ok(reaction_type) = std::str::FromStr::from_str(emoji_part) {
                                Some(reaction_type)
                            } else { None }
                        } else {
                            Some(serenity::all::ReactionType::Unicode(emoji_part.to_string()))
                        };

                        if let Some(r) = reaction {
                            let _ = msg.react(&ctx.http, r).await;
                        }
                    }
                }
            }
        }
        // ----------------------

        if let Some(guild_id) = msg.guild_id {
            let db_pool = &data.db_pool;
            let msg_content = msg.content.to_lowercase();
            
            let autoreply = sqlx::query(
                "SELECT response, media_url FROM khivella_autoreplies WHERE guild_id = $1 AND trigger = $2"
            )
            .bind(guild_id.to_string())
            .bind(&msg_content)
            .fetch_optional(db_pool)
            .await;

            if let Ok(Some(row)) = autoreply {
                use sqlx::Row;
                let response_opt: Option<String> = row.try_get("response").unwrap_or(None);
                let media_opt: Option<String> = row.try_get("media_url").unwrap_or(None);
                
                let mut final_content = String::new();
                if let Some(resp) = response_opt {
                    if !resp.is_empty() {
                        final_content.push_str(&resp);
                    }
                }
                if let Some(media) = media_opt {
                    if !media.is_empty() {
                        if !final_content.is_empty() {
                            final_content.push('\n');
                        }
                        final_content.push_str(&media);
                    }
                }

                if !final_content.is_empty() {
                    let _ = msg.channel_id.send_message(&ctx.http, serenity::builder::CreateMessage::new().content(final_content)).await;
                }
            }

            if format!("{:?}", msg.kind).contains("PremiumGuild") {
                let row = sqlx::query("SELECT channel_id, style, text, background_url FROM khivella_booster WHERE guild_id = $1")
                    .bind(guild_id.to_string()).fetch_optional(db_pool).await;

                if let Ok(Some(r)) = row {
                    use sqlx::Row;
                    let channel_id: Option<String> = r.try_get("channel_id").unwrap_or(None);
                    let text: Option<String> = r.try_get("text").unwrap_or(None);
                    
                    let mut msg_text = text.unwrap_or_else(|| "Thank you {username} for boosting the server!".to_string());
                    msg_text = msg_text.replace("{username}", &msg.author.name);
                    
                    if let Some(cid) = channel_id {
                        if let Ok(id) = cid.parse::<u64>() {
                            let channel = serenity::model::id::ChannelId::new(id);
                            let _ = channel.send_message(&ctx.http, serenity::builder::CreateMessage::new().content(msg_text)).await;
                        }
                    }
                }
            }

            let sticky_row = sqlx::query("SELECT message, last_message_id FROM khivella_sticky WHERE guild_id = $1 AND channel_id = $2")
                .bind(guild_id.to_string())
                .bind(msg.channel_id.to_string())
                .fetch_optional(db_pool)
                .await;

            if let Ok(Some(r)) = sticky_row {
                use sqlx::Row;
                let message: String = r.get("message");
                let last_message_id: Option<String> = r.try_get("last_message_id").unwrap_or(None);

                if let Some(last_id) = last_message_id {
                    if let Ok(lid) = last_id.parse::<u64>() {
                        let _ = ctx.http.delete_message(msg.channel_id, serenity::model::id::MessageId::new(lid), None).await;
                    }
                }

                if let Ok(new_msg) = msg.channel_id.send_message(&ctx.http, serenity::builder::CreateMessage::new().content(message)).await {
                    let _ = sqlx::query("UPDATE khivella_sticky SET last_message_id = $1 WHERE guild_id = $2 AND channel_id = $3")
                        .bind(new_msg.id.to_string())
                        .bind(guild_id.to_string())
                        .bind(msg.channel_id.to_string())
                        .execute(db_pool)
                        .await;
                }
            }
        }

        let is_enabled = *data.chatbot_enabled.read().await;

        if is_enabled {
            let bot_id = ctx.cache.current_user().id;
            
            let is_mention = msg.mentions.iter().any(|u| u.id == bot_id);
            let is_reply = if let Some(ref_msg) = &msg.referenced_message {
                ref_msg.author.id == bot_id
            } else {
                false
            };
            
            let contains_porpan = msg.content.to_lowercase().contains("porpan");

            if is_mention || is_reply || contains_porpan {
                let mut prompt = msg.content.clone();
                let mention_tag1 = format!("<@{}>", bot_id);
                let mention_tag2 = format!("<@!{}>", bot_id);
                prompt = prompt.replace(&mention_tag1, "").replace(&mention_tag2, "").trim().to_string();

                if prompt.is_empty() {
                    prompt = "Halo 4FunBot!".to_string();
                }

                let _ = msg.channel_id.broadcast_typing(&ctx.http).await;
                crate::utils::ai::handle_chat(ctx, msg, data, &prompt).await;
            }
        }
    }
    
    Ok(())
}

use crate::types::{Context, Error};
use crate::utils::embeds::send_embed;
use songbird::input::{Compose, YoutubeDl};
use serenity::all::Mentionable;

#[poise::command(slash_command, prefix_command, guild_only, category = "Music")]
pub async fn join(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let channel_id = {
        let guild = ctx.guild().unwrap();
        guild.voice_states.get(&ctx.author().id)
            .and_then(|voice_state| voice_state.channel_id)
    };

    let connect_to = match channel_id {
        Some(channel) => channel,
        None => {
            send_embed(ctx, "Error", "You need to join a voice channel first.", 0xED4245).await?;
            return Ok(());
        }
    };

    let manager = songbird::get(ctx.serenity_context()).await.unwrap().clone();
    
    if let Ok(_handler_lock) = manager.join(guild_id, connect_to).await {
        send_embed(ctx, "Voice Channel", &format!("Successfully connected to {}.", connect_to.mention()), 0x3498db).await?;
    } else {
        send_embed(ctx, "Error", "Failed to join the voice channel.", 0xED4245).await?;
    }

    Ok(())
}

#[poise::command(slash_command, prefix_command, guild_only, category = "Music")]
pub async fn leave(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let manager = songbird::get(ctx.serenity_context()).await.unwrap().clone();
    let has_handler = manager.get(guild_id).is_some();

    if has_handler {
        if let Err(e) = manager.remove(guild_id).await {
            send_embed(ctx, "Error", &format!("Failed to leave the voice channel: {:?}", e), 0xED4245).await?;
        } else {
            send_embed(ctx, "Voice Channel", "Disconnected from the voice channel.", 0x3498db).await?;
        }
    } else {
        send_embed(ctx, "Error", "I am not in a voice channel.", 0xED4245).await?;
    }

    Ok(())
}

use songbird::events::{Event, EventContext, EventHandler as VoiceEventHandler, TrackEvent};
use serenity::async_trait;

struct TrackErrorNotifier;

#[async_trait]
impl VoiceEventHandler for TrackErrorNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(track_list) = ctx {
            for (state, _handle) in *track_list {
                tracing::error!("Track encountered an error during playback! State: {:?}", state.playing);
            }
        }
        None
    }
}

#[poise::command(slash_command, prefix_command, guild_only, category = "Music")]
pub async fn play(
    ctx: Context<'_>, 
    #[description = "Search query or URL"] 
    #[rest] query: String
) -> Result<(), Error> {
    ctx.defer().await?;
    let guild_id = ctx.guild_id().unwrap();
    let manager = songbird::get(ctx.serenity_context()).await.unwrap().clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;

        let http_client = reqwest::Client::new();
        let query_text = if query.starts_with("http") {
            query
        } else {
            format!("ytsearch:{}", query)
        };
        
        let mut src = YoutubeDl::new(http_client, query_text);
        
        let metadata = match src.aux_metadata().await {
            Ok(m) => m,
            Err(_) => {
                send_embed(ctx, "Error", "Failed to fetch track metadata.", 0xED4245).await?;
                return Ok(());
            }
        };

        let title = metadata.title.clone().unwrap_or_else(|| "Unknown Track".to_string());
        let channel = metadata.channel.clone().unwrap_or_else(|| "Unknown Artist".to_string());
        let duration = metadata.duration.map(|d| format!("{:02}:{:02}", d.as_secs() / 60, d.as_secs() % 60)).unwrap_or_else(|| "Live".to_string());
        let url = metadata.source_url.clone().unwrap_or_else(|| "https://youtube.com".to_string());

        let track_handle = handler.enqueue_input(src.into()).await;
        let _ = track_handle.add_event(Event::Track(TrackEvent::Error), TrackErrorNotifier);
        
        let embed = serenity::builder::CreateEmbed::new()
            .title("🎶 Added to Queue")
            .description(format!("**[{}]({})**", title, url))
            .color(0x3498db)
            .field("Channel", channel, true)
            .field("Duration", duration, true)
            .field("Position", format!("{}", handler.queue().len()), true);

        ctx.send(poise::CreateReply::default().embed(embed)).await?;
    } else {
        send_embed(ctx, "Error", "I am not in a voice channel. Use `/join` first.", 0xED4245).await?;
    }

    Ok(())
}

#[poise::command(slash_command, prefix_command, guild_only, category = "Music")]
pub async fn nowplaying(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let manager = songbird::get(ctx.serenity_context()).await.unwrap().clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        
        if let Some(current) = queue.current() {
            let state = current.get_info().await.unwrap();
            let metadata = current.metadata().clone();
            
            let title = metadata.title.unwrap_or_else(|| "Unknown Track".to_string());
            let url = metadata.source_url.unwrap_or_else(|| "https://youtube.com".to_string());
            
            let current_pos = state.position.as_secs();
            let total_dur = metadata.duration.map(|d| d.as_secs()).unwrap_or(0);
            
            let progress_bar = if total_dur > 0 {
                let percent = (current_pos as f64 / total_dur as f64) * 20.0;
                let mut bar = String::new();
                for i in 0..20 {
                    if i == percent as usize {
                        bar.push('🔘');
                    } else if i < percent as usize {
                        bar.push('▬');
                    } else {
                        bar.push('➖');
                    }
                }
                format!("{} `[{:02}:{:02} / {:02}:{:02}]`", bar, current_pos / 60, current_pos % 60, total_dur / 60, total_dur % 60)
            } else {
                "Live Stream".to_string()
            };

            let embed = serenity::builder::CreateEmbed::new()
                .title("🎧 Now Playing")
                .description(format!("**[{}]({})**\n\n{}", title, url, progress_bar))
                .color(0x3498db);
                
            ctx.send(poise::CreateReply::default().embed(embed)).await?;
        } else {
            send_embed(ctx, "Now Playing", "Nothing is currently playing.", 0x3498db).await?;
        }
    } else {
        send_embed(ctx, "Error", "I am not in a voice channel.", 0xED4245).await?;
    }

    Ok(())
}

#[poise::command(slash_command, prefix_command, guild_only, category = "Music")]
pub async fn volume(
    ctx: Context<'_>, 
    #[description = "Volume level (1-100)"] vol: f32
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let manager = songbird::get(ctx.serenity_context()).await.unwrap().clone();

    if vol < 1.0 || vol > 100.0 {
        send_embed(ctx, "Error", "Volume must be between 1 and 100.", 0xED4245).await?;
        return Ok(());
    }

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        
        if let Some(current) = queue.current() {
            let _ = current.set_volume(vol / 100.0);
            send_embed(ctx, "Volume", &format!("🔊 Volume set to **{}%**", vol), 0x3498db).await?;
        } else {
            send_embed(ctx, "Error", "Nothing is playing right now.", 0xED4245).await?;
        }
    } else {
        send_embed(ctx, "Error", "I am not in a voice channel.", 0xED4245).await?;
    }

    Ok(())
}

#[poise::command(slash_command, prefix_command, guild_only, category = "Music")]
pub async fn pause(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let manager = songbird::get(ctx.serenity_context()).await.unwrap().clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        
        if queue.is_empty() {
            send_embed(ctx, "Playback", "Queue is empty.", 0x3498db).await?;
            return Ok(());
        }

        let _ = queue.pause();
        send_embed(ctx, "Playback", "⏸️ Paused the current track.", 0x3498db).await?;
    } else {
        send_embed(ctx, "Error", "I am not in a voice channel.", 0xED4245).await?;
    }

    Ok(())
}

#[poise::command(slash_command, prefix_command, guild_only, category = "Music")]
pub async fn resume(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let manager = songbird::get(ctx.serenity_context()).await.unwrap().clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        
        if queue.is_empty() {
            send_embed(ctx, "Playback", "Queue is empty.", 0x3498db).await?;
            return Ok(());
        }

        let _ = queue.resume();
        send_embed(ctx, "Playback", "▶️ Resumed the current track.", 0x3498db).await?;
    } else {
        send_embed(ctx, "Error", "I am not in a voice channel.", 0xED4245).await?;
    }

    Ok(())
}

#[poise::command(slash_command, prefix_command, guild_only, category = "Music")]
pub async fn skip(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let manager = songbird::get(ctx.serenity_context()).await.unwrap().clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        
        if queue.is_empty() {
            send_embed(ctx, "Playback", "Queue is empty.", 0x3498db).await?;
            return Ok(());
        }

        let _ = queue.skip();
        send_embed(ctx, "Playback", "⏭️ Skipped the current track.", 0x3498db).await?;
    } else {
        send_embed(ctx, "Error", "I am not in a voice channel.", 0xED4245).await?;
    }

    Ok(())
}

#[poise::command(slash_command, prefix_command, guild_only, category = "Music")]
pub async fn stop(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let manager = songbird::get(ctx.serenity_context()).await.unwrap().clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        
        queue.stop();
        send_embed(ctx, "Playback", "⏹️ Stopped playing and cleared the queue.", 0x3498db).await?;
    } else {
        send_embed(ctx, "Error", "I am not in a voice channel.", 0xED4245).await?;
    }

    Ok(())
}

#[poise::command(slash_command, prefix_command, guild_only, category = "Music")]
pub async fn queue(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let manager = songbird::get(ctx.serenity_context()).await.unwrap().clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        let tracks = queue.current_queue();
        
        if tracks.is_empty() {
            send_embed(ctx, "Queue", "The queue is currently empty.", 0x3498db).await?;
            return Ok(());
        }

        let mut desc = String::new();
        
        for (i, track) in tracks.iter().enumerate().take(10) {
            let meta = track.metadata();
            let title = meta.title.clone().unwrap_or_else(|| "Unknown".to_string());
            let dur = meta.duration.map(|d| format!("{:02}:{:02}", d.as_secs() / 60, d.as_secs() % 60)).unwrap_or_else(|| "00:00".to_string());
            
            if i == 0 {
                desc.push_str(&format!("🎵 **Currently Playing:**\n**{}** (`{}`)\n\n**Up Next:**\n", title, dur));
            } else {
                desc.push_str(&format!("`{}.` **{}** (`{}`)\n", i, title, dur));
            }
        }
        
        if tracks.len() > 10 {
            desc.push_str(&format!("\n*...and {} more tracks*", tracks.len() - 10));
        }
        
        let embed = serenity::builder::CreateEmbed::new()
            .title("📑 Music Queue")
            .description(desc)
            .color(0x3498db);

        ctx.send(poise::CreateReply::default().embed(embed)).await?;
    } else {
        send_embed(ctx, "Error", "I am not in a voice channel.", 0xED4245).await?;
    }

    Ok(())
}

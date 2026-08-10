use crate::config::discord_limits;
use crate::{Context, Data};
use poise::serenity_prelude as serenity;

#[derive(Clone, Copy)]
pub enum Tone {
    Primary,
    Success,
    Warning,
    Error,
    Neutral,
}

fn color(data: &Data, tone: Tone) -> u32 {
    match tone {
        Tone::Primary => data.config.colors.primary,
        Tone::Success => data.config.colors.success,
        Tone::Warning => data.config.colors.warning,
        Tone::Error => data.config.colors.error,
        Tone::Neutral => data.config.colors.neutral,
    }
}

pub fn embed(data: &Data, tone: Tone) -> serenity::CreateEmbed {
    serenity::CreateEmbed::new().color(color(data, tone))
}

pub fn panel(data: &Data, tone: Tone, description: impl Into<String>) -> serenity::CreateEmbed {
    embed(data, tone).description(truncate(
        &description.into(),
        discord_limits::EMBED_DESCRIPTION_CHARS,
    ))
}

fn truncate(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_owned();
    }
    let mut truncated = value
        .chars()
        .take(max_chars.saturating_sub(1))
        .collect::<String>();
    if max_chars > 0 {
        truncated.push('…');
    }
    truncated
}

pub fn reply_builder(
    data: &Data,
    tone: Tone,
    description: impl Into<String>,
) -> poise::CreateReply {
    embed_reply(panel(data, tone, description))
}

pub fn embed_reply(embed: serenity::CreateEmbed) -> poise::CreateReply {
    poise::CreateReply::default()
        .embed(embed)
        .allowed_mentions(serenity::CreateAllowedMentions::new())
}

pub async fn reply<'a>(
    ctx: Context<'a>,
    tone: Tone,
    description: impl Into<String>,
) -> Result<poise::ReplyHandle<'a>, serenity::Error> {
    ctx.send(reply_builder(ctx.data(), tone, description)).await
}

pub async fn private_reply<'a>(
    ctx: Context<'a>,
    tone: Tone,
    description: impl Into<String>,
) -> Result<poise::ReplyHandle<'a>, serenity::Error> {
    ctx.send(reply_builder(ctx.data(), tone, description).ephemeral(true))
        .await
}

#[cfg(test)]
mod tests {
    use super::truncate;

    #[test]
    fn truncates_embed_text_on_unicode_boundaries() {
        assert_eq!(truncate("aé日", 3), "aé日");
        assert_eq!(truncate("aé日本", 3), "aé…");
        assert_eq!(truncate("anything", 0), "");
    }
}

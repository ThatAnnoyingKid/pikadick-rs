use crate::{
    PoiseContext,
    PoiseError,
};

fn vaporwave_str(data: &str) -> String {
    data.chars()
        .map(|ch| {
            let ch_u32 = u32::from(ch);
            if (33..=270).contains(&ch_u32) {
                ch_u32
                    .checked_add(65_248)
                    .and_then(char::from_u32)
                    .unwrap_or(ch)
            } else {
                ' '
            }
        })
        .collect()
}

#[poise::command(
    slash_command,
    description_localized("en-US", "Vaporwave a phrase"),
    check = "crate::checks::enabled"
)]
pub async fn vaporwave(
    ctx: PoiseContext<'_>,
    #[description = "The phrase to vaporwave."] phrase: String,
) -> Result<(), PoiseError> {
    ctx.reply(vaporwave_str(&phrase)).await?;
    Ok(())
}

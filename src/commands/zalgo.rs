use crate::{
    PoiseContext,
    PoiseError,
};
use zalgo::ZalgoBuilder;

#[poise::command(
    slash_command,
    description_localized("en-US", "Zalgoify a phrase"),
    check = "crate::checks::enabled"
)]
pub async fn zalgo(
    ctx: PoiseContext<'_>,
    #[description = "The phrase to zalgoify."] phrase: String,
    #[description = "The length of the output phrase. Defaults to 2,000."] length: Option<u16>,
) -> Result<(), PoiseError> {
    let output_length = length.unwrap_or(2_000);

    let phrase_length = phrase.chars().count();
    let total = (f64::from(output_length) - phrase_length as f64) / phrase_length as f64;
    let max = (total / 3.0) as usize;

    if max == 0 {
        ctx.reply("The phrase cannot be zalgoified within the given limits.")
            .await?;
        return Ok(());
    }

    let output = ZalgoBuilder::new()
        .set_up(max)
        .set_down(max)
        .set_mid(max)
        .zalgoify(&phrase);

    ctx.reply(output).await?;

    Ok(())
}

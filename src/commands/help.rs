use crate::{
    PoiseContext,
    PoiseError,
};

#[poise::command(slash_command)]
pub async fn help(
    ctx: PoiseContext<'_>,
    #[description = "Specific command to show help about"] command: Option<String>,
) -> Result<(), PoiseError> {
    let config = poise::builtins::HelpConfiguration {
        ..Default::default()
    };
    poise::builtins::help(ctx, command.as_deref(), config).await?;
    Ok(())
}

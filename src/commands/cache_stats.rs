use crate::BotContext;
use anyhow::Context as _;
use pikadick_slash_framework::ClientData;
use std::fmt::Write;
use tracing::info;
use twilight_model::http::interaction::{
    InteractionResponse,
    InteractionResponseType,
};
use twilight_util::builder::{
    embed::{
        EmbedBuilder,
        EmbedFieldBuilder,
    },
    InteractionResponseDataBuilder,
};

pub fn create_slash_command() -> anyhow::Result<pikadick_slash_framework::Command<BotContext>> {
    pikadick_slash_framework::CommandBuilder::<BotContext>::new()
        .name("cache-stats")
        .description("Get cache usage stats")
        .on_process(|client_data, interaction, _args: ()| async move {
            let interaction_client = client_data.interaction_client();
            let stats = client_data.generate_cache_stats();
            let mut response_data = InteractionResponseDataBuilder::new();

            info!("reporting all cache stats");

            let mut embed_builder = EmbedBuilder::new().title("Cache Stats").color(0xFF_00_00);
            for (stat_family_name, stat_family) in stats.into_iter() {
                // Low ball, but better than nothing
                let mut output = String::with_capacity(stat_family.len() * 16);

                for (stat_name, stat) in stat_family.iter() {
                    writeln!(&mut output, "**{stat_name}**: {stat} item(s)")?;
                }

                embed_builder =
                    embed_builder.field(EmbedFieldBuilder::new(stat_family_name, output));
            }
            let embed = embed_builder.build();
            response_data = response_data.embeds(std::iter::once(embed));

            let response_data = response_data.build();
            let response = InteractionResponse {
                kind: InteractionResponseType::ChannelMessageWithSource,
                data: Some(response_data),
            };
            interaction_client
                .create_response(interaction.id, interaction.token.as_str(), &response)
                .exec()
                .await
                .context("failed to send response")?;

            Ok(())
        })
        .build()
        .context("failed to build command")
}

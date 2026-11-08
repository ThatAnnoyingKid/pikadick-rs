use super::autocomplete_user;
use crate::{
    PoiseContext,
    PoiseError,
    util::SimpleCanvas,
};
use anyhow::Context;
use poise::{
    CreateReply,
    serenity_prelude::CreateAttachment,
};
use skrifa::{
    MetadataProvider,
    prelude::{
        LocationRef,
        Size as SkrifaSize,
    },
};
use std::sync::{
    Arc,
    LazyLock,
};
use tracing::warn;
use vello_cpu::{
    Glyph,
    Pixmap,
    kurbo::{
        BezPath,
        Rect,
    },
    peniko::{
        Blob,
        FontData,
        color::{
            AlphaColor,
            Srgb,
        },
    },
};

const fn hex_color_const(data: u32) -> AlphaColor<Srgb> {
    let r = (data >> 16) & 0xFF;
    let g = (data >> 8) & 0xFF;
    let b = data & 0xFF;

    AlphaColor::new([r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0])
}

static FONT_REF: LazyLock<skrifa::FontRef<'static>> = LazyLock::new(|| {
    skrifa::FontRef::new(crate::constants::ROBOTO_MEDIUM_TTF_BYTES).expect("Failed to parse font")
});
const MARGIN: u16 = 20;
const ICON_SIZE: u16 = 300;
const WIDTH: u16 = 1000;
const HEIGHT: u16 = 300 + (MARGIN * 2);
/// #121212
const BACKGROUND_COLOR: AlphaColor<Srgb> = hex_color_const(0x121212);
// #E8E1D2
const PRIMARY_TEXT_COLOR: AlphaColor<Srgb> = hex_color_const(0xE8E1D2);

fn capitalize(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut chars = input.chars();
    let first = match chars.next() {
        Some(ch) => ch.to_ascii_uppercase(),
        None => return output,
    };
    output.push(first);
    output.extend(chars);
    output
}

fn squircle_path(size: f64, control: f64) -> BezPath {
    let mut path = BezPath::new();

    let c = size / 2.0;
    let k = size * control;

    path.move_to((c, 0.0));
    path.curve_to((c + k, 0.0), (size, c - k), (size, c));
    path.curve_to((size, c + k), (c + k, size), (c, size));
    path.curve_to((c - k, size), (0.0, c + k), (0.0, c));
    path.curve_to((0.0, c - k), (c - k, 0.0), (c, 0.0));

    path.close_path();
    path
}

fn get_font_line_height(font_size: f32) -> f32 {
    let metrics = FONT_REF.metrics(SkrifaSize::new(font_size), LocationRef::default());
    metrics.ascent - metrics.descent + metrics.leading
}

#[derive(Debug)]
pub(super) struct UserStatsRenderer {
    canvas: SimpleCanvas,
    font_blob: Blob<u8>,
}

impl UserStatsRenderer {
    pub fn new() -> Self {
        let canvas = SimpleCanvas::new(WIDTH, HEIGHT);
        let font = Arc::new(crate::constants::ROBOTO_MEDIUM_TTF_BYTES);
        let font_blob = Blob::new(font);

        Self { canvas, font_blob }
    }

    fn draw_background(&mut self) {
        self.canvas.context.set_paint(BACKGROUND_COLOR);
        self.canvas
            .context
            .fill_rect(&Rect::new(0.0, 0.0, f64::from(WIDTH), f64::from(HEIGHT)));
    }

    fn draw_profile_icon(&mut self, icon_png: Vec<u8>) -> anyhow::Result<()> {
        let pixmap = Pixmap::from_png(std::io::Cursor::new(icon_png))?;
        let clip_path = squircle_path(pixmap.width().into(), 0.49);
        self.canvas.draw_image(
            pixmap,
            MARGIN.into(),
            MARGIN.into(),
            ICON_SIZE.into(),
            ICON_SIZE.into(),
            Some(&clip_path),
        );

        Ok(())
    }

    fn draw_text(&mut self, text: &str, x: f32, y: f32, font_size: f32) -> anyhow::Result<Rect> {
        self.canvas.context.set_paint(PRIMARY_TEXT_COLOR);

        let charmap = FONT_REF.charmap();
        let glyph_metrics =
            FONT_REF.glyph_metrics(SkrifaSize::new(font_size), LocationRef::default());

        let mut current_x = x;
        let mut mapped_glyphs = Vec::new();

        let mut min_x = x;
        let mut min_y = y;
        let mut max_x = x;
        let mut max_y = y;

        for ch in text.chars() {
            if let Some(glyph_id) = charmap.map(ch) {
                mapped_glyphs.push(Glyph {
                    id: glyph_id.to_u32(),
                    x: current_x,
                    y,
                });

                let bounds = glyph_metrics.bounds(glyph_id);
                min_x = min_x.min(x + bounds.map_or(0.0, |bounds| bounds.x_min));
                min_y = min_y.min(y + bounds.map_or(0.0, |bounds| bounds.y_min));
                max_x = max_x.max(current_x + bounds.map_or(0.0, |bounds| bounds.x_max));
                max_y = max_y.max(y + bounds.map_or(0.0, |bounds| bounds.y_max));

                let advance = glyph_metrics.advance_width(glyph_id).unwrap_or(0.0);
                current_x += advance;
            }
        }

        self.canvas
            .context
            .glyph_run(
                &mut self.canvas.resources,
                &FontData {
                    data: self.font_blob.clone(),
                    index: 0,
                },
            )
            .font_size(font_size)
            .hint(true)
            .fill_glyphs(mapped_glyphs.into_iter())?;

        Ok(Rect::new(
            min_x.into(),
            min_y.into(),
            max_x.into(),
            max_y.into(),
        ))
    }

    pub fn render(
        &mut self,
        user: ugg::model::User,
        icon_png: Vec<u8>,
        _rank_icon: Option<Vec<u8>>,
    ) -> anyhow::Result<Vec<u8>> {
        const NAME_FONT_SIZE: f32 = 48.0;
        const LEVEL_FONT_SIZE: f32 = 32.0;
        const REGION_FONT_SIZE: f32 = 32.0;
        const RANK_FONT_SIZE: f32 = 32.0;

        let name_text = format!("{}#{}", user.riot_user_name, user.riot_tag_line);
        let level_text = format!("Level: {}", user.summoner_level);
        let region_text = format!("Region: {}", user.region_id.to_ascii_uppercase());
        let rank_text = match (user.tier, user.rank, user.lp) {
            (Some(tier), Some(rank), Some(lp)) => {
                let tier = capitalize(&tier);
                let rank = rank.to_ascii_uppercase();
                Some(format!("Rank: {tier} {rank}, {lp} LP"))
            }
            _ => None,
        };

        self.canvas.reset();

        self.draw_background();
        self.draw_profile_icon(icon_png)?;

        let start_x = f32::from(MARGIN * 2 + ICON_SIZE);
        let mut start_y = get_font_line_height(NAME_FONT_SIZE);
        self.draw_text(name_text.as_str(), start_x, start_y, NAME_FONT_SIZE)?;
        start_y += get_font_line_height(LEVEL_FONT_SIZE);
        self.draw_text(level_text.as_str(), start_x, start_y, LEVEL_FONT_SIZE)?;
        start_y += get_font_line_height(REGION_FONT_SIZE);
        self.draw_text(region_text.as_str(), start_x, start_y, REGION_FONT_SIZE)?;
        if let Some(rank_text) = rank_text {
            start_y += get_font_line_height(RANK_FONT_SIZE);
            self.draw_text(rank_text.as_str(), start_x, start_y, RANK_FONT_SIZE)?;
        }

        let target = self.canvas.render();

        Ok(target.into_png()?)
    }
}

#[poise::command(
    slash_command,
    description_localized("en-US", "Get lol stats for a user.")
)]
pub async fn user(
    ctx: PoiseContext<'_>,
    #[description = "The user to look up stats for."]
    #[autocomplete = "autocomplete_user"]
    user: String,
    #[description = "Refresh user stats."] refresh: Option<bool>,
) -> Result<(), PoiseError> {
    let refresh = refresh.unwrap_or(false);

    let ugg_client = &ctx.data().ugg_client;

    ctx.defer().await?;

    let result = ugg_client
        .get_profile_suggestion(&user)
        .await
        .context("Failed to get user");
    let mut profile_suggestion = match result {
        Ok(user) => user,
        Err(error) => {
            warn!("{error:?}");
            ctx.reply(format!("{error:?}")).await?;

            return Ok(());
        }
    };
    if refresh {
        let result = ugg_client
            .update_player_profile(
                &profile_suggestion.riot_user_name,
                &profile_suggestion.riot_tag_line,
            )
            .await
            .context("Failed to update player profile");
        if let Err(error) = result {
            warn!("{error:?}");
            ctx.reply(format!("{error:?}")).await?;
            return Ok(());
        }

        let result = ugg_client
            .get_profile_suggestion(&user)
            .await
            .context("Failed to get user");
        let new_profile_suggestion = match result {
            Ok(user) => user,
            Err(error) => {
                warn!("{error:?}");
                ctx.reply(format!("{error:?}")).await?;

                return Ok(());
            }
        };
        profile_suggestion = new_profile_suggestion;
    }

    let file_name = format!(
        "lol-profile-{}-{}.png",
        profile_suggestion.riot_user_name, profile_suggestion.riot_tag_line
    );

    let png = ugg_client.render_user_stats(profile_suggestion).await?;

    let attachment = CreateAttachment::bytes(png, file_name);
    let create_reply = CreateReply::new().attachment(attachment);

    ctx.send(create_reply).await?;

    Ok(())
}

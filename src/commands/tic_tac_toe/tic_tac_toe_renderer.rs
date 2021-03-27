use anyhow::Context;
use log::info;
use minimax::tic_tac_toe::{
    TicTacToeIter,
    TicTacToeTeam,
};
use std::{
    sync::Arc,
    time::Instant,
};
use tokio::sync::Semaphore;

const FONT_BYTES: &[u8] = include_bytes!("../../../assets/Roboto/Roboto-Thin.ttf");

const RENDERED_SIZE: u16 = 300;
const SQUARE_SIZE: u16 = RENDERED_SIZE / 3;

const MAX_PARALLEL_RENDER_LIMIT: usize = 4;

/// Render a Tic-Tac-Toe board
#[derive(Debug, Clone)]
pub(crate) struct TicTacToeRenderer {
    background_pixmap: Arc<tiny_skia::Pixmap>,
    number_paths: Arc<Vec<tiny_skia::Path>>,

    render_semaphore: Arc<Semaphore>,
}

#[allow(clippy::new_without_default)]
impl TicTacToeRenderer {
    /// Make a new [`TicTacToeRenderer`].
    pub(crate) fn new() -> anyhow::Result<Self> {
        let font_face =
            ttf_parser::Face::from_slice(FONT_BYTES, 0).with_context(|| "Invalid Font")?;

        let mut background_pixmap =
            tiny_skia::Pixmap::new(RENDERED_SIZE.into(), RENDERED_SIZE.into())
                .with_context(|| "Failed to create background pixmap")?;

        let mut paint = tiny_skia::Paint::default();
        for i in 0..3 {
            for j in 0..3 {
                let x = i * SQUARE_SIZE;
                let y = j * SQUARE_SIZE;
                let square = tiny_skia::Rect::from_xywh(
                    x as f32,
                    y as f32,
                    SQUARE_SIZE as f32,
                    SQUARE_SIZE as f32,
                )
                .with_context(|| "Failed to make square")?;

                if (j * 3 + i) % 2 == 0 {
                    paint.set_color_rgba8(255, 0, 0, 255);
                } else {
                    paint.set_color_rgba8(119, 119, 119, 255);
                }

                background_pixmap
                    .fill_rect(square, &paint, tiny_skia::Transform::identity(), None)
                    .with_context(|| "Failed to fill square")?;
            }
        }

        let mut number_paths = Vec::with_capacity(9);
        let mut paint = tiny_skia::Paint::default();
        paint.set_color_rgba8(255, 255, 255, 255);
        for i in b'0'..b'9' {
            let glyph_id = font_face
                .glyph_index(char::from(i))
                .with_context(|| format!("Missing glyph for '{}'", char::from(i)))?;

            let mut builder = SkiaBuilder::new();
            let _bb = font_face
                .outline_glyph(glyph_id, &mut builder)
                .with_context(|| format!("Missing glyph bounds for '{}'", char::from(i)))?;
            let path = builder.into_path().with_context(|| {
                format!("Failed to generate glyph path for '{}'", char::from(i))
            })?;

            number_paths.push(path);
        }

        Ok(Self {
            background_pixmap: Arc::new(background_pixmap),
            number_paths: Arc::new(number_paths),
            render_semaphore: Arc::new(Semaphore::new(MAX_PARALLEL_RENDER_LIMIT)),
        })
    }

    /// Render a Tic-Tac-Toe board with `tiny_skia`.
    pub(crate) fn render_board(&self, state: u16) -> anyhow::Result<Vec<u8>> {
        let draw_start = Instant::now();
        let mut pixmap = self.background_pixmap.as_ref().as_ref().to_owned();

        let mut paint = tiny_skia::Paint::default();
        let mut stroke = tiny_skia::Stroke::default();
        // Author might add more fields
        #[allow(clippy::field_reassign_with_default)]
        {
            paint.anti_alias = true;
            stroke.width = 4.0;
        }
        for (i, team) in TicTacToeIter::new(state).enumerate() {
            let transform = tiny_skia::Transform::from_translate(
                ((i % 3) * usize::from(SQUARE_SIZE)) as f32,
                ((i / 3) * usize::from(SQUARE_SIZE)) as f32,
            );

            if let Some(team) = team {
                paint.set_color_rgba8(0, 0, 0, 255);
                let path = match team {
                    TicTacToeTeam::X => {
                        let mut path_builder = tiny_skia::PathBuilder::new();
                        path_builder.move_to(0.0, 0.0);
                        path_builder.line_to(100.0, 100.0);
                        path_builder.move_to(0.0, 100.0);
                        path_builder.line_to(100.0, 0.0);
                        path_builder.finish()
                    }
                    TicTacToeTeam::O => tiny_skia::PathBuilder::from_circle(50.0, 50.0, 50.0),
                };
                let path = path
                    .with_context(|| format!("Failed to build path for team '{:?}'", team))?
                    .transform(transform)
                    .with_context(|| format!("Failed to transform path for team '{:?}'", team))?;

                pixmap
                    .stroke_path(
                        &path,
                        &paint,
                        &stroke,
                        tiny_skia::Transform::identity(),
                        None,
                    )
                    .with_context(|| format!("Failed to draw path for teamt '{:?}'", team))?;
            } else {
                paint.set_color_rgba8(255, 255, 255, 255);
                let path = self.number_paths[i].clone();
                let bounds = path.bounds();

                let ratio = SQUARE_SIZE as f32 / bounds.height().max(bounds.width());
                let path = path
                    .transform(transform.pre_scale(ratio, ratio))
                    .with_context(|| format!("Failed to transform path for digit '{}'", i))?;

                pixmap
                    .fill_path(
                        &path,
                        &paint,
                        Default::default(),
                        tiny_skia::Transform::identity(),
                        None,
                    )
                    .with_context(|| format!("Failed to draw path for digit '{}'", i))?;
            }
        }

        let draw_end = Instant::now();
        info!("Board draw time: {:?}", draw_end - draw_start);

        let encode_start = Instant::now();
        let img = pixmap
            .encode_png()
            .with_context(|| "failed to encode board")?;
        let encode_end = Instant::now();

        info!("Board png encode time: {:?}", encode_end - encode_start);

        Ok(img)
    }

    /// Render a Tic-Tac-Toe board on a threadpool
    pub(crate) async fn render_board_async(&self, state: u16) -> anyhow::Result<Vec<u8>> {
        // TODO: LRU cache
        let _permit = self.render_semaphore.acquire().await?;
        let self_clone = self.clone();
        tokio::task::spawn_blocking(move || self_clone.render_board(state)).await?
    }
}

/// Utility to draw a font glyph to a path.
#[derive(Debug)]
pub(crate) struct SkiaBuilder(tiny_skia::PathBuilder);

impl SkiaBuilder {
    /// Make a new [`SkiaBuilder`].
    pub(crate) fn new() -> Self {
        Self(Default::default())
    }

    /// Get the inner [`tiny_skia::Path`].
    pub(crate) fn into_path(self) -> Option<tiny_skia::Path> {
        let mut path = self.0.finish()?;

        // This transform is needed to make ttf's coordinate system agree with tiny-skia's
        let bounds = path.bounds();
        let transform = tiny_skia::Transform::from_scale(1.0, -1.0)
            .post_translate(-bounds.x(), bounds.y() + bounds.height());
        path = path.transform(transform)?;

        Some(path)
    }
}

impl Default for SkiaBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ttf_parser::OutlineBuilder for SkiaBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.move_to(x, y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.0.line_to(x, y);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.0.quad_to(x1, y1, x, y);
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        // TODO: This is not used, is it implemented correctly?
        self.0.cubic_to(x1, y1, x2, y2, x, y);
    }

    fn close(&mut self) {
        self.0.close();
    }
}

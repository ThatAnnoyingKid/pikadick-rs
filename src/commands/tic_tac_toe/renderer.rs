use anyhow::Context;
use std::{
    sync::Arc,
    time::Instant,
};
use tiny_skia::{
    Paint,
    Path,
    PathBuilder,
    Pixmap,
    Rect,
    Stroke,
    Transform,
};
use tokio::sync::Semaphore;
use tracing::info;
use ttf_parser::{
    Face,
    OutlineBuilder,
};

const FONT_BYTES: &[u8] =
    include_bytes!("../../../assets/Averia_Serif_Libre/AveriaSerifLibre-Light.ttf");

const RENDERED_SIZE: u16 = 300;
const SQUARE_SIZE: u16 = RENDERED_SIZE / 3;
const SQUARE_SIZE_USIZE: usize = SQUARE_SIZE as usize;
const SQUARE_SIZE_F32: f32 = SQUARE_SIZE as f32;
const HALF_SQUARE_SIZE_F32: f32 = SQUARE_SIZE_F32 / 2.0;

const MAX_PARALLEL_RENDER_LIMIT: usize = 4;

/// Render a Tic-Tac-Toe board
#[derive(Debug, Clone)]
pub(crate) struct Renderer {
    background_pixmap: Arc<Pixmap>,
    number_paths: Arc<Vec<Path>>,

    render_semaphore: Arc<Semaphore>,
}

#[allow(clippy::new_without_default)]
impl Renderer {
    /// Make a new [`Renderer`].
    pub(crate) fn new() -> anyhow::Result<Self> {
        let font_face = Face::from_slice(FONT_BYTES, 0).context("invalid font")?;

        let mut background_pixmap = Pixmap::new(RENDERED_SIZE.into(), RENDERED_SIZE.into())
            .context("failed to create background pixmap")?;

        let mut paint = Paint::default();
        for i in 0..3 {
            for j in 0..3 {
                let x = i * SQUARE_SIZE;
                let y = j * SQUARE_SIZE;
                let square = Rect::from_xywh(x as f32, y as f32, SQUARE_SIZE_F32, SQUARE_SIZE_F32)
                    .context("failed to make square")?;

                if (j * 3 + i) % 2 == 0 {
                    paint.set_color_rgba8(255, 0, 0, 255);
                } else {
                    paint.set_color_rgba8(119, 119, 119, 255);
                }

                background_pixmap
                    .fill_rect(square, &paint, Transform::identity(), None)
                    .context("failed to fill square")?;
            }
        }

        let mut number_paths = Vec::with_capacity(10);
        let mut paint = Paint::default();
        paint.set_color_rgba8(255, 255, 255, 255);
        for i in b'0'..=b'9' {
            let glyph_id = font_face
                .glyph_index(char::from(i))
                .with_context(|| format!("missing glyph for '{}'", char::from(i)))?;

            let mut builder = SkiaBuilder::new();
            let _bb = font_face
                .outline_glyph(glyph_id, &mut builder)
                .with_context(|| format!("missing glyph bounds for '{}'", char::from(i)))?;
            let path = builder.into_path().with_context(|| {
                format!("failed to generate glyph path for '{}'", char::from(i))
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
    // Author might add more fields
    #[allow(clippy::field_reassign_with_default)]
    pub(crate) fn render_board(&self, board: tic_tac_toe::Board) -> anyhow::Result<Vec<u8>> {
        let draw_start = Instant::now();
        let mut pixmap = self.background_pixmap.as_ref().as_ref().to_owned();

        const PIECE_WIDTH: u16 = 4;

        let mut paint = Paint::default();
        let mut stroke = Stroke::default();
        paint.anti_alias = true;
        stroke.width = f32::from(PIECE_WIDTH);

        for (i, team) in board.iter() {
            let transform = Transform::from_translate(
                ((u16::from(i) % 3) * SQUARE_SIZE) as f32,
                ((u16::from(i) / 3) * SQUARE_SIZE) as f32,
            );

            if let Some(team) = team {
                paint.set_color_rgba8(0, 0, 0, 255);
                let path = match team {
                    tic_tac_toe::Team::X => {
                        let mut path_builder = PathBuilder::new();
                        path_builder.move_to((PIECE_WIDTH / 2).into(), (PIECE_WIDTH / 2).into());
                        path_builder.line_to(
                            SQUARE_SIZE_F32 - f32::from(PIECE_WIDTH / 2),
                            SQUARE_SIZE_F32 - f32::from(PIECE_WIDTH / 2),
                        );
                        path_builder.move_to(
                            (PIECE_WIDTH / 2).into(),
                            SQUARE_SIZE_F32 - f32::from(PIECE_WIDTH / 2),
                        );
                        path_builder.line_to(
                            SQUARE_SIZE_F32 - f32::from(PIECE_WIDTH / 2),
                            f32::from(PIECE_WIDTH / 2),
                        );
                        path_builder.finish()
                    }
                    tic_tac_toe::Team::O => PathBuilder::from_circle(
                        HALF_SQUARE_SIZE_F32,
                        HALF_SQUARE_SIZE_F32,
                        HALF_SQUARE_SIZE_F32 - f32::from(PIECE_WIDTH / 2),
                    ),
                };
                let path =
                    path.with_context(|| format!("failed to build path for team '{:?}'", team))?;

                pixmap
                    .stroke_path(&path, &paint, &stroke, transform, None)
                    .with_context(|| format!("failed to draw path for team '{:?}'", team))?;
            } else {
                paint.set_color_rgba8(255, 255, 255, 255);
                let path = &self.number_paths[usize::from(i) + 1];
                let bounds = path.bounds();

                let ratio = ((SQUARE_SIZE / 2) as f32) / bounds.height().max(bounds.width());
                let transform = transform.pre_scale(ratio, ratio).post_translate(
                    (SQUARE_SIZE_F32 / 2.0) - (ratio * bounds.width() / 2.0),
                    (SQUARE_SIZE_F32 / 2.0) - (ratio * bounds.height() / 2.0),
                );

                pixmap
                    .fill_path(path, &paint, Default::default(), transform, None)
                    .with_context(|| format!("failed to draw path for digit '{}'", i))?;
            }
        }

        // Draw winning line
        if let Some(winner_info) = board.get_winner_info() {
            stroke.width = 10.0;
            paint.set_color_rgba8(48, 48, 48, 255);

            let start_index = winner_info.start_tile_index();
            let start = usize::from(start_index);
            let mut start_x = ((start % 3) * SQUARE_SIZE_USIZE + (SQUARE_SIZE_USIZE / 2)) as f32;
            let mut start_y = ((start / 3) * SQUARE_SIZE_USIZE + (SQUARE_SIZE_USIZE / 2)) as f32;

            let end_index = winner_info.end_tile_index();
            let end = usize::from(end_index);
            let mut end_x = ((end % 3) * SQUARE_SIZE_USIZE + (SQUARE_SIZE_USIZE / 2)) as f32;
            let mut end_y = ((end / 3) * SQUARE_SIZE_USIZE + (SQUARE_SIZE_USIZE / 2)) as f32;

            match winner_info.win_type {
                tic_tac_toe::WinType::Horizontal => {
                    start_x -= SQUARE_SIZE_F32 / 4.0;
                    end_x += SQUARE_SIZE_F32 / 4.0;
                }
                tic_tac_toe::WinType::Vertical => {
                    start_y -= SQUARE_SIZE_F32 / 4.0;
                    end_y += SQUARE_SIZE_F32 / 4.0;
                }
                tic_tac_toe::WinType::Diagonal => {
                    start_x -= SQUARE_SIZE_F32 / 4.0;
                    start_y -= SQUARE_SIZE_F32 / 4.0;
                    end_x += SQUARE_SIZE_F32 / 4.0;
                    end_y += SQUARE_SIZE_F32 / 4.0;
                }
                tic_tac_toe::WinType::AntiDiagonal => {
                    start_x += SQUARE_SIZE_F32 / 4.0;
                    start_y -= SQUARE_SIZE_F32 / 4.0;
                    end_x -= SQUARE_SIZE_F32 / 4.0;
                    end_y += SQUARE_SIZE_F32 / 4.0;
                }
            }

            let mut path_builder = PathBuilder::new();
            path_builder.move_to(start_x, start_y);
            path_builder.line_to(end_x, end_y);
            let path = path_builder
                .finish()
                .context("failed to draw winning line")?;

            pixmap
                .stroke_path(&path, &paint, &stroke, Transform::identity(), None)
                .context("failed to draw path for winning line")?;
        }

        let draw_end = Instant::now();
        info!("Board draw time: {:?}", draw_end - draw_start);

        let encode_start = Instant::now();
        let img = pixmap.encode_png().context("failed to encode board")?;
        let encode_end = Instant::now();

        info!("Board png encode time: {:?}", encode_end - encode_start);

        Ok(img)
    }

    /// Render a Tic-Tac-Toe board on a threadpool
    pub(crate) async fn render_board_async(
        &self,
        board: tic_tac_toe::Board,
    ) -> anyhow::Result<Vec<u8>> {
        // TODO: LRU cache
        let _permit = self.render_semaphore.acquire().await?;
        let self_clone = self.clone();
        tokio::task::spawn_blocking(move || self_clone.render_board(board)).await?
    }
}

/// Utility to draw a font glyph to a path.
#[derive(Debug)]
pub(crate) struct SkiaBuilder(PathBuilder);

impl SkiaBuilder {
    /// Make a new [`SkiaBuilder`].
    pub(crate) fn new() -> Self {
        Self(Default::default())
    }

    /// Get the inner [`tiny_skia::Path`].
    pub(crate) fn into_path(self) -> Option<Path> {
        let mut path = self.0.finish()?;

        // This transform is needed to make ttf's coordinate system agree with tiny-skia's
        let bounds = path.bounds();
        let transform = Transform::from_scale(1.0, -1.0)
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

impl OutlineBuilder for SkiaBuilder {
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

#[cfg(test)]
mod test {
    use super::*;
    use tic_tac_toe::Team;

    #[test]
    fn render_board() {
        let renderer = Renderer::new().expect("failed to make renderer");
        let board = tic_tac_toe::Board::new()
            .set(0, Some(Team::X))
            .set(4, Some(Team::X))
            .set(8, Some(Team::X));
        let img = renderer.render_board(board).expect("failed to render");
        std::fs::write("ttt-render-test.png", img).expect("failed to save");
    }
}

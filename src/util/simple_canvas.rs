use skrifa::{
    FontRef,
    MetadataProvider,
    metrics::BoundingBox,
    prelude::{
        LocationRef,
        Size as SkrifaSize,
    },
};
use std::sync::Arc;
use vello_cpu::{
    Glyph,
    Image,
    ImageSource,
    Pixmap,
    RenderContext,
    Resources,
    peniko::{
        Blob,
        FontData as VelloFontData,
        ImageSampler,
        kurbo::{
            Affine,
            BezPath,
            Rect,
        },
    },
};

#[derive(Debug, Copy, Clone)]
pub struct FontId(usize);

pub struct FontData {
    pub font_data: VelloFontData,
    pub font_ref: FontRef<'static>,
}

impl std::fmt::Debug for FontData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FontData")
            .field("font_data", &self.font_data)
            .finish()
    }
}

#[derive(Debug)]
pub struct SimpleCanvas {
    pub context: RenderContext,
    pub resources: Resources,
    pub fonts: Vec<FontData>,
}

impl SimpleCanvas {
    /// Make a new `SimpleCanvas`.
    pub fn new(width: u16, height: u16) -> Self {
        let context = RenderContext::new(width, height);
        let resources = Resources::new();

        Self {
            context,
            resources,
            fonts: Vec::new(),
        }
    }

    /// Add a new font.
    pub fn add_font(&mut self, font_ttf: &'static [u8]) -> anyhow::Result<FontId> {
        let font = Arc::new(font_ttf);
        let font_blob = Blob::new(font);
        let font_data = VelloFontData {
            data: font_blob,
            index: 0,
        };

        let font_ref = FontRef::new(font_ttf)?;

        let index = self.fonts.len();
        self.fonts.push(FontData {
            font_data,
            font_ref,
        });

        Ok(FontId(index))
    }

    /// Get font data.
    ///
    /// # Panics
    /// Panics if the font does not exist.
    pub fn get_font_data(&self, id: FontId) -> &FontData {
        self.fonts.get(id.0).expect("Missing font")
    }

    /// Get the line height for a font.
    pub fn get_line_height(&self, id: FontId, font_size: f32) -> f32 {
        let font_data = self.get_font_data(id);
        let metrics = font_data
            .font_ref
            .metrics(SkrifaSize::new(font_size), LocationRef::default());
        metrics.ascent - metrics.descent + metrics.leading
    }

    /// Get the bounding box for some text.
    #[expect(dead_code)]
    pub fn get_text_bounding_box(
        &self,
        font_id: FontId,
        text: &str,
        font_size: f32,
    ) -> BoundingBox {
        let font = self.get_font_data(font_id);

        let charmap = font.font_ref.charmap();
        let glyph_metrics = font
            .font_ref
            .glyph_metrics(SkrifaSize::new(font_size), LocationRef::default());

        let mut current_x = 0.0;
        let mut x_min: f32 = 0.0;
        let mut x_max: f32 = 0.0;
        let mut y_min: f32 = 0.0;
        let mut y_max: f32 = 0.0;
        for ch in text.chars() {
            if let Some(glyph_id) = charmap.map(ch) {
                let bounds = glyph_metrics.bounds(glyph_id);
                if let Some(bounds) = bounds {
                    x_min = x_min.min(bounds.x_min + current_x);
                    x_max = x_max.max(bounds.x_max + current_x);
                    y_min = y_min.min(bounds.y_min);
                    y_max = y_max.max(bounds.y_max);
                }

                let advance = glyph_metrics.advance_width(glyph_id).unwrap_or(0.0);
                current_x += advance;
            }
        }

        BoundingBox {
            x_min,
            x_max,
            y_min,
            y_max,
        }
    }

    /// Reset the context.
    pub fn reset(&mut self) {
        self.context.reset();
    }

    /// Draw an image at x, y.
    pub fn draw_image(
        &mut self,
        image: Pixmap,
        x: f64,
        y: f64,
        target_width: f64,
        target_height: f64,
        clip_path: Option<&BezPath>,
    ) {
        let image_width = f64::from(image.width());
        let image_height = f64::from(image.height());

        let icon_image = Image {
            image: ImageSource::Pixmap(Arc::new(image)),
            sampler: ImageSampler::default(),
        };

        let old_transform = *self.context.transform();

        let transform = Affine::translate((x, y))
            * Affine::scale_non_uniform(target_width / image_width, target_height / image_height);
        self.context.set_transform(transform);
        if let Some(clip_path) = clip_path {
            self.context.push_clip_path(clip_path);
        }
        self.context.set_paint(icon_image);
        self.context
            .fill_rect(&Rect::new(0.0, 0.0, image_width, image_height));
        if clip_path.is_some() {
            self.context.pop_clip();
        }
        self.context.set_transform(old_transform);
    }

    /// Fill in text.
    pub fn fill_text(
        &mut self,
        font_id: FontId,
        text: &str,
        x: f32,
        y: f32,
        font_size: f32,
    ) -> anyhow::Result<()> {
        let font = self.get_font_data(font_id);
        let font_data = font.font_data.clone();

        let charmap = font.font_ref.charmap();
        let glyph_metrics = font
            .font_ref
            .glyph_metrics(SkrifaSize::new(font_size), LocationRef::default());

        let mut current_x = x;
        let mut mapped_glyphs = Vec::new();

        for ch in text.chars() {
            if let Some(glyph_id) = charmap.map(ch) {
                mapped_glyphs.push(Glyph {
                    id: glyph_id.to_u32(),
                    x: current_x,
                    y,
                });

                let advance = glyph_metrics.advance_width(glyph_id).unwrap_or(0.0);
                current_x += advance;
            }
        }

        self.context
            .glyph_run(&mut self.resources, &font_data)
            .font_size(font_size)
            .hint(true)
            .fill_glyphs(mapped_glyphs.into_iter())?;

        Ok(())
    }

    /// Render an image.
    pub fn render(&mut self) -> Pixmap {
        let mut target = Pixmap::new(self.context.width(), self.context.height());

        self.context.flush();
        self.context.render(&mut target, &mut self.resources);

        target
    }
}

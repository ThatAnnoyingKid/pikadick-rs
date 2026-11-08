use std::sync::Arc;
use vello_cpu::{
    Image,
    ImageSource,
    Pixmap,
    RenderContext,
    Resources,
    peniko::{
        ImageSampler,
        kurbo::{
            Affine,
            BezPath,
            Rect,
        },
    },
};

#[derive(Debug)]
pub struct SimpleCanvas {
    pub context: RenderContext,
    pub resources: Resources,
}

impl SimpleCanvas {
    /// Make a new `SimpleCanvas`.
    pub fn new(width: u16, height: u16) -> Self {
        let context = RenderContext::new(width, height);
        let resources = Resources::new();

        Self { context, resources }
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
        self.context.fill_rect(&Rect::new(
            0.0,
            0.0,
            image_width,
            image_height,
        ));
        if clip_path.is_some() {
            self.context.pop_clip();
        }
        self.context.set_transform(old_transform);
    }

    /// Render an image.
    pub fn render(&mut self) -> Pixmap {
        let mut target = Pixmap::new(self.context.width(), self.context.height());

        self.context.flush();
        self.context.render(&mut target, &mut self.resources);

        target
    }
}

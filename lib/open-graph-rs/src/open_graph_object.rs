use scraper::{
    Html,
    Selector,
};
use std::path::Path;
use url::Url;

/// An error that may occur while parsing an [`OpenGraphObject`].
#[derive(Debug, thiserror::Error)]
pub enum FromHtmlError {
    /// Missing title field
    #[error("missing title")]
    MissingTitle,

    /// Missing Type field
    #[error("missing type")]
    MissingType,

    /// Missing Image field
    #[error("missing image")]
    MissingImage,

    /// Invalid Image field
    #[error("invalid image: {0}")]
    InvalidImage(url::ParseError),

    /// Missing Url field
    #[error("missing url")]
    MissingUrl,

    /// Invalid Image field
    #[error("invalid url: {0}")]
    InvalidUrl(url::ParseError),

    /// Invalid Audio Url
    #[error("invalid audio url: {0}")]
    InvalidAudioUrl(url::ParseError),

    /// Invalid Video Url
    #[error("invalid video url: {0}")]
    InvalidVideoUrl(url::ParseError),

    /// Ran into unimplemented functionality
    #[error("unimplemented: '{0}'")]
    Unimplemented(String),
}

/// An OpenGraphObject.
///
/// See <https://ogp.me/>
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct OpenGraphObject {
    /// Object Title
    pub title: String,

    /// Object Type/Kind
    pub kind: String,

    /// Object Image Url
    pub image: Url,

    /// Object Permanent Url
    pub url: Url,

    /// Audio Url
    pub audio_url: Option<Url>,

    /// Object Description
    pub description: Option<String>,

    /// Video Url
    pub video_url: Option<Url>,
}

impl OpenGraphObject {
    /// Make a new [`OpenGraphObject`] from [`Html`].
    pub fn from_html(html: &Html) -> Result<Self, FromHtmlError> {
        let title = lookup_meta_kv(html, "og:title")
            .ok_or(FromHtmlError::MissingTitle)?
            .to_string();

        let kind = lookup_meta_kv(html, "og:type")
            .ok_or(FromHtmlError::MissingType)?
            .to_string();

        match kind.as_str() {
            "instapp:photo" => {
                // Not in spec, but Instagram uses it.
                // TODO: Fill fields though testing
            }
            "video.movie" | "video.tv_show" | "video.other" => {
                //let video_actors = lookup_meta_kv(html, "video:actor")
                // video:actor:role
                // video:director
                // video:writer
                // video:duration
                // video:release_date
                // video:tag
            }
            "video.episode" => {
                return Err(FromHtmlError::Unimplemented("video.episode".into()));
            }
            "video" => {
                // Not in spec, but Instagram uses it.
                // TODO: Fill fields though testing
            }
            _unknown => {
                // return Err(FromHtmlError::Unimplemented(format!("kind: {}", unknown)));
                // Its better to not error out here and get as many fields as possible
            }
        }

        let image = lookup_meta_kv(html, "og:image")
            .map(Url::parse)
            .ok_or(FromHtmlError::MissingImage)?
            .map_err(FromHtmlError::InvalidImage)?;

        let url = lookup_meta_kv(html, "og:url")
            .map(Url::parse)
            .ok_or(FromHtmlError::MissingUrl)?
            .map_err(FromHtmlError::InvalidUrl)?;

        let audio_url = lookup_meta_kv(html, "og:audio")
            .map(|s| Url::parse(s).map_err(FromHtmlError::InvalidAudioUrl))
            .transpose()?;

        let description = lookup_meta_kv(html, "og:description").map(ToString::to_string);

        let video_url = lookup_meta_kv(html, "og:video")
            .map(|s| Url::parse(s).map_err(FromHtmlError::InvalidVideoUrl))
            .transpose()?;

        Ok(Self {
            title,
            kind,
            image,
            url,

            audio_url,
            description,
            video_url,
        })
    }

    /// Check whether this is a video
    pub fn is_video(&self) -> bool {
        self.kind.split('.').next() == Some("video")
    }

    /// Check whether this is an image
    pub fn is_image(&self) -> bool {
        // instapp:photo is weird in the sense that it might be a slideshow.
        // However, there isn't OGP data provided for anything but the first slide.
        // The best we can do is let users at least download the first slide.
        self.kind == "instapp:photo"
    }

    /// Try to get the video url's file name
    pub fn get_image_file_name(&self) -> Option<&str> {
        Path::new(self.image.path()).file_name()?.to_str()
    }

    /// Try to get the video url's file name
    pub fn get_video_url_file_name(&self) -> Option<&str> {
        Path::new(self.video_url.as_ref()?.path())
            .file_name()?
            .to_str()
    }
}

impl std::str::FromStr for OpenGraphObject {
    type Err = FromHtmlError;

    fn from_str(data: &str) -> Result<Self, Self::Err> {
        OpenGraphObject::from_html(&Html::parse_document(data))
    }
}

/// Lookup the value for a `<meta property = {name} content = {value} />`
fn lookup_meta_kv<'a>(html: &'a Html, name: &str) -> Option<&'a str> {
    let selector = Selector::parse(&format!("meta[property=\"{}\"]", name)).ok()?;
    html.select(&selector).next()?.value().attr("content")
}

#[cfg(test)]
mod test {
    use super::*;

    const VIDEO_OBJ: &str = include_str!("../test_data/insta_video.html");

    #[test]
    fn parse_video_obj() {
        let obj: OpenGraphObject = VIDEO_OBJ.parse().expect("invalid open graph object");
        dbg!(&obj);
    }
}

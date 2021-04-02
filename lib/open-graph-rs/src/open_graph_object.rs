use scraper::{
    Html,
    Selector,
};
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
    #[error("unimplemented")]
    Unimplemented,
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
    /// Make a new [`OpenGraphObject`] from a [`Html`].
    pub fn from_html(html: &Html) -> Result<Self, FromHtmlError> {
        let title = lookup_meta_kv(html, "og:title")
            .ok_or(FromHtmlError::MissingTitle)?
            .to_string();

        let kind = lookup_meta_kv(html, "og:type")
            .ok_or(FromHtmlError::MissingType)?
            .to_string();

        match kind.as_str() {
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
                return Err(FromHtmlError::Unimplemented);
            }
            "video" => {
                // Not in spec, but Instagram uses it.
                // TODO: Fill fields though testing
            }
            _ => {
                return Err(FromHtmlError::Unimplemented);
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
            video_url,
            description,
        })
    }

    /// Check whether this is a video
    pub fn is_video(&self) -> bool {
        self.kind.split('.').next() == Some("video")
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
        let html = Html::parse_document(VIDEO_OBJ);
        let obj = OpenGraphObject::from_html(&html).expect("invalid open graph object");
        dbg!(&obj);
    }
}

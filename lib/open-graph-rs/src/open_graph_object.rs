use select::{
    document::Document,
    predicate::{
        And,
        Attr,
        Name,
    },
};
use url::Url;

/// An error that may occur while parsing an [`OpenGraphObject`].
///
#[derive(Debug, thiserror::Error)]
pub enum FromDocError {
    /// Missing title field
    ///
    #[error("missing title")]
    MissingTitle,

    /// Missing Type field
    ///
    #[error("missing type")]
    MissingType,

    /// Missing Image field
    ///
    #[error("missing image")]
    MissingImage,

    /// Invalid Image field
    ///
    #[error("invalid image: {0}")]
    InvalidImage(url::ParseError),

    /// Missing Url field
    ///
    #[error("missing url")]
    MissingUrl,

    /// Invalid Image field
    ///
    #[error("invalid url: {0}")]
    InvalidUrl(url::ParseError),

    /// Invalid Audio Url
    ///
    #[error("invalid audio url: {0}")]
    InvalidAudioUrl(url::ParseError),

    /// Invalid Video Url
    ///
    #[error("invalid video url: {0}")]
    InvalidVideoUrl(url::ParseError),

    /// Ran into unimplemented functionality
    ///
    #[error("unimplemented")]
    Unimplemented,
}

/// An OpenGraphObject.
///
/// See https://ogp.me/.
///
#[derive(Debug)]
pub struct OpenGraphObject {
    /// Object Title
    ///
    pub title: String,

    /// Object Type/Kind
    ///
    pub kind: String,

    /// Object Image Url
    ///
    pub image: Url,

    /// Object Permanent Url
    ///
    pub url: Url,

    /// Audio Url
    ///
    pub audio_url: Option<Url>,

    /// Object Description
    ///
    pub description: Option<String>,

    /// Video Url
    ///
    pub video_url: Option<Url>,
}

impl OpenGraphObject {
    /// Make a new [`OpenGraphObject`] from a [`Document`].
    ///
    pub fn from_doc(doc: &Document) -> Result<Self, FromDocError> {
        let title = lookup_meta_kv(doc, "og:title")
            .ok_or(FromDocError::MissingTitle)?
            .to_string();
        let kind = lookup_meta_kv(doc, "og:type")
            .ok_or(FromDocError::MissingType)?
            .to_string();

        match kind.as_str() {
            "video.movie" | "video.tv_show" | "video.other" => {
                //let video_actors = lookup_meta_kv(doc, "video:actor")
                // video:actor:role
                // video:director
                // video:writer
                // video:duration
                // video:release_date
                // video:tag
            }
            "video.episode" => {
                return Err(FromDocError::Unimplemented);
            }
            _ => {
                return Err(FromDocError::Unimplemented);
            }
        }

        let image = lookup_meta_kv(doc, "og:image")
            .map(Url::parse)
            .ok_or(FromDocError::MissingImage)?
            .map_err(FromDocError::InvalidImage)?;

        let url = lookup_meta_kv(doc, "og:url")
            .map(Url::parse)
            .ok_or(FromDocError::MissingUrl)?
            .map_err(FromDocError::InvalidUrl)?;

        let audio_url = lookup_meta_kv(doc, "og:audio")
            .map(|s| Url::parse(s).map_err(FromDocError::InvalidAudioUrl))
            .transpose()?;

        let description = lookup_meta_kv(doc, "og:description").map(ToString::to_string);

        let video_url = lookup_meta_kv(doc, "og:video")
            .map(|s| Url::parse(s).map_err(FromDocError::InvalidVideoUrl))
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
}

/// Lookup the value for a `<meta property = {name} content = {value} />`
///
fn lookup_meta_kv<'a>(doc: &'a Document, name: &str) -> Option<&'a str> {
    doc.find(And(Name("meta"), Attr("property", name)))
        .next()?
        .attr("content")
}

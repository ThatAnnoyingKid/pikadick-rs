use select::{
    document::Document,
    predicate::{
        Attr,
        Child,
        Name,
        Text,
    },
};
use url::Url;

/// A Post page
#[derive(Debug)]
pub struct Post {
    /// Not included for videos/gifs
    ///
    pub thumb_url: Option<Url>,

    /// Image URL
    ///
    pub image_url: Url,
}

impl Post {
    /// Try to make a [`Post`] from a [`Document`].
    ///
    pub fn from_doc(doc: &Document) -> Result<Self, FromDocError> {
        let options_header = doc
            .find(Child(Name("div"), Name("h5")))
            .find(|el| el.find(Text).last().and_then(|el| el.as_text()) == Some("Options"))
            .and_then(|el| el.parent())
            .ok_or(FromDocError::MissingOptionsSection)?;

        let thumb_url = doc
            .find(Attr("id", "image"))
            .last()
            .and_then(|el| el.attr("src"))
            .map(Url::parse)
            .transpose()
            .map_err(FromDocError::InvalidThumbUrl)?;

        let image_url = options_header
            .find(Name("li"))
            .find_map(|el| {
                let a = el.find(Name("a")).last()?;
                let a_text = el
                    .find(Text)
                    .filter_map(|el| Some(el.as_text()?.trim()))
                    .filter(|el| !el.is_empty())
                    .next()?;
                if a_text == "Original image" {
                    Url::parse(a.attr("href")?).ok()
                } else {
                    None
                }
            })
            .ok_or(FromDocError::MissingImageUrl)?;

        Ok(Post {
            thumb_url,
            image_url,
        })
    }

    /// Try to get the image name.
    ///
    pub fn get_image_name(&self) -> Option<&str> {
        self.image_url.path_segments()?.last()
    }
}

/// Error that may occur while parsing a [`Post`] from a [`Document`].
///
#[derive(Debug, thiserror::Error)]
pub enum FromDocError {
    /// Invalid thumbnail url
    ///
    #[error("invalid thumbnail url: {0}")]
    InvalidThumbUrl(url::ParseError),

    #[error("missing options section")]
    MissingOptionsSection,

    /// Missing Image Url
    ///
    #[error("missing image url")]
    MissingImageUrl,
}

#[cfg(test)]
mod test {
    use super::*;

    const GIF_DOC_STR: &str = include_str!("../../test_data/gif_post.html");

    #[test]
    fn from_doc_gif() {
        let doc = Document::from(GIF_DOC_STR);
        Post::from_doc(&doc).unwrap();
    }
}

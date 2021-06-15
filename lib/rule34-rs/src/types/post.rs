use scraper::{
    ElementRef,
    Html,
    Selector,
};
use url::Url;

/// Error that may occur while parsing a [`Post`] from [`Html`].
#[derive(Debug, thiserror::Error)]
pub enum FromHtmlError {
    /// Invalid thumbnail url
    #[error("invalid thumbnail url")]
    InvalidThumbUrl(url::ParseError),

    /// Missing the options section
    #[error("missing options section")]
    MissingOptionsSection,

    /// Missing Image Url
    #[error("missing image url")]
    MissingImageUrl,

    /// invalid image url
    #[error("invalid image url")]
    InvalidImageUrl(url::ParseError),
}

/// A Post page
#[derive(Debug)]
pub struct Post {
    /// Thumbnail Url
    ///
    /// Not included for videos/gifs.
    pub thumb_url: Option<Url>,

    /// Image URL
    pub image_url: Url,
}

impl Post {
    /// Try to make a [`Post`] from [`Html`].
    pub fn from_html(html: &Html) -> Result<Self, FromHtmlError> {
        let options_header_selector =
            Selector::parse("div > h5").expect("invalid options header selector");
        let options_header = html
            .select(&options_header_selector)
            .find_map(|element| {
                if element.text().next()? != "Options" {
                    return None;
                }

                ElementRef::wrap(element.parent()?)
            })
            .ok_or(FromHtmlError::MissingOptionsSection)?;

        let thumb_url_selector = Selector::parse("#image").expect("invalid thumb_url selector");
        let thumb_url = html
            .select(&thumb_url_selector)
            .last()
            .and_then(|element| element.value().attr("src"))
            .map(Url::parse)
            .transpose()
            .map_err(FromHtmlError::InvalidThumbUrl)?;

        let li_selector = Selector::parse("li").expect("invalid li selector");
        let a_selector = Selector::parse("a[href]").expect("invalid a selector");
        let image_url = options_header
            .select(&li_selector)
            .find_map(|element| {
                let a = element.select(&a_selector).last()?;
                let a_text = a.text().next()?.trim();

                if a_text != "Original image" {
                    return None;
                }

                let url = a.value().attr("href")?;
                Some(Url::parse(url).map_err(FromHtmlError::InvalidImageUrl))
            })
            .ok_or(FromHtmlError::MissingImageUrl)??;

        Ok(Post {
            thumb_url,
            image_url,
        })
    }

    /// Try to get the image name.
    pub fn get_image_name(&self) -> Option<&str> {
        self.image_url.path_segments()?.last()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const GIF_HTML_STR: &str = include_str!("../../test_data/gif_post.html");

    #[test]
    fn from_gif_html() {
        let html = Html::parse_document(GIF_HTML_STR);
        Post::from_html(&html).expect("invalid gif post");
    }
}

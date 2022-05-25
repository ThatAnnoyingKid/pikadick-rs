use scraper::{
    ElementRef,
    Html,
    Selector,
};
use url::Url;

/// Error that may occur while parsing a [`Post`] from [`Html`].
#[derive(Debug, thiserror::Error)]
pub enum FromHtmlError {
    /// Missing the stats section
    #[error("missing stats section")]
    MissingStatsSection,

    /// Missing the post id
    #[error("missing post id")]
    MissingPostId,

    ///The post id is invalid
    #[error("invalid post id")]
    InvalidPostId(#[source] std::num::ParseIntError),

    /// Missing the post date
    #[error("missing post date")]
    MissingPostDate,

    /// Invalid Post source
    #[error("invalid post source")]
    InvalidPostSource(#[source] url::ParseError),

    /// Invalid thumbnail url
    #[error("invalid thumbnail url")]
    InvalidThumbUrl(#[source] url::ParseError),

    /// Missing the options section
    #[error("missing options section")]
    MissingOptionsSection,

    /// Missing Image Url
    #[error("missing image url")]
    MissingImageUrl,

    /// invalid image url
    #[error("invalid image url")]
    InvalidImageUrl(#[source] url::ParseError),

    /// Missing the sidebar
    #[error("missing sidebar")]
    MissingSidebar,

    /// Missing the sidebar title
    #[error("missing sidebar title")]
    MissingSidebarTitle,

    /// The sidebar title is invalid
    #[error("invalid sidebar title")]
    InvalidSidebarTitle(#[source] SidebarTitleFromStrError),

    /// A tag was expected but not found
    #[error("missing tag")]
    MissingTag,

    /// A parent post url was invalid
    #[error("invalid parent post")]
    InvalidParentPost(#[source] std::num::ParseIntError),
}

/// A Post page
#[derive(Debug)]
pub struct HtmlPost {
    /// The post id
    pub id: u64,

    /// The post date
    pub date: String,

    /// The post source
    pub source: Option<Url>,

    /// Thumbnail Url
    ///
    /// Not included for videos/gifs.
    pub thumb_url: Option<Url>,

    /// Image URL
    pub image_url: Url,

    /// Copyright tags
    pub copyright_tags: Vec<String>,

    /// Character tags
    pub character_tags: Vec<String>,

    /// Artist tags
    pub artist_tags: Vec<String>,

    /// General tags
    pub general_tags: Vec<String>,

    /// Meta tags
    pub meta_tags: Vec<String>,

    /// Whether this post has child posts
    pub has_child_posts: bool,

    /// Whether this post has a parent post
    pub parent_post: Option<u64>,
}

impl HtmlPost {
    /// Try to make a [`Post`] from [`Html`].
    pub fn from_html(html: &Html) -> Result<Self, FromHtmlError> {
        lazy_static::lazy_static! {
            static ref STATS_SELECTOR: Selector = Selector::parse("#stats").expect("invalid stats selector");
            static ref LI_SELECTOR: Selector = Selector::parse("li").expect("invalid li selector");

            static ref OPTIONS_HEADER_SELECTOR: Selector = Selector::parse("div > h5").expect("invalid options header selector");

            static ref THUMB_URL_SELECTOR: Selector = Selector::parse("#image").expect("invalid thumb_url selector");

            static ref A_SELECTOR: Selector = Selector::parse("a[href]").expect("invalid a selector");

            static ref SIDEBAR_SELECTOR: Selector = Selector::parse("#tag-sidebar").expect("invalid sidebar selector");

            static ref STATUS_NOTICE_SELECTOR: Selector = Selector::parse(".status-notice").expect("invalid status notice selector");
        }

        let mut id_str = None;
        let mut date = None;
        let mut source_str = None;

        let stats_header_element_iter = html
            .select(&STATS_SELECTOR)
            .next()
            .ok_or(FromHtmlError::MissingStatsSection)?
            .select(&LI_SELECTOR);
        for element in stats_header_element_iter {
            if let Some(text) = element.text().next() {
                let text = text.trim();

                if id_str.is_none() && text.starts_with("Id: ") {
                    id_str = Some(text.trim_start_matches("Id: "));
                }

                if date.is_none() && text.starts_with("Posted: ") {
                    date = Some(text.trim_start_matches("Posted: "));
                }

                if source_str.is_none() && text.starts_with("Source:") {
                    source_str = element
                        .select(&A_SELECTOR)
                        .next()
                        .and_then(|a| a.value().attr("href"));
                }
            }
        }

        let id = id_str
            .ok_or(FromHtmlError::MissingPostId)?
            .parse()
            .map_err(FromHtmlError::InvalidPostId)?;
        let date = date.ok_or(FromHtmlError::MissingPostDate)?.to_string();
        let source = source_str
            .map(|source| source.parse())
            .transpose()
            .map_err(FromHtmlError::InvalidPostSource)?;

        let options_header = html
            .select(&OPTIONS_HEADER_SELECTOR)
            .find_map(|element| {
                let text = element.text().next()?;

                if text != "Options" {
                    return None;
                }

                let parent = ElementRef::wrap(element.parent()?)?;

                Some(parent)
            })
            .ok_or(FromHtmlError::MissingOptionsSection)?;

        let thumb_url = html
            .select(&THUMB_URL_SELECTOR)
            .last()
            .and_then(|element| element.value().attr("src"))
            .map(Url::parse)
            .transpose()
            .map_err(FromHtmlError::InvalidThumbUrl)?;

        let image_url = options_header
            .select(&LI_SELECTOR)
            .find_map(|element| {
                let a = element.select(&A_SELECTOR).last()?;
                let a_text = a.text().next()?.trim();

                if a_text != "Original image" {
                    return None;
                }

                let url = a.value().attr("href")?;
                Some(Url::parse(url).map_err(FromHtmlError::InvalidImageUrl))
            })
            .ok_or(FromHtmlError::MissingImageUrl)??;

        let sidebar = html
            .select(&SIDEBAR_SELECTOR)
            .next()
            .ok_or(FromHtmlError::MissingSidebar)?;
        let mut sidebar_title = None;
        let mut copyright_tags = Vec::new();
        let mut character_tags = Vec::new();
        let mut artist_tags = Vec::new();
        let mut general_tags = Vec::new();
        let mut meta_tags = Vec::new();
        for element in sidebar.select(&LI_SELECTOR) {
            // Titles have no classes
            let is_title = element.value().classes().count() == 0;

            if is_title {
                sidebar_title = Some(
                    element
                        .text()
                        .next()
                        .ok_or(FromHtmlError::MissingSidebarTitle)?
                        .parse::<SidebarTitle>()
                        .map_err(FromHtmlError::InvalidSidebarTitle)?,
                );
            } else if let Some(sidebar_title) = sidebar_title {
                let tag = element
                    .select(&A_SELECTOR)
                    .next()
                    .and_then(|el| el.text().next())
                    .ok_or(FromHtmlError::MissingTag)?;
                match sidebar_title {
                    SidebarTitle::Copyright => copyright_tags.push(tag.to_string()),
                    SidebarTitle::Character => character_tags.push(tag.to_string()),
                    SidebarTitle::Artist => artist_tags.push(tag.to_string()),
                    SidebarTitle::General => general_tags.push(tag.to_string()),
                    SidebarTitle::Meta => meta_tags.push(tag.to_string()),
                }
            }
        }

        let mut has_child_posts = false;
        let mut parent_post = None;

        for element in html.select(&STATUS_NOTICE_SELECTOR) {
            for text in element.text().map(|text| text.trim()) {
                match text {
                    "child posts" => {
                        has_child_posts = true;
                    }
                    "parent post" => {
                        if parent_post.is_none() {
                            parent_post = element
                                .select(&A_SELECTOR)
                                .next()
                                .and_then(|element| {
                                    let url = element.value().attr("href")?;

                                    let mut trimmed = false;
                                    let query = url.trim_start_matches(|c| {
                                        if !trimmed && c == '?' {
                                            trimmed = true;
                                            trimmed
                                        } else {
                                            !trimmed
                                        }
                                    });

                                    url::form_urlencoded::parse(query.as_bytes()).find_map(
                                        |(k, v)| {
                                            if k == "id" {
                                                Some(
                                                    v.parse::<u64>()
                                                        .map_err(FromHtmlError::InvalidParentPost),
                                                )
                                            } else {
                                                None
                                            }
                                        },
                                    )
                                })
                                .transpose()?;
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(Self {
            id,
            date,
            source,
            thumb_url,
            image_url,
            copyright_tags,
            character_tags,
            artist_tags,
            general_tags,
            meta_tags,
            has_child_posts,
            parent_post,
        })
    }

    /// Try to get the image name.
    pub fn get_image_name(&self) -> Option<&str> {
        self.image_url.path_segments()?.last()
    }

    /// Get the post url for this post.
    ///
    /// This allocates, so cache the result.
    pub fn get_html_post_url(&self) -> Url {
        crate::post_id_to_html_post_url(self.id)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SidebarTitleFromStrError {
    #[error("invalid title '{0}'")]
    InvalidTitle(String),
}

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
enum SidebarTitle {
    Copyright,
    Character,
    Artist,
    General,
    Meta,
}

impl std::str::FromStr for SidebarTitle {
    type Err = SidebarTitleFromStrError;

    fn from_str(data: &str) -> Result<Self, Self::Err> {
        match data {
            "Copyright" => Ok(Self::Copyright),
            "Character" => Ok(Self::Character),
            "Artist" => Ok(Self::Artist),
            "General" => Ok(Self::General),
            "Meta" => Ok(Self::Meta),
            _ => Err(SidebarTitleFromStrError::InvalidTitle(data.to_string())),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const GIF_HTML_STR: &str = include_str!("../../test_data/gif_post.html");

    #[test]
    fn from_gif_html() {
        let html = Html::parse_document(GIF_HTML_STR);
        HtmlPost::from_html(&html).expect("invalid gif post");
    }
}

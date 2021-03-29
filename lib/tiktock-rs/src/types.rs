use url::Url;

/// A url for tiktok post page
///
#[derive(Debug)]
pub struct PostUrl(Url);

impl PostUrl {
    /// Make a [`PostUrl`] from a [`Url`] or return the original url.
    ///
    pub fn from_url(url: Url) -> Result<Self, Url> {
        if url.host_str() != Some("www.tiktok.com") && url.host_str() != Some("tiktok.com") {
            return Err(url);
        }

        let mut path = match url.path_segments() {
            Some(path) => path,
            None => {
                return Err(url);
            }
        };

        let _user = match path.next() {
            Some(user) => user,
            None => {
                return Err(url);
            }
        };

        if path.next().map_or(true, |p| p != "video") {
            return Err(url);
        }

        let _id = match path.next() {
            Some(id) => id,
            None => {
                return Err(url);
            }
        };

        if path.next().map_or(false, |p| !p.is_empty()) {
            return Err(url);
        }

        Ok(Self(url))
    }

    /// Get this as a [`str`].
    ///
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

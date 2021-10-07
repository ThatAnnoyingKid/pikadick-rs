use crate::Error;
use bytes::Bytes;
use cookie_store::CookieStore;
use reqwest::header::HeaderValue;
use std::{
    fmt::Write,
    sync::RwLock,
};
use url::Url;

/// A Cookie Jar
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct CookieJar(RwLock<cookie_store::CookieStore>);

impl CookieJar {
    /// Make a new Cookie Jar.
    pub fn new() -> Self {
        Self(RwLock::new(Default::default()))
    }

    /// Clean the jar of expired cookies
    pub fn clean(&self) {
        let mut cookie_store = self.0.write().expect("cookie jar poisoned");

        let to_remove: Vec<_> = cookie_store
            .iter_any()
            .filter(|cookie| cookie.is_expired())
            .map(|cookie| {
                let domain = cookie
                    .domain()
                    .map(ToString::to_string)
                    .unwrap_or_else(String::new);
                let name = cookie.name().to_string();

                let path = cookie
                    .path()
                    .map(ToString::to_string)
                    .unwrap_or_else(String::new);

                (domain, name, path)
            })
            .collect();

        for (domain, name, path) in to_remove {
            cookie_store.remove(&domain, &name, &path);
        }
    }

    /// Save the cookie jar as json
    pub fn save_json<W>(&self, mut writer: W) -> Result<(), Error>
    where
        W: std::io::Write,
    {
        let cookie_store = self.0.read().expect("cookie jar poisoned");
        cookie_store
            .save_json(&mut writer)
            .map_err(Error::CookieStore)?;
        Ok(())
    }

    /// Load cookies from a json cookie file
    pub fn load_json<R>(&self, mut reader: R) -> Result<(), Error>
    where
        R: std::io::BufRead,
    {
        let mut cookie_store = self.0.write().expect("cookie jar poisoned");
        *cookie_store = CookieStore::load_json(&mut reader).map_err(Error::CookieStore)?;
        Ok(())
    }
}

impl reqwest::cookie::CookieStore for CookieJar {
    fn set_cookies(&self, headers: &mut dyn Iterator<Item = &HeaderValue>, url: &Url) {
        use cookie::Cookie;

        let iter = headers.filter_map(|val| {
            let val = val.to_str().ok()?;
            let cookie = Cookie::parse(val).ok()?;
            Some(cookie.into_owned())
        });

        self.0
            .write()
            .expect("cookie jar poisoned")
            .store_response_cookies(iter, url);
    }

    fn cookies(&self, url: &Url) -> Option<HeaderValue> {
        let mut val = String::new();
        let cookie_jar = self.0.read().expect("cookie jar poisoned");

        for cookie in cookie_jar.get_request_cookies(url) {
            let name = cookie.name();
            let value = cookie.value();

            val.reserve(name.len() + value.len() + 1 + 1);
            write!(&mut val, "{}={}; ", name, value).ok()?;
        }
        val.pop(); // Remove ' '
        val.pop(); // Remove ';'

        if val.is_empty() {
            None
        } else {
            HeaderValue::from_maybe_shared(Bytes::from(val)).ok()
        }
    }
}

impl Default for CookieJar {
    fn default() -> Self {
        Self::new()
    }
}

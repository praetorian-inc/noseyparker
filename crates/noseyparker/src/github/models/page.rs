use crate::github::Result;
use lazy_static::lazy_static;
use regex::{Regex, RegexBuilder};
use url::Url;

// -------------------------------------------------------------------------------------------------
// Page
// -------------------------------------------------------------------------------------------------
pub struct Page<T> {
    pub items: Vec<T>,
    pub links: HeaderLinks,
}

impl<T: serde::de::DeserializeOwned> Page<T> {
    pub async fn from_response(response: reqwest::Response) -> Result<Self> {
        let links = get_header_links(&response)?;
        let items = response.json().await?;
        Ok(Page { items, links })
    }
}

#[derive(Debug)]
pub struct HeaderLinks {
    pub next: Option<Url>,
    // NOTE: these could be parsed out of the headers, but are not currently used, so we ignore them
    // pub prev: Option<Url>,
    // pub first: Option<Url>,
    // pub last: Option<Url>,
}

lazy_static! {
    static ref HEADER_LINKS_PATTERN: Regex =
        RegexBuilder::new(r#"<([^>]+)>; \s* rel \s* = \s* "next""#)
            .ignore_whitespace(true)
            .build()
            .expect("header links regex should compile");
}

fn get_header_links(response: &reqwest::Response) -> Result<HeaderLinks> {
    let mut next = None;

    let headers = response.headers();

    // println!("*** {headers:#?}");
    for value in headers.get_all(reqwest::header::LINK) {
        // println!("*** {value:#?}");

        let value = match value.to_str() {
            Ok(v) => v,
            Err(_) => continue,
        };

        let captures = match HEADER_LINKS_PATTERN.captures(value) {
            Some(v) => v,
            None => continue,
        };

        let capture = match captures.get(1) {
            Some(v) => v,
            None => continue,
        };

        let url = match Url::parse(capture.as_str()) {
            Ok(v) => v,
            Err(_) => continue,
        };

        next = Some(url);
        break;
    }

    Ok(HeaderLinks { next })
}

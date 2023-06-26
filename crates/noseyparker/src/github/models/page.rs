use crate::github::Result;
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
    pub prev: Option<Url>,
    pub first: Option<Url>,
    pub last: Option<Url>,
}

fn get_header_links(response: &reqwest::Response) -> Result<HeaderLinks> {
    use hyperx::header::{RelationType, TypedHeaders};

    let mut first = None;
    let mut prev = None;
    let mut next = None;
    let mut last = None;

    // println!("{:#?}", response.headers());
    if let Ok(link_header) = response.headers().decode::<hyperx::header::Link>() {
        for value in link_header.values() {
            if let Some(relations) = value.rel() {
                for reltype in relations {
                    let dst = match reltype {
                        RelationType::First => Some(&mut first),
                        RelationType::Last => Some(&mut last),
                        RelationType::Next => Some(&mut next),
                        RelationType::Prev => Some(&mut prev),
                        _ => None,
                    };
                    if let Some(dst) = dst {
                        *dst = Some(Url::parse(value.link())?);
                    }
                }
            }
        }
    }

    Ok(HeaderLinks {
        first,
        prev,
        next,
        last,
    })
}

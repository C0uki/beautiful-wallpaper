//! Image-board search.
//!
//! The original's "Anime" tab searches a handful of booru sites by tag. This
//! is the same thing, minus two of its providers: Zerochan has no tag search
//! in its API at all (upstream substitutes the colour parameter, which does
//! not do what a user typing tags expects), and `t.alcy.cc` is a random-image
//! CDN with no metadata to show. The five that remain are the ones where
//! typing a tag actually searches for it.
//!
//! Boards host adult work alongside everything else, so **the rating filter is
//! part of the query, not a pass over the results**. A client-side filter is
//! one forgotten branch away from displaying what it was meant to exclude,
//! and it wastes a request either way. The filter is on unless it is switched
//! off deliberately, and the tab it belongs to is hidden entirely by default
//! (`policies.weeb`, 0 as it ships).

use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

use crate::wallpaper::online::OnlineError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum BooruProvider {
    /// yande.re and Konachan share the Moebooru API; only the host differs.
    Yandere,
    Konachan,
    Danbooru,
    Gelbooru,
    /// Gelbooru's API on a board that carries only safe-rated work.
    Safebooru,
}

impl BooruProvider {
    pub fn parse(name: &str) -> Result<Self, OnlineError> {
        match name {
            "yandere" => Ok(Self::Yandere),
            "konachan" => Ok(Self::Konachan),
            "danbooru" => Ok(Self::Danbooru),
            "gelbooru" => Ok(Self::Gelbooru),
            "safebooru" => Ok(Self::Safebooru),
            other => Err(OnlineError::UnknownProvider(other.to_owned())),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Yandere => "yandere",
            Self::Konachan => "konachan",
            Self::Danbooru => "danbooru",
            Self::Gelbooru => "gelbooru",
            Self::Safebooru => "safebooru",
        }
    }

    /// The site itself, for the "open in browser" link on a result.
    pub fn site(self) -> &'static str {
        match self {
            Self::Yandere => "https://yande.re",
            Self::Konachan => "https://konachan.net",
            Self::Danbooru => "https://danbooru.donmai.us",
            Self::Gelbooru => "https://gelbooru.com",
            Self::Safebooru => "https://safebooru.org",
        }
    }

    /// Whether the board can return anything but safe-rated work.
    ///
    /// Safebooru cannot, so the shell does not offer it a toggle that would
    /// change nothing.
    pub fn has_adult_content(self) -> bool {
        !matches!(self, Self::Safebooru)
    }

    /// The tag that restricts a search to safe-rated work on this board.
    ///
    /// Gelbooru renamed `safe` to `general`; sending the wrong one is silently
    /// ignored rather than rejected, which would leave the filter off.
    fn safe_tag(self) -> &'static str {
        match self {
            Self::Gelbooru | Self::Safebooru => "rating:general",
            _ => "rating:safe",
        }
    }
}

/// What to search for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BooruQuery {
    pub provider: BooruProvider,
    /// Space-separated tags, as the boards themselves take them.
    pub tags: String,
    /// One-based, as every one of these APIs counts.
    pub page: u32,
    pub limit: u32,
    /// Off unless switched on deliberately. Ignored on boards that carry no
    /// adult work.
    pub allow_adult: bool,
}

impl Default for BooruQuery {
    fn default() -> Self {
        Self {
            provider: BooruProvider::Safebooru,
            tags: String::new(),
            page: 1,
            limit: 30,
            allow_adult: false,
        }
    }
}

/// One result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BooruItem {
    pub id: String,
    pub width: u32,
    pub height: u32,
    /// A small image for the grid.
    pub preview: String,
    /// The full image, for setting as a wallpaper.
    pub file: String,
    pub tags: String,
    /// The board's own rating letter — `s`, `q`, `e` — normalised.
    pub rating: String,
    pub adult: bool,
    /// The post's page on the board.
    pub page_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BooruPage {
    pub items: Vec<BooruItem>,
    pub page: u32,
}

/// The tags actually sent, with the rating filter applied.
///
/// Public so the caller can show what was searched — and so the filter is
/// testable on its own rather than only through a whole URL.
pub fn effective_tags(query: &BooruQuery) -> String {
    let tags = query.tags.trim();

    // Safebooru has nothing else to return, so its filter is implicit; adding
    // the tag would only narrow an already-safe board.
    if query.allow_adult && query.provider.has_adult_content() {
        return tags.to_owned();
    }
    if query.provider == BooruProvider::Safebooru {
        return tags.to_owned();
    }

    if tags.is_empty() {
        query.provider.safe_tag().to_owned()
    } else {
        format!("{tags} {}", query.provider.safe_tag())
    }
}

/// Builds the request URL.
pub fn request_url(query: &BooruQuery) -> String {
    let tags = encode(&effective_tags(query));
    let limit = query.limit.clamp(1, 100);
    let page = query.page.max(1);

    match query.provider {
        // Moebooru: one-based pages.
        BooruProvider::Yandere => {
            format!("https://yande.re/post.json?tags={tags}&limit={limit}&page={page}")
        }
        BooruProvider::Konachan => {
            format!("https://konachan.net/post.json?tags={tags}&limit={limit}&page={page}")
        }
        BooruProvider::Danbooru => {
            format!("https://danbooru.donmai.us/posts.json?tags={tags}&limit={limit}&page={page}")
        }
        // Gelbooru's `pid` is a zero-based *page* index, not an offset, and
        // not the same parameter as everyone else's `page`.
        BooruProvider::Gelbooru => format!(
            "https://gelbooru.com/index.php?page=dapi&s=post&q=index&json=1\
             &tags={tags}&limit={limit}&pid={}",
            page - 1
        ),
        BooruProvider::Safebooru => format!(
            "https://safebooru.org/index.php?page=dapi&s=post&q=index&json=1\
             &tags={tags}&limit={limit}&pid={}",
            page - 1
        ),
    }
}

/// Parses a response into the common shape.
pub fn parse_page(
    provider: BooruProvider,
    body: &str,
    page: u32,
) -> Result<BooruPage, OnlineError> {
    let root: Value = serde_json::from_str(body).map_err(|source| OnlineError::BadResponse {
        provider: provider.as_str(),
        detail: source.to_string(),
    })?;

    // Gelbooru answers either a bare array or `{"post": [...]}` depending on
    // whether there were results; Safebooru returns an empty *string* for no
    // results. None of that is an error worth showing.
    let items = match &root {
        Value::Array(items) => items.clone(),
        Value::Object(object) => object
            .get("post")
            .and_then(|posts| posts.as_array())
            .cloned()
            .unwrap_or_default(),
        _ => Vec::new(),
    };

    Ok(BooruPage {
        items: items
            .iter()
            .filter_map(|item| parse_item(provider, item))
            .collect(),
        page,
    })
}

fn parse_item(provider: BooruProvider, item: &Value) -> Option<BooruItem> {
    let id = item
        .get("id")
        .map(|id| match id {
            Value::Number(number) => number.to_string(),
            Value::String(text) => text.clone(),
            _ => String::new(),
        })
        .filter(|id| !id.is_empty())?;

    let rating = normalise_rating(item.get("rating").and_then(|r| r.as_str()).unwrap_or("s"));

    let (preview, file) = match provider {
        BooruProvider::Danbooru => (
            text(item, "preview_file_url")?,
            text(item, "large_file_url").or_else(|| text(item, "file_url"))?,
        ),
        BooruProvider::Gelbooru | BooruProvider::Safebooru => {
            // Gelbooru's JSON API gives absolute URLs; Safebooru's older one
            // gives bare file names that need its image host prefixed.
            let preview = text(item, "preview_url")
                .or_else(|| directory_url(item, "thumbnails", "thumbnail_"))?;
            let file = text(item, "file_url").or_else(|| directory_url(item, "images", ""))?;
            (preview, file)
        }
        _ => (
            text(item, "preview_url")?,
            text(item, "file_url").or_else(|| text(item, "sample_url"))?,
        ),
    };

    let tags = text(item, "tags")
        .or_else(|| text(item, "tag_string"))
        .unwrap_or_default();

    Some(BooruItem {
        page_url: post_url(provider, &id),
        id,
        width: number(item, "width")
            .or_else(|| number(item, "image_width"))
            .unwrap_or(0),
        height: number(item, "height")
            .or_else(|| number(item, "image_height"))
            .unwrap_or(0),
        preview,
        file,
        tags,
        adult: rating != "s",
        rating,
    })
}

/// Safebooru's older API returns a directory and a file name rather than a URL.
fn directory_url(item: &Value, folder: &str, prefix: &str) -> Option<String> {
    let directory = item.get("directory")?.as_str()?;
    let image = item.get("image")?.as_str()?;
    let name = if prefix.is_empty() {
        image.to_owned()
    } else {
        // Thumbnails are always JPEG whatever the original was.
        let stem = image
            .rsplit_once('.')
            .map(|(stem, _)| stem)
            .unwrap_or(image);
        format!("{prefix}{stem}.jpg")
    };
    Some(format!("https://safebooru.org/{folder}/{directory}/{name}"))
}

fn post_url(provider: BooruProvider, id: &str) -> String {
    match provider {
        BooruProvider::Gelbooru | BooruProvider::Safebooru => {
            format!("{}/index.php?page=post&s=view&id={id}", provider.site())
        }
        _ => format!("{}/posts/{id}", provider.site()),
    }
}

/// Boards spell their ratings differently; the shell only cares whether a
/// result is safe.
fn normalise_rating(rating: &str) -> String {
    match rating {
        "s" | "safe" | "g" | "general" => "s",
        "q" | "questionable" | "sensitive" => "q",
        _ => "e",
    }
    .to_owned()
}

fn text(item: &Value, key: &str) -> Option<String> {
    let value = item.get(key)?.as_str()?;
    (!value.is_empty()).then(|| value.to_owned())
}

fn number(item: &Value, key: &str) -> Option<u32> {
    item.get(key)?.as_u64().map(|value| value as u32)
}

fn encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            b' ' => out.push_str("%20"),
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOARDS: [BooruProvider; 5] = [
        BooruProvider::Yandere,
        BooruProvider::Konachan,
        BooruProvider::Danbooru,
        BooruProvider::Gelbooru,
        BooruProvider::Safebooru,
    ];

    fn query(provider: BooruProvider, tags: &str, allow_adult: bool) -> BooruQuery {
        BooruQuery {
            provider,
            tags: tags.to_owned(),
            allow_adult,
            ..BooruQuery::default()
        }
    }

    #[test]
    fn every_board_restricts_to_safe_work_by_default() {
        // The point of the whole module. If this ever regresses, the tab shows
        // what it was built not to show.
        for provider in BOARDS {
            let url = request_url(&query(provider, "landscape", false));
            assert!(
                url.contains("rating%3A") || provider == BooruProvider::Safebooru,
                "{} sent no rating filter: {url}",
                provider.as_str()
            );
        }
    }

    #[test]
    fn the_filter_survives_an_empty_search() {
        // Browsing with no tags at all is the common case, and the one where a
        // naive "append to the tag list" leaves a dangling space or nothing.
        for provider in BOARDS {
            if provider == BooruProvider::Safebooru {
                continue;
            }
            let tags = effective_tags(&query(provider, "", false));
            assert_eq!(tags, provider.safe_tag(), "{}", provider.as_str());
        }
    }

    #[test]
    fn gelbooru_gets_its_own_spelling_of_the_filter() {
        // Gelbooru ignores `rating:safe` silently rather than rejecting it,
        // so the wrong spelling would leave the filter off with no error.
        assert!(
            effective_tags(&query(BooruProvider::Gelbooru, "cat", false))
                .contains("rating:general")
        );
        assert!(
            effective_tags(&query(BooruProvider::Yandere, "cat", false)).contains("rating:safe")
        );
    }

    #[test]
    fn the_filter_lifts_only_when_asked_and_only_where_it_means_something() {
        assert_eq!(
            effective_tags(&query(BooruProvider::Yandere, "cat", true)),
            "cat"
        );
        // Safebooru has nothing else to return, so the toggle changes nothing
        // there and the tag is not added either way.
        assert_eq!(
            effective_tags(&query(BooruProvider::Safebooru, "cat", false)),
            "cat"
        );
        assert!(!BooruProvider::Safebooru.has_adult_content());
    }

    #[test]
    fn tags_are_encoded_so_a_colon_or_space_cannot_break_the_query() {
        let url = request_url(&query(BooruProvider::Yandere, "blue sky", false));
        assert!(url.contains("blue%20sky"));
        assert!(url.contains("rating%3Asafe"));
        assert!(!url.contains("rating:safe"), "a bare colon reached the URL");
    }

    #[test]
    fn gelbooru_pages_from_zero_and_everyone_else_from_one() {
        let mut wanted = query(BooruProvider::Gelbooru, "cat", false);
        wanted.page = 3;
        assert!(request_url(&wanted).contains("pid=2"));

        wanted.provider = BooruProvider::Yandere;
        assert!(request_url(&wanted).contains("page=3"));
    }

    #[test]
    fn a_page_of_zero_does_not_underflow() {
        let mut wanted = query(BooruProvider::Gelbooru, "cat", false);
        wanted.page = 0;
        // `page - 1` on a u32 zero would panic in debug and wrap in release.
        assert!(request_url(&wanted).contains("pid=0"));
    }

    #[test]
    fn a_moebooru_response_parses() {
        let body = r#"[{"id":123,"width":1920,"height":1080,"rating":"s",
                        "tags":"scenery sky","preview_url":"https://e/p.jpg",
                        "file_url":"https://e/f.png"}]"#;
        let page = parse_page(BooruProvider::Yandere, body, 1).unwrap();
        assert_eq!(page.items.len(), 1);

        let item = &page.items[0];
        assert_eq!(item.id, "123");
        assert_eq!(item.width, 1920);
        assert!(!item.adult);
        assert_eq!(item.page_url, "https://yande.re/posts/123");
    }

    #[test]
    fn a_danbooru_response_uses_its_own_field_names() {
        let body = r#"[{"id":7,"image_width":800,"image_height":600,"rating":"g",
                        "tag_string":"cat","preview_file_url":"https://e/p.jpg",
                        "large_file_url":"https://e/l.jpg"}]"#;
        let item = &parse_page(BooruProvider::Danbooru, body, 1).unwrap().items[0];
        assert_eq!(item.width, 800);
        assert_eq!(item.tags, "cat");
        // Danbooru's `g` is everyone else's `s`.
        assert_eq!(item.rating, "s");
        assert!(!item.adult);
    }

    #[test]
    fn gelbooru_wraps_its_results_in_an_object_when_there_are_any() {
        let body = r#"{"@attributes":{"count":1},
                       "post":[{"id":9,"width":10,"height":10,"rating":"general",
                                "tags":"a","preview_url":"https://e/p.jpg",
                                "file_url":"https://e/f.jpg"}]}"#;
        let page = parse_page(BooruProvider::Gelbooru, body, 1).unwrap();
        assert_eq!(page.items.len(), 1);
        assert_eq!(
            page.items[0].page_url,
            "https://gelbooru.com/index.php?page=post&s=view&id=9"
        );
    }

    #[test]
    fn no_results_is_an_empty_page_rather_than_an_error() {
        // Each board says "nothing found" differently, and none of them is a
        // failure the user should see as one.
        for body in ["[]", r#"{"@attributes":{"count":0}}"#, r#""""#] {
            let page = parse_page(BooruProvider::Gelbooru, body, 1).unwrap();
            assert!(page.items.is_empty(), "{body}");
        }
    }

    #[test]
    fn safebooru_builds_urls_from_a_directory_and_a_file_name() {
        // Its older API returns no URLs at all.
        let body = r#"[{"id":5,"width":100,"height":100,"rating":"safe","tags":"a",
                        "directory":"12/34","image":"abc.png"}]"#;
        let item = &parse_page(BooruProvider::Safebooru, body, 1).unwrap().items[0];
        assert_eq!(item.file, "https://safebooru.org/images/12/34/abc.png");
        // Thumbnails are JPEG whatever the original is.
        assert_eq!(
            item.preview,
            "https://safebooru.org/thumbnails/12/34/thumbnail_abc.jpg"
        );
    }

    #[test]
    fn a_result_with_no_usable_image_is_dropped_rather_than_shown_broken() {
        let body = r#"[{"id":1,"rating":"s","tags":"a"},
                       {"id":2,"rating":"s","tags":"b",
                        "preview_url":"https://e/p.jpg","file_url":"https://e/f.jpg"}]"#;
        let page = parse_page(BooruProvider::Yandere, body, 1).unwrap();
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].id, "2");
    }

    #[test]
    fn malformed_json_is_reported_with_the_board_that_sent_it() {
        let error = parse_page(BooruProvider::Danbooru, "{ not json", 1).unwrap_err();
        assert!(error.to_string().contains("danbooru"));
    }

    #[test]
    fn provider_names_round_trip() {
        for provider in BOARDS {
            assert_eq!(BooruProvider::parse(provider.as_str()).unwrap(), provider);
        }
        assert!(BooruProvider::parse("nope").is_err());
    }
}

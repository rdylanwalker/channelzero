use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
struct WatchmodeSearchResponse {
    title_results: Option<Vec<WatchmodeSearchItem>>,
}

#[derive(Debug, Deserialize)]
struct WatchmodeSearchItem {
    id: i64,
}

#[derive(Debug, Deserialize)]
struct WatchmodeSource {
    name: String,
    web_url: Option<String>,
    region: Option<String>,
}

fn watchmode_name_to_slug(name: &str) -> Option<&'static str> {
    let lower = name.to_lowercase();
    if lower.contains("netflix") {
        Some("netflix")
    } else if lower == "max" || lower.contains("hbo") {
        Some("max")
    } else if lower.contains("disney") {
        Some("disney_plus")
    } else if lower.contains("prime") || lower.contains("amazon") {
        Some("prime_video")
    } else if lower.contains("hulu") {
        Some("hulu")
    } else if lower.contains("apple") {
        Some("apple_tv_plus")
    } else if lower.contains("peacock") {
        Some("peacock")
    } else if lower.contains("paramount") {
        Some("paramount_plus")
    } else {
        None
    }
}

pub async fn fetch_watchmode_links(
    client: &Client,
    api_key: &str,
    imdb_id: &str,
    region: &str,
) -> Option<HashMap<String, String>> {
    let search_url = format!(
        "https://api.watchmode.com/v1/search/?apiKey={api_key}&search_field=imdb_id&search_value={imdb_id}"
    );
    let resp = client.get(&search_url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let search: WatchmodeSearchResponse = resp.json().await.ok()?;
    let wm_id = search.title_results?.into_iter().next()?.id;

    let sources_url = format!(
        "https://api.watchmode.com/v1/title/{wm_id}/sources/?apiKey={api_key}&regions={region}"
    );
    let s_resp = client.get(&sources_url).send().await.ok()?;
    if !s_resp.status().is_success() {
        return None;
    }
    let sources: Vec<WatchmodeSource> = s_resp.json().await.ok()?;

    let mut map = HashMap::new();
    for s in sources {
        if let Some(r) = &s.region {
            if !r.eq_ignore_ascii_case(region) {
                continue;
            }
        }
        if let (Some(slug), Some(url)) = (watchmode_name_to_slug(&s.name), s.web_url) {
            if !url.is_empty() {
                map.entry(slug.to_string()).or_insert(url);
            }
        }
    }

    if map.is_empty() {
        None
    } else {
        Some(map)
    }
}

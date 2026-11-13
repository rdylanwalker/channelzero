use anyhow::Result;
use chrono::{Duration, NaiveDate};
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use tracing::debug;

use crate::config::{provider_meta_by_id, provider_search_url, tvmaze_channel_to_slug, AppConfig};
use crate::models::{MediaType, ProviderLink, ReleaseKind, SourceRating};
use crate::sync::RawProgram;

#[derive(Debug, Deserialize)]
struct TvMazeScheduleEntry {
    name: Option<String>,
    season: Option<i32>,
    number: Option<i32>,
    airdate: Option<String>,
    #[serde(rename = "show")]
    show_opt: Option<TvMazeShow>,
    #[serde(rename = "_embedded")]
    embedded: Option<TvMazeEmbedded>,
}

#[derive(Debug, Deserialize)]
struct TvMazeEmbedded {
    show: Option<TvMazeShow>,
}

#[derive(Debug, Deserialize)]
struct TvMazeShow {
    id: i64,
    name: String,
    url: Option<String>,
    #[serde(rename = "officialSite")]
    official_site: Option<String>,
    premiered: Option<String>,
    genres: Option<Vec<String>>,
    summary: Option<String>,
    rating: Option<TvMazeRating>,
    image: Option<TvMazeImage>,
    network: Option<TvMazeChannel>,
    #[serde(rename = "webChannel")]
    web_channel: Option<TvMazeChannel>,
    externals: Option<TvMazeExternals>,
}

#[derive(Debug, Deserialize)]
struct TvMazeRating {
    average: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct TvMazeImage {
    medium: Option<String>,
    original: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TvMazeChannel {
    name: String,
}

#[derive(Debug, Deserialize)]
struct TvMazeExternals {
    imdb: Option<String>,
}

fn strip_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_tag = false;
    for c in input.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.trim().to_string()
}

fn matches_provider_domain(url: &str, slug: &str) -> bool {
    let lower = url.to_lowercase();
    match slug {
        "netflix" => lower.contains("netflix.com"),
        "max" => lower.contains("max.com") || lower.contains("hbomax.com"),
        "disney_plus" => lower.contains("disneyplus.com"),
        "prime_video" => lower.contains("amazon.com") || lower.contains("primevideo.com"),
        "hulu" => lower.contains("hulu.com"),
        "apple_tv_plus" => lower.contains("tv.apple.com"),
        "peacock" => lower.contains("peacocktv.com"),
        "paramount_plus" => lower.contains("paramountplus.com"),
        _ => false,
    }
}

pub async fn fetch_tvmaze_releases(
    client: &Client,
    config: &AppConfig,
    today: NaiveDate,
) -> Result<Vec<RawProgram>> {
    let mut dedup: HashMap<i64, RawProgram> = HashMap::new();

    // Fetch web/streaming schedule for the past new_release_days + today,
    // pacing requests gently when fetching more than 14 days to respect TVmaze rate limits.
    let lookback = config.new_release_days as i64;
    for offset in 0..=lookback {
        if offset > 0 && lookback > 14 {
            tokio::time::sleep(std::time::Duration::from_millis(80)).await;
        }
        let date = today - Duration::days(offset);
        let date_str = date.format("%Y-%m-%d").to_string();
        let url = format!("https://api.tvmaze.com/schedule/web?date={date_str}");

        let resp = match client.get(&url).send().await {
            Ok(r) => r,
            Err(e) => {
                debug!("TVmaze request failed for {date_str}: {e}");
                continue;
            }
        };

        if !resp.status().is_success() {
            debug!("TVmaze HTTP {} for {date_str}", resp.status());
            continue;
        }

        let entries: Vec<TvMazeScheduleEntry> = match resp.json().await {
            Ok(v) => v,
            Err(e) => {
                debug!("TVmaze JSON parse error for {date_str}: {e}");
                continue;
            }
        };

        for entry in entries {
            let show = match entry
                .show_opt
                .or_else(|| entry.embedded.and_then(|e| e.show))
            {
                Some(s) => s,
                None => continue,
            };

            let channel_name = show
                .web_channel
                .as_ref()
                .map(|c| c.name.as_str())
                .or_else(|| show.network.as_ref().map(|c| c.name.as_str()));

            let slug = match channel_name.and_then(tvmaze_channel_to_slug) {
                Some(s) if config.providers.iter().any(|p| p == s) => s,
                _ => continue,
            };

            let airdate = entry.airdate.unwrap_or_else(|| date_str.clone());
            let release_kind = match (entry.season, entry.number) {
                (Some(1), Some(1)) => ReleaseKind::NewTitle,
                _ => ReleaseKind::NewEpisode,
            };
            let episode_info = match (entry.season, entry.number, entry.name.as_deref()) {
                (Some(s), Some(e), Some(ep_name)) => Some(format!("S{s:02}E{e:02} · {ep_name}")),
                (Some(s), Some(e), None) => Some(format!("S{s:02}E{e:02}")),
                _ => entry.name,
            };

            let release_year = show
                .premiered
                .as_deref()
                .and_then(|p| p.get(0..4))
                .and_then(|y| y.parse::<i32>().ok());

            let provider_meta = provider_meta_by_id(slug);
            let provider_name = provider_meta
                .map(|m| m.name)
                .unwrap_or_else(|| slug.to_string());

            let (watch_url, is_direct) = match show.official_site.as_deref() {
                Some(site) if matches_provider_domain(site, slug) => (site.to_string(), true),
                _ => (provider_search_url(slug, &show.name), false),
            };

            let fallback_url = show
                .url
                .clone()
                .unwrap_or_else(|| format!("https://www.tvmaze.com/shows/{}", show.id));

            let mut ratings = Vec::new();
            let avg_rating = show.rating.as_ref().and_then(|r| r.average);
            if let Some(r) = avg_rating {
                ratings.push(SourceRating {
                    source: "TVmaze".into(),
                    score: format!("{r:.1}/10"),
                    normalized_10: Some(r),
                });
            }

            let poster_url = show
                .image
                .as_ref()
                .and_then(|i| i.medium.clone().or_else(|| i.original.clone()));

            let program = RawProgram {
                title: show.name,
                media_type: MediaType::Series,
                release_kind,
                release_date: airdate,
                release_year,
                episode_info,
                synopsis: show.summary.as_deref().map(strip_html),
                poster_url,
                backdrop_url: None,
                genres: show.genres.unwrap_or_default(),
                providers: vec![ProviderLink {
                    id: slug.to_string(),
                    name: provider_name,
                    url: watch_url,
                    is_direct,
                }],
                fallback_url,
                ratings,
                source: "TVmaze".into(),
                imdb_id: show.externals.and_then(|e| e.imdb),
                tmdb_id: None,
                tvmaze_id: Some(show.id),
            };

            // Keep the most recent airdate per show
            dedup
                .entry(show.id)
                .and_modify(|existing| {
                    if program.release_date > existing.release_date {
                        *existing = program.clone();
                    }
                })
                .or_insert(program);
        }
    }

    Ok(dedup.into_values().collect())
}

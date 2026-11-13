use anyhow::Result;
use chrono::{Duration, NaiveDate};
use reqwest::Client;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tracing::debug;

use crate::config::{
    provider_meta_by_id, provider_search_url, slug_to_tmdb_provider_ids, tmdb_provider_id_to_slug,
    AppConfig,
};
use crate::models::{MediaType, ProviderLink, ReleaseKind, SourceRating};
use crate::sync::RawProgram;

#[derive(Debug, Deserialize)]
struct TmdbMultiSearchResponse {
    results: Vec<TmdbMultiSearchItem>,
}

#[derive(Debug, Deserialize)]
struct TmdbMultiSearchItem {
    id: i64,
    media_type: Option<String>,
    title: Option<String>,
    name: Option<String>,
    overview: Option<String>,
    poster_path: Option<String>,
    backdrop_path: Option<String>,
    release_date: Option<String>,
    first_air_date: Option<String>,
    vote_average: Option<f64>,
    genre_ids: Option<Vec<i64>>,
}

#[derive(Debug, Deserialize)]
struct TmdbDiscoverResponse {
    results: Vec<TmdbDiscoverItem>,
    total_pages: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct TmdbDiscoverItem {
    id: i64,
    title: Option<String>,
    name: Option<String>,
    overview: Option<String>,
    poster_path: Option<String>,
    backdrop_path: Option<String>,
    release_date: Option<String>,
    first_air_date: Option<String>,
    vote_average: Option<f64>,
    genre_ids: Option<Vec<i64>>,
}

#[derive(Debug, Deserialize)]
struct TmdbWatchProvidersResponse {
    results: HashMap<String, TmdbRegionProviders>,
}

#[derive(Debug, Deserialize)]
struct TmdbRegionProviders {
    link: Option<String>,
    flatrate: Option<Vec<TmdbProviderEntry>>,
}

#[derive(Debug, Deserialize)]
struct TmdbProviderEntry {
    provider_id: i64,
}

#[derive(Debug, Deserialize)]
struct TmdbExternalIds {
    imdb_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TmdbEnrichResponse {
    #[serde(rename = "watch/providers")]
    watch_providers: Option<TmdbWatchProvidersResponse>,
    external_ids: Option<TmdbExternalIds>,
    last_air_date: Option<String>,
    last_episode_to_air: Option<TmdbEpisodeSummary>,
}

#[derive(Debug, Deserialize)]
struct TmdbEpisodeSummary {
    name: Option<String>,
    air_date: Option<String>,
    season_number: Option<i32>,
    episode_number: Option<i32>,
}

fn tmdb_genre_name(id: i64) -> Option<&'static str> {
    match id {
        28 => Some("Action"),
        12 => Some("Adventure"),
        16 => Some("Animation"),
        35 => Some("Comedy"),
        80 => Some("Crime"),
        99 => Some("Documentary"),
        18 => Some("Drama"),
        10751 => Some("Family"),
        14 => Some("Fantasy"),
        36 => Some("History"),
        27 => Some("Horror"),
        10402 => Some("Music"),
        9648 => Some("Mystery"),
        10749 => Some("Romance"),
        878 => Some("Science-Fiction"),
        10770 => Some("TV Movie"),
        53 => Some("Thriller"),
        10752 => Some("War"),
        37 => Some("Western"),
        10759 => Some("Action & Adventure"),
        10762 => Some("Kids"),
        10763 => Some("News"),
        10764 => Some("Reality"),
        10765 => Some("Sci-Fi & Fantasy"),
        10766 => Some("Soap"),
        10767 => Some("Talk"),
        10768 => Some("War & Politics"),
        _ => None,
    }
}

pub async fn fetch_tmdb_releases(
    client: &Client,
    config: &AppConfig,
    api_key: &str,
    today: NaiveDate,
    lookback_days: u32,
) -> Result<Vec<RawProgram>> {
    let min_date = (today - Duration::days(lookback_days as i64))
        .format("%Y-%m-%d")
        .to_string();
    let max_date = today.format("%Y-%m-%d").to_string();

    let tmdb_provider_ids: Vec<String> = config
        .providers
        .iter()
        .flat_map(|slug| slug_to_tmdb_provider_ids(slug).iter().copied())
        .map(|id| id.to_string())
        .collect();

    if tmdb_provider_ids.is_empty() {
        return Ok(Vec::new());
    }
    let watch_providers_param = tmdb_provider_ids.join("|");

    let max_pages = discover_page_limit(lookback_days);
    let mut seen_movie_ids: HashSet<i64> = HashSet::new();
    let mut seen_tv_ids: HashSet<i64> = HashSet::new();
    let mut movie_candidates: Vec<TmdbDiscoverItem> = Vec::new();
    let mut new_tv_candidates: Vec<TmdbDiscoverItem> = Vec::new();
    let mut returning_tv_candidates: Vec<TmdbDiscoverItem> = Vec::new();

    // 1. Discover Movies (paginated)
    for page in 1..=max_pages {
        let movie_url = format!(
            "https://api.themoviedb.org/3/discover/movie?api_key={api_key}&watch_region={}&with_watch_providers={watch_providers_param}&primary_release_date.gte={min_date}&primary_release_date.lte={max_date}&sort_by=popularity.desc&page={page}",
            config.region
        );
        match client.get(&movie_url).send().await {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(data) = resp.json::<TmdbDiscoverResponse>().await {
                    let total_pages = data.total_pages.unwrap_or(1);
                    if data.results.is_empty() {
                        break;
                    }
                    for item in data.results {
                        if seen_movie_ids.insert(item.id) {
                            movie_candidates.push(item);
                        }
                    }
                    if page >= total_pages {
                        break;
                    }
                } else {
                    break;
                }
            }
            Ok(resp) => {
                debug!("TMDB discover movie HTTP {}", resp.status());
                break;
            }
            Err(_) => break,
        }
    }

    // 2a. Discover New TV Series (paginated, filtered by first_air_date)
    for page in 1..=max_pages {
        let tv_url = format!(
            "https://api.themoviedb.org/3/discover/tv?api_key={api_key}&watch_region={}&with_watch_providers={watch_providers_param}&first_air_date.gte={min_date}&first_air_date.lte={max_date}&sort_by=popularity.desc&page={page}",
            config.region
        );
        match client.get(&tv_url).send().await {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(data) = resp.json::<TmdbDiscoverResponse>().await {
                    let total_pages = data.total_pages.unwrap_or(1);
                    if data.results.is_empty() {
                        break;
                    }
                    for item in data.results {
                        if seen_tv_ids.insert(item.id) {
                            new_tv_candidates.push(item);
                        }
                    }
                    if page >= total_pages {
                        break;
                    }
                } else {
                    break;
                }
            }
            Ok(resp) => {
                debug!("TMDB discover tv HTTP {}", resp.status());
                break;
            }
            Err(_) => break,
        }
    }

    // 2b. Discover Returning TV Seasons / New Episodes (paginated, filtered by air_date)
    for page in 1..=max_pages {
        let tv_air_url = format!(
            "https://api.themoviedb.org/3/discover/tv?api_key={api_key}&watch_region={}&with_watch_providers={watch_providers_param}&air_date.gte={min_date}&air_date.lte={max_date}&sort_by=popularity.desc&page={page}",
            config.region
        );
        match client.get(&tv_air_url).send().await {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(data) = resp.json::<TmdbDiscoverResponse>().await {
                    let total_pages = data.total_pages.unwrap_or(1);
                    if data.results.is_empty() {
                        break;
                    }
                    for item in data.results {
                        if seen_tv_ids.insert(item.id) {
                            returning_tv_candidates.push(item);
                        }
                    }
                    if page >= total_pages {
                        break;
                    }
                } else {
                    break;
                }
            }
            Ok(resp) => {
                debug!("TMDB discover returning tv HTTP {}", resp.status());
                break;
            }
            Err(_) => break,
        }
    }

    // 3. Enrich all candidates concurrently using a single append_to_response call per item.
    let sem = Arc::new(Semaphore::new(12));
    let mut set = JoinSet::new();

    for item in movie_candidates {
        let permit = sem.clone();
        let client = client.clone();
        let config = config.clone();
        let api_key = api_key.to_string();
        set.spawn(async move {
            let _guard = permit.acquire().await.ok()?;
            enrich_tmdb_item(&client, &config, &api_key, item, MediaType::Movie, None).await
        });
    }

    for item in new_tv_candidates {
        let permit = sem.clone();
        let client = client.clone();
        let config = config.clone();
        let api_key = api_key.to_string();
        set.spawn(async move {
            let _guard = permit.acquire().await.ok()?;
            enrich_tmdb_item(&client, &config, &api_key, item, MediaType::Series, None).await
        });
    }

    for item in returning_tv_candidates {
        let permit = sem.clone();
        let client = client.clone();
        let config = config.clone();
        let api_key = api_key.to_string();
        let min_d = min_date.clone();
        let max_d = max_date.clone();
        set.spawn(async move {
            let _guard = permit.acquire().await.ok()?;
            enrich_tmdb_item(
                &client,
                &config,
                &api_key,
                item,
                MediaType::Series,
                Some((&min_d, &max_d)),
            )
            .await
        });
    }

    let mut out = Vec::new();
    while let Some(res) = set.join_next().await {
        if let Ok(Some(prog)) = res {
            out.push(prog);
        }
    }

    Ok(out)
}

fn encode_query_component(input: &str) -> String {
    input
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            b' ' => "+".to_string(),
            _ => format!("%{:02X}", b),
        })
        .collect()
}

pub async fn search_tmdb_catalog(
    client: &Client,
    config: &AppConfig,
    api_key: &str,
    query: &str,
) -> Result<Vec<RawProgram>> {
    let encoded = encode_query_component(query.trim());
    let url = format!(
        "https://api.themoviedb.org/3/search/multi?api_key={api_key}&query={encoded}&include_adult=false"
    );
    let resp = client.get(&url).send().await?;
    if !resp.status().is_success() {
        return Ok(Vec::new());
    }
    let data: TmdbMultiSearchResponse = match resp.json().await {
        Ok(v) => v,
        Err(_) => return Ok(Vec::new()),
    };

    let sem = Arc::new(Semaphore::new(8));
    let mut set = JoinSet::new();

    for entry in data.results.into_iter().take(12) {
        let media_type = match entry.media_type.as_deref() {
            Some("movie") => MediaType::Movie,
            Some("tv") => MediaType::Series,
            _ => continue,
        };
        let item = TmdbDiscoverItem {
            id: entry.id,
            title: entry.title,
            name: entry.name,
            overview: entry.overview,
            poster_path: entry.poster_path,
            backdrop_path: entry.backdrop_path,
            release_date: entry.release_date,
            first_air_date: entry.first_air_date,
            vote_average: entry.vote_average,
            genre_ids: entry.genre_ids,
        };
        let permit = sem.clone();
        let client = client.clone();
        let config = config.clone();
        let api_key = api_key.to_string();
        set.spawn(async move {
            let _guard = permit.acquire().await.ok()?;
            enrich_tmdb_item(&client, &config, &api_key, item, media_type, None).await
        });
    }

    let mut out = Vec::new();
    while let Some(res) = set.join_next().await {
        if let Ok(Some(prog)) = res {
            out.push(prog);
        }
    }
    Ok(out)
}

fn discover_page_limit(new_release_days: u32) -> u32 {
    new_release_days.div_ceil(15).clamp(1, 5)
}

fn is_date_in_window(date: &str, min_date: &str, max_date: &str) -> bool {
    !date.is_empty() && date >= min_date && date <= max_date
}

fn format_tmdb_episode_info(ep: &TmdbEpisodeSummary) -> Option<String> {
    let ep_name = ep.name.as_deref().map(str::trim).filter(|s| !s.is_empty());
    match (ep.season_number, ep.episode_number, ep_name) {
        (Some(s), Some(e), Some(name)) => Some(format!("S{s:02}E{e:02} · {name}")),
        (Some(s), Some(e), None) => Some(format!("S{s:02}E{e:02}")),
        _ => ep_name.map(str::to_string),
    }
}

async fn enrich_tmdb_item(
    client: &Client,
    config: &AppConfig,
    api_key: &str,
    item: TmdbDiscoverItem,
    media_type: MediaType,
    returning_window: Option<(&str, &str)>,
) -> Option<RawProgram> {
    let title = match media_type {
        MediaType::Movie => item.title.clone()?,
        MediaType::Series => item.name.clone()?,
    };
    let base_release_date = match media_type {
        MediaType::Movie => item.release_date.clone().unwrap_or_default(),
        MediaType::Series => item.first_air_date.clone().unwrap_or_default(),
    };
    if base_release_date.is_empty() {
        return None;
    }
    let release_year = base_release_date
        .get(0..4)
        .and_then(|y| y.parse::<i32>().ok());

    let kind_path = match media_type {
        MediaType::Movie => "movie",
        MediaType::Series => "tv",
    };

    let enrich_url = format!(
        "https://api.themoviedb.org/3/{kind_path}/{}?api_key={api_key}&append_to_response=watch/providers,external_ids",
        item.id
    );
    let details: TmdbEnrichResponse = match client.get(&enrich_url).send().await {
        Ok(resp) if resp.status().is_success() => resp.json().await.ok()?,
        _ => return None,
    };

    let (release_kind, release_date, episode_info) =
        if let Some((min_date, max_date)) = returning_window {
            let ep_air_date = details
                .last_episode_to_air
                .as_ref()
                .and_then(|e| e.air_date.as_deref())
                .filter(|d| !d.is_empty())
                .or(details.last_air_date.as_deref())?;
            if !is_date_in_window(ep_air_date, min_date, max_date) {
                return None;
            }
            let ep_info = details
                .last_episode_to_air
                .as_ref()
                .and_then(format_tmdb_episode_info);
            (ReleaseKind::NewEpisode, ep_air_date.to_string(), ep_info)
        } else {
            (ReleaseKind::NewTitle, base_release_date, None)
        };

    let mut providers = Vec::new();
    let mut tmdb_watch_link = format!("https://www.themoviedb.org/{kind_path}/{}", item.id);

    if let Some(wp) = details.watch_providers {
        if let Some(region_data) = wp.results.get(&config.region) {
            if let Some(link) = &region_data.link {
                tmdb_watch_link = link.clone();
            }
            if let Some(flatrate) = &region_data.flatrate {
                for entry in flatrate {
                    if let Some(slug) = tmdb_provider_id_to_slug(entry.provider_id) {
                        if config.providers.iter().any(|p| p == slug)
                            && !providers.iter().any(|p: &ProviderLink| p.id == slug)
                        {
                            let name = provider_meta_by_id(slug)
                                .map(|m| m.name)
                                .unwrap_or_else(|| slug.to_string());
                            providers.push(ProviderLink {
                                id: slug.to_string(),
                                name,
                                url: provider_search_url(slug, &title),
                                is_direct: false,
                            });
                        }
                    }
                }
            }
        }
    }

    if providers.is_empty() {
        return None;
    }

    let imdb_id = details.external_ids.and_then(|e| e.imdb_id);

    let genres = item
        .genre_ids
        .unwrap_or_default()
        .into_iter()
        .filter_map(|id| tmdb_genre_name(id).map(|s| s.to_string()))
        .collect();

    let mut ratings = Vec::new();
    let rating = item.vote_average.filter(|&v| v > 0.0);
    if let Some(r) = rating {
        ratings.push(SourceRating {
            source: "TMDB".into(),
            score: format!("{r:.1}/10"),
            normalized_10: Some(r),
        });
    }

    Some(RawProgram {
        title,
        media_type,
        release_kind,
        release_date,
        release_year,
        episode_info,
        synopsis: item.overview.filter(|s| !s.trim().is_empty()),
        poster_url: item
            .poster_path
            .map(|p| format!("https://image.tmdb.org/t/p/w500{p}")),
        backdrop_url: item
            .backdrop_path
            .map(|p| format!("https://image.tmdb.org/t/p/w780{p}")),
        genres,
        providers,
        fallback_url: tmdb_watch_link,
        ratings,
        source: "TMDB".into(),
        imdb_id,
        tmdb_id: Some(item.id),
        tvmaze_id: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discover_page_limit_scales_with_days() {
        assert_eq!(discover_page_limit(7), 1);
        assert_eq!(discover_page_limit(15), 1);
        assert_eq!(discover_page_limit(30), 2);
        assert_eq!(discover_page_limit(60), 4);
        assert_eq!(discover_page_limit(90), 5);
        assert_eq!(discover_page_limit(365), 5);
    }

    #[test]
    fn date_window_check_handles_bounds_and_empty() {
        assert!(!is_date_in_window("", "2026-10-01", "2026-10-31"));
        assert!(!is_date_in_window("2026-09-30", "2026-10-01", "2026-10-31"));
        assert!(is_date_in_window("2026-10-01", "2026-10-01", "2026-10-31"));
        assert!(is_date_in_window("2026-10-15", "2026-10-01", "2026-10-31"));
        assert!(is_date_in_window("2026-10-31", "2026-10-01", "2026-10-31"));
        assert!(!is_date_in_window("2026-11-01", "2026-10-01", "2026-10-31"));
    }

    #[test]
    fn episode_info_formats_season_episode_and_name() {
        let full = TmdbEpisodeSummary {
            name: Some("The Ascension".into()),
            air_date: Some("2026-06-24".into()),
            season_number: Some(4),
            episode_number: Some(12),
        };
        assert_eq!(
            format_tmdb_episode_info(&full).as_deref(),
            Some("S04E12 · The Ascension")
        );

        let no_name = TmdbEpisodeSummary {
            name: Some("  ".into()),
            air_date: None,
            season_number: Some(4),
            episode_number: Some(1),
        };
        assert_eq!(
            format_tmdb_episode_info(&no_name).as_deref(),
            Some("S04E01")
        );

        let only_name = TmdbEpisodeSummary {
            name: Some("Special".into()),
            air_date: None,
            season_number: None,
            episode_number: None,
        };
        assert_eq!(
            format_tmdb_episode_info(&only_name).as_deref(),
            Some("Special")
        );
    }
}

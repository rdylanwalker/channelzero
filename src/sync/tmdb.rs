use anyhow::Result;
use chrono::{Duration, NaiveDate};
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use tracing::debug;

use crate::config::{
    provider_meta_by_id, provider_search_url, slug_to_tmdb_provider_ids, tmdb_provider_id_to_slug,
    AppConfig,
};
use crate::models::{MediaType, ProviderLink, ReleaseKind, SourceRating};
use crate::sync::RawProgram;

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

    let mut out = Vec::new();
    let max_pages = discover_page_limit(lookback_days);

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
                        if let Some(prog) =
                            enrich_tmdb_item(client, config, api_key, item, MediaType::Movie).await
                        {
                            out.push(prog);
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

    // 2. Discover TV Series (paginated)
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
                        if let Some(prog) =
                            enrich_tmdb_item(client, config, api_key, item, MediaType::Series).await
                        {
                            out.push(prog);
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

    Ok(out)
}

fn discover_page_limit(new_release_days: u32) -> u32 {
    new_release_days.div_ceil(15).clamp(1, 5)
}

async fn enrich_tmdb_item(
    client: &Client,
    config: &AppConfig,
    api_key: &str,
    item: TmdbDiscoverItem,
    media_type: MediaType,
) -> Option<RawProgram> {
    let title = match media_type {
        MediaType::Movie => item.title.clone()?,
        MediaType::Series => item.name.clone()?,
    };
    let release_date = match media_type {
        MediaType::Movie => item.release_date.clone().unwrap_or_default(),
        MediaType::Series => item.first_air_date.clone().unwrap_or_default(),
    };
    if release_date.is_empty() {
        return None;
    }
    let release_year = release_date.get(0..4).and_then(|y| y.parse::<i32>().ok());

    let kind_path = match media_type {
        MediaType::Movie => "movie",
        MediaType::Series => "tv",
    };

    // Fetch regional watch providers
    let wp_url = format!(
        "https://api.themoviedb.org/3/{kind_path}/{}/watch/providers?api_key={api_key}",
        item.id
    );
    let mut providers = Vec::new();
    let mut tmdb_watch_link = format!("https://www.themoviedb.org/{kind_path}/{}", item.id);

    if let Ok(resp) = client.get(&wp_url).send().await {
        if resp.status().is_success() {
            if let Ok(wp) = resp.json::<TmdbWatchProvidersResponse>().await {
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
        }
    }

    if providers.is_empty() {
        return None;
    }

    // Fetch external IDs (IMDb)
    let ext_url = format!(
        "https://api.themoviedb.org/3/{kind_path}/{}/external_ids?api_key={api_key}",
        item.id
    );
    let imdb_id = match client.get(&ext_url).send().await {
        Ok(resp) if resp.status().is_success() => resp
            .json::<TmdbExternalIds>()
            .await
            .ok()
            .and_then(|e| e.imdb_id),
        _ => None,
    };

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
        release_kind: ReleaseKind::NewTitle,
        release_date,
        release_year,
        episode_info: None,
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
}

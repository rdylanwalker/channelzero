pub mod omdb;
pub mod tmdb;
pub mod tvmaze;
pub mod watchmode;

use chrono::{NaiveDate, Utc};
use chrono_tz::Tz;
use reqwest::Client;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tracing::{error, info};

use crate::config::AppConfig;
use crate::db::Database;
use crate::models::{MediaType, ProviderLink, ReleaseKind, SourceRating, SourceStatus};

#[derive(Debug, Clone)]
pub struct RawProgram {
    pub title: String,
    pub media_type: MediaType,
    pub release_kind: ReleaseKind,
    pub release_date: String,
    pub release_year: Option<i32>,
    pub episode_info: Option<String>,
    pub synopsis: Option<String>,
    pub poster_url: Option<String>,
    pub backdrop_url: Option<String>,
    pub genres: Vec<String>,
    pub providers: Vec<ProviderLink>,
    pub fallback_url: String,
    pub ratings: Vec<SourceRating>,
    pub source: String,
    pub imdb_id: Option<String>,
    pub tmdb_id: Option<i64>,
    pub tvmaze_id: Option<i64>,
}

#[derive(Debug, Clone)]
struct MergedProgram {
    pub dedup_key: String,
    pub title: String,
    pub media_type: MediaType,
    pub release_kind: ReleaseKind,
    pub release_date: String,
    pub release_year: Option<i32>,
    pub episode_info: Option<String>,
    pub synopsis: Option<String>,
    pub poster_url: Option<String>,
    pub backdrop_url: Option<String>,
    pub genres: Vec<String>,
    pub providers: Vec<ProviderLink>,
    pub fallback_url: String,
    pub ratings: Vec<SourceRating>,
    pub sources: Vec<String>,
    pub imdb_id: Option<String>,
    pub tmdb_id: Option<i64>,
    pub tvmaze_id: Option<i64>,
}

fn normalize_title(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || c.is_ascii_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn make_dedup_key(title: &str, year: Option<i32>, media_type: &MediaType) -> String {
    let norm = normalize_title(title);
    let y = year.map(|v| v.to_string()).unwrap_or_else(|| "0".into());
    format!("{norm}|{y}|{}", media_type.as_str())
}

fn today_in_tz(tz_name: &str) -> NaiveDate {
    let tz: Tz = tz_name.parse().unwrap_or(chrono_tz::UTC);
    Utc::now().with_timezone(&tz).date_naive()
}

pub async fn run_sync(
    db: &Database,
    config: &AppConfig,
    is_syncing: Arc<AtomicBool>,
) -> anyhow::Result<()> {
    if is_syncing
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        info!("Sync already in progress, skipping duplicate trigger.");
        return Ok(());
    }

    let result = run_sync_inner(db, config).await;
    is_syncing.store(false, Ordering::SeqCst);
    result
}

async fn run_sync_inner(db: &Database, config: &AppConfig) -> anyhow::Result<()> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .user_agent("ChannelZero/0.1 (local-streaming-dashboard)")
        .build()?;

    let today = today_in_tz(&config.timezone);
    let now_str = Utc::now().to_rfc3339();
    let max_profile_days = db.max_profile_new_release_days().await?.unwrap_or(0);
    let effective_days = config.new_release_days.max(max_profile_days).clamp(1, 365);
    let mut all_raw: Vec<RawProgram> = Vec::new();

    // 1 & 2. Fetch TVmaze and TMDB concurrently
    let tmdb_key = config.sources.tmdb.api_key.as_deref().unwrap_or("");
    let tvmaze_fut = async {
        if config.sources.tvmaze.enabled {
            Some(tvmaze::fetch_tvmaze_releases(&client, config, today, effective_days).await)
        } else {
            None
        }
    };
    let tmdb_fut = async {
        if config.sources.tmdb.enabled && !tmdb_key.is_empty() {
            Some(tmdb::fetch_tmdb_releases(&client, config, tmdb_key, today, effective_days).await)
        } else {
            None
        }
    };
    let (tvmaze_res, tmdb_res) = tokio::join!(tvmaze_fut, tmdb_fut);

    match tvmaze_res {
        Some(Ok(items)) => {
            let count = items.len() as i64;
            info!("TVmaze fetched {count} items");
            all_raw.extend(items);
            db.upsert_source_status(&SourceStatus {
                name: "TVmaze".into(),
                tier: "Anonymous / Free".into(),
                enabled: true,
                configured: true,
                last_sync_at: Some(now_str.clone()),
                last_status: "ok".into(),
                last_error: None,
                items_fetched: count,
            })
            .await?;
        }
        Some(Err(e)) => {
            error!("TVmaze sync error: {e}");
            db.upsert_source_status(&SourceStatus {
                name: "TVmaze".into(),
                tier: "Anonymous / Free".into(),
                enabled: true,
                configured: true,
                last_sync_at: Some(now_str.clone()),
                last_status: "error".into(),
                last_error: Some(e.to_string()),
                items_fetched: 0,
            })
            .await?;
        }
        None => {
            db.upsert_source_status(&SourceStatus {
                name: "TVmaze".into(),
                tier: "Anonymous / Free".into(),
                enabled: false,
                configured: true,
                last_sync_at: None,
                last_status: "disabled".into(),
                last_error: None,
                items_fetched: 0,
            })
            .await?;
        }
    }

    match tmdb_res {
        Some(Ok(items)) => {
            let count = items.len() as i64;
            info!("TMDB fetched {count} items");
            all_raw.extend(items);
            db.upsert_source_status(&SourceStatus {
                name: "TMDB".into(),
                tier: "Free API Key".into(),
                enabled: true,
                configured: true,
                last_sync_at: Some(now_str.clone()),
                last_status: "ok".into(),
                last_error: None,
                items_fetched: count,
            })
            .await?;
        }
        Some(Err(e)) => {
            error!("TMDB sync error: {e}");
            db.upsert_source_status(&SourceStatus {
                name: "TMDB".into(),
                tier: "Free API Key".into(),
                enabled: true,
                configured: true,
                last_sync_at: Some(now_str.clone()),
                last_status: "error".into(),
                last_error: Some(e.to_string()),
                items_fetched: 0,
            })
            .await?;
        }
        None => {
            db.upsert_source_status(&SourceStatus {
                name: "TMDB".into(),
                tier: "Free API Key".into(),
                enabled: config.sources.tmdb.enabled,
                configured: !tmdb_key.is_empty(),
                last_sync_at: None,
                last_status: if config.sources.tmdb.enabled {
                    "missing_key".into()
                } else {
                    "disabled".into()
                },
                last_error: if config.sources.tmdb.enabled && tmdb_key.is_empty() {
                    Some("TMDB_API_KEY not configured".into())
                } else {
                    None
                },
                items_fetched: 0,
            })
            .await?;
        }
    }

    // Merge & deduplicate raw items
    let mut merged = merge_programs(all_raw, config);

    // 3. Free-Tier Enrichment: OMDb
    let omdb_key = config.sources.omdb.api_key.as_deref().unwrap_or("");
    if config.sources.omdb.enabled && !omdb_key.is_empty() {
        let mut enriched_count = 0i64;
        let sem = Arc::new(Semaphore::new(8));
        let mut set = JoinSet::new();
        for (idx, prog) in merged.iter().take(35).enumerate() {
            if let Some(imdb_id) = prog.imdb_id.clone() {
                let permit = sem.clone();
                let client = client.clone();
                let key = omdb_key.to_string();
                set.spawn(async move {
                    let _guard = permit.acquire().await.ok()?;
                    let ratings = omdb::fetch_omdb_ratings(&client, &key, &imdb_id).await?;
                    Some((idx, ratings))
                });
            }
        }
        while let Some(res) = set.join_next().await {
            if let Ok(Some((idx, extra_ratings))) = res {
                let prog = &mut merged[idx];
                for r in extra_ratings {
                    if !prog
                        .ratings
                        .iter()
                        .any(|existing| existing.source == r.source)
                    {
                        prog.ratings.push(r);
                    }
                }
                if !prog.sources.iter().any(|s| s == "OMDb") {
                    prog.sources.push("OMDb".into());
                }
                enriched_count += 1;
            }
        }
        db.upsert_source_status(&SourceStatus {
            name: "OMDb".into(),
            tier: "Free API Key".into(),
            enabled: true,
            configured: true,
            last_sync_at: Some(now_str.clone()),
            last_status: "ok".into(),
            last_error: None,
            items_fetched: enriched_count,
        })
        .await?;
    } else {
        db.upsert_source_status(&SourceStatus {
            name: "OMDb".into(),
            tier: "Free API Key".into(),
            enabled: config.sources.omdb.enabled,
            configured: !omdb_key.is_empty(),
            last_sync_at: None,
            last_status: if config.sources.omdb.enabled {
                "missing_key".into()
            } else {
                "disabled".into()
            },
            last_error: if config.sources.omdb.enabled && omdb_key.is_empty() {
                Some("OMDB_API_KEY not configured".into())
            } else {
                None
            },
            items_fetched: 0,
        })
        .await?;
    }

    // 4. Optional / Paid Enrichment: Watchmode
    let wm_key = config.sources.watchmode.api_key.as_deref().unwrap_or("");
    if config.sources.watchmode.enabled && !wm_key.is_empty() {
        let mut enriched_count = 0i64;
        let sem = Arc::new(Semaphore::new(8));
        let mut set = JoinSet::new();
        for (idx, prog) in merged.iter().take(20).enumerate() {
            if let Some(imdb_id) = prog.imdb_id.clone() {
                let permit = sem.clone();
                let client = client.clone();
                let key = wm_key.to_string();
                let region = config.region.clone();
                set.spawn(async move {
                    let _guard = permit.acquire().await.ok()?;
                    let links =
                        watchmode::fetch_watchmode_links(&client, &key, &imdb_id, &region).await?;
                    Some((idx, links))
                });
            }
        }
        while let Some(res) = set.join_next().await {
            if let Ok(Some((idx, links))) = res {
                let prog = &mut merged[idx];
                for prov in prog.providers.iter_mut() {
                    if let Some(direct_url) = links.get(&prov.id) {
                        prov.url = direct_url.clone();
                        prov.is_direct = true;
                    }
                }
                if !prog.sources.iter().any(|s| s == "Watchmode") {
                    prog.sources.push("Watchmode".into());
                }
                enriched_count += 1;
            }
        }
        db.upsert_source_status(&SourceStatus {
            name: "Watchmode".into(),
            tier: "Optional / Paid Key".into(),
            enabled: true,
            configured: true,
            last_sync_at: Some(now_str.clone()),
            last_status: "ok".into(),
            last_error: None,
            items_fetched: enriched_count,
        })
        .await?;
    } else {
        db.upsert_source_status(&SourceStatus {
            name: "Watchmode".into(),
            tier: "Optional / Paid Key".into(),
            enabled: config.sources.watchmode.enabled,
            configured: !wm_key.is_empty(),
            last_sync_at: None,
            last_status: if config.sources.watchmode.enabled {
                "missing_key".into()
            } else {
                "disabled".into()
            },
            last_error: if config.sources.watchmode.enabled && wm_key.is_empty() {
                Some("WATCHMODE_API_KEY not configured".into())
            } else {
                None
            },
            items_fetched: 0,
        })
        .await?;
    }

    // Persist merged programs into SQLite in a single transaction to avoid per-row fsyncs.
    let total_merged = merged.len();
    let mut tx = db.pool.begin().await?;
    for prog in merged {
        upsert_merged_program_exec(&mut *tx, prog, &now_str).await?;
    }
    tx.commit().await?;
    info!("Persisted {total_merged} merged programs");

    Ok(())
}

fn merge_programs(raw_items: Vec<RawProgram>, config: &AppConfig) -> Vec<MergedProgram> {
    let mut by_key: HashMap<String, MergedProgram> = HashMap::new();
    let mut imdb_to_key: HashMap<String, String> = HashMap::new();

    for raw in raw_items {
        let candidate_key = raw
            .imdb_id
            .as_ref()
            .and_then(|imdb| imdb_to_key.get(imdb).cloned())
            .unwrap_or_else(|| make_dedup_key(&raw.title, raw.release_year, &raw.media_type));

        if let Some(imdb) = &raw.imdb_id {
            imdb_to_key
                .entry(imdb.clone())
                .or_insert_with(|| candidate_key.clone());
        }

        by_key
            .entry(candidate_key.clone())
            .and_modify(|existing| {
                // Keep latest release date & episode info if newer
                if raw.release_date > existing.release_date {
                    existing.release_date = raw.release_date.clone();
                    existing.release_kind = raw.release_kind.clone();
                    if raw.episode_info.is_some() {
                        existing.episode_info = raw.episode_info.clone();
                    }
                } else if raw.release_date == existing.release_date {
                    if existing.episode_info.is_none() && raw.episode_info.is_some() {
                        existing.episode_info = raw.episode_info.clone();
                    }
                    if raw.release_kind == ReleaseKind::NewEpisode
                        && existing.release_kind == ReleaseKind::NewTitle
                    {
                        existing.release_kind = ReleaseKind::NewEpisode;
                    }
                }
                if existing.synopsis.is_none() {
                    existing.synopsis = raw.synopsis.clone();
                }
                if existing.poster_url.is_none() {
                    existing.poster_url = raw.poster_url.clone();
                }
                if existing.backdrop_url.is_none() {
                    existing.backdrop_url = raw.backdrop_url.clone();
                }
                if existing.imdb_id.is_none() {
                    existing.imdb_id = raw.imdb_id.clone();
                }
                if existing.tmdb_id.is_none() {
                    existing.tmdb_id = raw.tmdb_id;
                }
                if existing.tvmaze_id.is_none() {
                    existing.tvmaze_id = raw.tvmaze_id;
                }
                for g in &raw.genres {
                    if !existing.genres.iter().any(|eg| eg.eq_ignore_ascii_case(g)) {
                        existing.genres.push(g.clone());
                    }
                }
                for p in &raw.providers {
                    if let Some(ep) = existing.providers.iter_mut().find(|ep| ep.id == p.id) {
                        if p.is_direct && !ep.is_direct {
                            ep.url = p.url.clone();
                            ep.is_direct = true;
                        }
                    } else {
                        existing.providers.push(p.clone());
                    }
                }
                for r in &raw.ratings {
                    if !existing.ratings.iter().any(|er| er.source == r.source) {
                        existing.ratings.push(r.clone());
                    }
                }
                if !existing.sources.iter().any(|s| s == &raw.source) {
                    existing.sources.push(raw.source.clone());
                }
            })
            .or_insert_with(|| MergedProgram {
                dedup_key: candidate_key,
                title: raw.title,
                media_type: raw.media_type,
                release_kind: raw.release_kind,
                release_date: raw.release_date,
                release_year: raw.release_year,
                episode_info: raw.episode_info,
                synopsis: raw.synopsis,
                poster_url: raw.poster_url,
                backdrop_url: raw.backdrop_url,
                genres: raw.genres,
                providers: raw.providers,
                fallback_url: raw.fallback_url,
                ratings: raw.ratings,
                sources: vec![raw.source],
                imdb_id: raw.imdb_id,
                tmdb_id: raw.tmdb_id,
                tvmaze_id: raw.tvmaze_id,
            });
    }

    let mut list: Vec<MergedProgram> = by_key.into_values().collect();
    for item in &mut list {
        item.providers
            .sort_by_key(|p| config.provider_priority(&p.id));
    }
    list.sort_by(|a, b| b.release_date.cmp(&a.release_date));
    list
}

#[cfg(test)]
async fn upsert_merged_program(
    db: &Database,
    _config: &AppConfig,
    prog: MergedProgram,
    now_str: &str,
) -> anyhow::Result<()> {
    upsert_merged_program_exec(&db.pool, prog, now_str).await
}

async fn upsert_merged_program_exec<'e, E>(
    exec: E,
    prog: MergedProgram,
    now_str: &str,
) -> anyhow::Result<()>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    let primary_url = prog
        .providers
        .first()
        .map(|p| p.url.clone())
        .unwrap_or_else(|| prog.fallback_url.clone());

    let avg_rating = {
        let vals: Vec<f64> = prog
            .ratings
            .iter()
            .filter_map(|r| r.normalized_10)
            .collect();
        if vals.is_empty() {
            None
        } else {
            Some(vals.iter().sum::<f64>() / vals.len() as f64)
        }
    };

    let id = uuid::Uuid::new_v4().to_string();
    let genres_json = serde_json::to_string(&prog.genres)?;
    let providers_json = serde_json::to_string(&prog.providers)?;
    let ratings_json = serde_json::to_string(&prog.ratings)?;
    let sources_json = serde_json::to_string(&prog.sources)?;

    sqlx::query(
        r#"
        INSERT INTO programs (
            id, dedup_key, title, media_type, release_kind, release_date, release_year,
            episode_info, synopsis, poster_url, backdrop_url, genres_json, providers_json,
            primary_url, rating, ratings_json, sources_json, imdb_id, tmdb_id, tvmaze_id, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(dedup_key) DO UPDATE SET
            title = excluded.title,
            release_kind = excluded.release_kind,
            release_date = MAX(programs.release_date, excluded.release_date),
            episode_info = COALESCE(excluded.episode_info, programs.episode_info),
            synopsis = COALESCE(excluded.synopsis, programs.synopsis),
            poster_url = COALESCE(excluded.poster_url, programs.poster_url),
            backdrop_url = COALESCE(excluded.backdrop_url, programs.backdrop_url),
            genres_json = excluded.genres_json,
            providers_json = excluded.providers_json,
            primary_url = excluded.primary_url,
            rating = COALESCE(excluded.rating, programs.rating),
            ratings_json = excluded.ratings_json,
            sources_json = excluded.sources_json,
            imdb_id = COALESCE(excluded.imdb_id, programs.imdb_id),
            tmdb_id = COALESCE(excluded.tmdb_id, programs.tmdb_id),
            tvmaze_id = COALESCE(excluded.tvmaze_id, programs.tvmaze_id),
            updated_at = excluded.updated_at
        "#,
    )
    .bind(id)
    .bind(prog.dedup_key)
    .bind(prog.title)
    .bind(prog.media_type.as_str())
    .bind(prog.release_kind.as_str())
    .bind(prog.release_date)
    .bind(prog.release_year)
    .bind(prog.episode_info)
    .bind(prog.synopsis)
    .bind(prog.poster_url)
    .bind(prog.backdrop_url)
    .bind(genres_json)
    .bind(providers_json)
    .bind(primary_url)
    .bind(avg_rating)
    .bind(ratings_json)
    .bind(sources_json)
    .bind(prog.imdb_id)
    .bind(prog.tmdb_id)
    .bind(prog.tvmaze_id)
    .bind(now_str)
    .execute(exec)
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RawConfig;
    use chrono::Datelike;

    fn test_config() -> AppConfig {
        let raw = RawConfig::default();
        AppConfig {
            port: raw.port,
            data_dir: "/tmp".into(),
            static_dir: "/tmp".into(),
            region: raw.region,
            timezone: raw.timezone,
            new_release_days: raw.new_release_days,
            refresh_interval_hours: raw.refresh_interval_hours,
            providers: raw.providers,
            sources: raw.sources,
        }
    }

    fn make_raw(title: &str, source: &str) -> RawProgram {
        RawProgram {
            title: title.into(),
            media_type: MediaType::Series,
            release_kind: ReleaseKind::NewTitle,
            release_date: "2026-11-10".into(),
            release_year: Some(2026),
            episode_info: None,
            synopsis: None,
            poster_url: None,
            backdrop_url: None,
            genres: vec!["Drama".into()],
            providers: vec![],
            fallback_url: "https://example.com/fallback".into(),
            ratings: vec![],
            source: source.into(),
            imdb_id: None,
            tmdb_id: None,
            tvmaze_id: None,
        }
    }

    #[test]
    fn normalize_title_and_dedup_key() {
        assert_eq!(
            normalize_title("  The Bear: Season 1! "),
            "the bear season 1"
        );
        assert_eq!(
            normalize_title("Spider-Man: No Way Home"),
            "spiderman no way home"
        );

        assert_eq!(
            make_dedup_key("The Bear!", Some(2022), &MediaType::Series),
            "the bear|2022|series"
        );
        assert_eq!(
            make_dedup_key("Dune", None, &MediaType::Movie),
            "dune|0|movie"
        );
    }

    #[test]
    fn today_in_tz_handles_valid_and_invalid() {
        let d1 = today_in_tz("America/New_York");
        let d2 = today_in_tz("Invalid/Timezone");
        assert!(d1.year() >= 2024);
        assert!(d2.year() >= 2024);
    }

    #[test]
    fn merge_programs_dedups_by_imdb_and_key_and_sorts_providers() {
        let cfg = test_config();

        let mut p1 = make_raw("The Bear", "TVmaze");
        p1.imdb_id = Some("tt14452776".into());
        p1.tvmaze_id = Some(500);
        p1.release_date = "2026-11-08".into();
        p1.providers.push(ProviderLink {
            id: "hulu".into(),
            name: "Hulu".into(),
            url: "https://hulu.com/search".into(),
            is_direct: false,
        });
        p1.ratings.push(SourceRating {
            source: "TVmaze".into(),
            score: "8.5/10".into(),
            normalized_10: Some(8.5),
        });

        let mut p2 = make_raw("The Bear (US)", "TMDB");
        p2.imdb_id = Some("tt14452776".into());
        p2.tmdb_id = Some(136315);
        p2.release_date = "2026-11-12".into();
        p2.episode_info = Some("S04E01".into());
        p2.synopsis = Some("Kitchen chaos".into());
        p2.poster_url = Some("https://img/poster.jpg".into());
        p2.backdrop_url = Some("https://img/backdrop.jpg".into());
        p2.genres = vec!["drama".into(), "Comedy".into()];
        p2.providers.push(ProviderLink {
            id: "hulu".into(),
            name: "Hulu".into(),
            url: "https://hulu.com/watch/1".into(),
            is_direct: true,
        });
        p2.providers.push(ProviderLink {
            id: "netflix".into(),
            name: "Netflix".into(),
            url: "https://netflix.com/1".into(),
            is_direct: false,
        });
        p2.ratings.push(SourceRating {
            source: "TMDB".into(),
            score: "8.7/10".into(),
            normalized_10: Some(8.7),
        });

        let mut p3 = make_raw("Older Movie", "TMDB");
        p3.media_type = MediaType::Movie;
        p3.release_date = "2026-11-01".into();

        let merged = merge_programs(vec![p1, p2, p3], &cfg);
        assert_eq!(merged.len(), 2);
        // Sorted by release_date descending
        assert_eq!(merged[0].title, "The Bear");
        assert_eq!(merged[0].release_date, "2026-11-12");
        assert_eq!(merged[0].episode_info.as_deref(), Some("S04E01"));
        assert_eq!(merged[0].synopsis.as_deref(), Some("Kitchen chaos"));
        assert_eq!(merged[0].genres, vec!["Drama", "Comedy"]);
        assert_eq!(merged[0].sources, vec!["TVmaze", "TMDB"]);
        assert_eq!(merged[0].ratings.len(), 2);
        // Netflix has higher priority (index 0) than Hulu (index 4)
        assert_eq!(merged[0].providers[0].id, "netflix");
        assert_eq!(merged[0].providers[1].id, "hulu");
        // Direct URL from p2 overwrote non-direct URL from p1
        assert!(merged[0].providers[1].is_direct);
        assert_eq!(merged[0].providers[1].url, "https://hulu.com/watch/1");
    }

    #[tokio::test]
    async fn upsert_merged_program_and_run_sync_offline_states() {
        let tmp = std::env::temp_dir().join(format!("cz_sync_{}", uuid::Uuid::new_v4()));
        let db = Database::connect_and_init(tmp.to_str().unwrap())
            .await
            .unwrap();
        let mut cfg = test_config();

        // Upsert with ratings & no providers -> uses fallback_url and averages rating
        let prog = MergedProgram {
            dedup_key: "test|2026|movie".into(),
            title: "Test Movie".into(),
            media_type: MediaType::Movie,
            release_kind: ReleaseKind::NewTitle,
            release_date: "2026-11-10".into(),
            release_year: Some(2026),
            episode_info: None,
            synopsis: Some("Synopsis".into()),
            poster_url: None,
            backdrop_url: None,
            genres: vec!["Action".into()],
            providers: vec![],
            fallback_url: "https://example.com/fb".into(),
            ratings: vec![
                SourceRating {
                    source: "TMDB".into(),
                    score: "8.0".into(),
                    normalized_10: Some(8.0),
                },
                SourceRating {
                    source: "IMDb".into(),
                    score: "9.0".into(),
                    normalized_10: Some(9.0),
                },
            ],
            sources: vec!["TMDB".into()],
            imdb_id: Some("tt999".into()),
            tmdb_id: Some(42),
            tvmaze_id: None,
        };
        upsert_merged_program(&db, &cfg, prog, "2026-11-10T00:00:00Z")
            .await
            .unwrap();
        assert_eq!(db.count_programs().await.unwrap(), 1);

        // If is_syncing is already true, run_sync returns Ok immediately
        let flag = Arc::new(AtomicBool::new(true));
        run_sync(&db, &cfg, flag.clone()).await.unwrap();
        assert!(flag.load(Ordering::SeqCst));

        // Run sync with all sources disabled
        flag.store(false, Ordering::SeqCst);
        cfg.sources.tvmaze.enabled = false;
        cfg.sources.tmdb.enabled = false;
        cfg.sources.omdb.enabled = false;
        cfg.sources.watchmode.enabled = false;
        run_sync(&db, &cfg, flag.clone()).await.unwrap();
        assert!(!flag.load(Ordering::SeqCst));

        let statuses = db.get_source_statuses().await.unwrap();
        assert_eq!(statuses.len(), 4);
        assert!(statuses.iter().all(|s| s.last_status == "disabled"));

        // Run sync with keyed sources enabled but missing keys
        cfg.sources.tmdb.enabled = true;
        cfg.sources.omdb.enabled = true;
        cfg.sources.watchmode.enabled = true;
        run_sync(&db, &cfg, flag.clone()).await.unwrap();

        let statuses2 = db.get_source_statuses().await.unwrap();
        let tmdb_status = statuses2.iter().find(|s| s.name == "TMDB").unwrap();
        assert_eq!(tmdb_status.last_status, "missing_key");
        assert!(tmdb_status.last_error.is_some());

        let _ = std::fs::remove_dir_all(&tmp);
    }
}

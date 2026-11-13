use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{Duration, Utc};
use chrono_tz::Tz;
use serde::Deserialize;
use std::collections::HashSet;

use crate::models::ProgramDto;
use crate::routes::AppState;

#[derive(Debug, Deserialize)]
pub struct ProgramsQuery {
    pub profile_id: Option<String>,
    pub q: Option<String>,
    pub provider: Option<String>,
    pub media_type: Option<String>,
    pub release_kind: Option<String>,
    pub genre: Option<String>,
    pub status: Option<String>,
    pub min_rating: Option<f64>,
    pub sort: Option<String>,
    pub order: Option<String>,
    pub include_hidden: Option<bool>,
}

pub async fn list_programs(
    State(state): State<AppState>,
    Query(params): Query<ProgramsQuery>,
) -> Result<Json<Vec<ProgramDto>>, (StatusCode, String)> {
    let profile_id = params.profile_id.as_deref().unwrap_or("default");

    let profile_days = state
        .db
        .get_profile(profile_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .and_then(|p| p.new_release_days)
        .unwrap_or(state.config.new_release_days);

    let tz: Tz = state.config.timezone.parse().unwrap_or(chrono_tz::UTC);
    let today = Utc::now().with_timezone(&tz).date_naive();
    let min_date = (today - Duration::days(profile_days as i64))
        .format("%Y-%m-%d")
        .to_string();

    let mut programs = state
        .db
        .get_programs_for_profile(profile_id, &min_date)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let hidden_state = state
        .db
        .get_hidden_state(profile_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let hidden_genres: HashSet<String> = hidden_state
        .hidden_genres
        .into_iter()
        .map(|g| g.to_lowercase())
        .collect();

    let include_hidden = params.include_hidden.unwrap_or(false);
    let q_lower = params.q.as_deref().map(|s| s.trim().to_lowercase());

    programs.retain(|p| {
        if !include_hidden {
            if p.not_interested {
                return false;
            }
            if p.genres
                .iter()
                .any(|g| hidden_genres.contains(&g.to_lowercase()))
            {
                return false;
            }
        }

        if let Some(q) = &q_lower {
            if !q.is_empty() {
                let in_title = p.title.to_lowercase().contains(q);
                let in_synopsis = p
                    .synopsis
                    .as_deref()
                    .map(|s| s.to_lowercase().contains(q))
                    .unwrap_or(false);
                if !in_title && !in_synopsis {
                    return false;
                }
            }
        }

        if let Some(prov) = params
            .provider
            .as_deref()
            .filter(|s| !s.is_empty() && *s != "all")
        {
            if !p.providers.iter().any(|pr| pr.id == prov) {
                return false;
            }
        }

        if let Some(mt) = params
            .media_type
            .as_deref()
            .filter(|s| !s.is_empty() && *s != "all")
        {
            if p.media_type.as_str() != mt {
                return false;
            }
        }

        if let Some(rk) = params
            .release_kind
            .as_deref()
            .filter(|s| !s.is_empty() && *s != "all")
        {
            if p.release_kind.as_str() != rk {
                return false;
            }
        }

        if let Some(g) = params
            .genre
            .as_deref()
            .filter(|s| !s.is_empty() && *s != "all")
        {
            if !p.genres.iter().any(|pg| pg.eq_ignore_ascii_case(g)) {
                return false;
            }
        }

        if let Some(status) = params.status.as_deref() {
            match status {
                "unseen" => {
                    if p.seen || p.watched {
                        return false;
                    }
                }
                "seen" => {
                    if !p.seen {
                        return false;
                    }
                }
                "watched" => {
                    if !p.watched {
                        return false;
                    }
                }
                "watchlist" if !p.watchlist => return false,
                _ => {}
            }
        }

        if let Some(min_r) = params.min_rating {
            if p.rating.unwrap_or(0.0) < min_r {
                return false;
            }
        }

        true
    });

    let sort_field = params.sort.as_deref().unwrap_or("release_date");
    let asc = params.order.as_deref() == Some("asc");

    programs.sort_by(|a, b| {
        let ord = match sort_field {
            "rating" => a
                .rating
                .unwrap_or(-1.0)
                .partial_cmp(&b.rating.unwrap_or(-1.0))
                .unwrap_or(std::cmp::Ordering::Equal),
            "title" => a.title.to_lowercase().cmp(&b.title.to_lowercase()),
            _ => a.release_date.cmp(&b.release_date),
        };
        if asc {
            ord
        } else {
            ord.reverse()
        }
    });

    Ok(Json(programs))
}

#[derive(Debug, Deserialize)]
pub struct SingleProgramQuery {
    pub profile_id: Option<String>,
}

pub async fn get_program(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<SingleProgramQuery>,
) -> Result<Json<ProgramDto>, (StatusCode, String)> {
    let profile_id = params.profile_id.as_deref().unwrap_or("default");
    let item = state
        .db
        .get_program_by_id(profile_id, &id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Program not found".into()))?;
    Ok(Json(item))
}

pub async fn list_genres(
    State(state): State<AppState>,
    Query(params): Query<SingleProgramQuery>,
) -> Result<Json<Vec<String>>, (StatusCode, String)> {
    let profile_id = params.profile_id.as_deref().unwrap_or("default");

    let profile_days = state
        .db
        .get_profile(profile_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .and_then(|p| p.new_release_days)
        .unwrap_or(state.config.new_release_days);

    let tz: Tz = state.config.timezone.parse().unwrap_or(chrono_tz::UTC);
    let today = Utc::now().with_timezone(&tz).date_naive();
    let min_date = (today - Duration::days(profile_days as i64))
        .format("%Y-%m-%d")
        .to_string();

    let genres = state
        .db
        .get_available_genres(profile_id, &min_date)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(genres))
}

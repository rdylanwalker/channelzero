use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;

use crate::models::SyncStatusResponse;
use crate::routes::AppState;
use crate::sync;

#[derive(Debug, Deserialize)]
pub struct SyncProfileQuery {
    pub profile_id: Option<String>,
}

pub async fn get_sync_status(
    State(state): State<AppState>,
    Query(params): Query<SyncProfileQuery>,
) -> Result<Json<SyncStatusResponse>, (StatusCode, String)> {
    let sources = state
        .db
        .get_source_statuses()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let total_programs = state
        .db
        .count_programs()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let last_sync_at = sources.iter().filter_map(|s| s.last_sync_at.clone()).max();

    let profile = if let Some(pid) = params.profile_id.as_deref() {
        state.db.get_profile(pid).await.ok().flatten()
    } else {
        None
    };

    Ok(Json(SyncStatusResponse {
        last_sync_at,
        is_syncing: state.is_syncing.load(Ordering::SeqCst),
        total_programs,
        refresh_interval_hours: state.config.refresh_interval_hours,
        new_release_days: profile
            .as_ref()
            .and_then(|p| p.new_release_days)
            .unwrap_or(state.config.new_release_days),
        region: profile
            .and_then(|p| p.region)
            .unwrap_or_else(|| state.config.region.clone()),
        sources,
    }))
}

#[derive(Debug, Serialize)]
pub struct RefreshResponse {
    pub started: bool,
    pub already_running: bool,
}

pub async fn trigger_refresh(
    State(state): State<AppState>,
    Query(params): Query<SyncProfileQuery>,
) -> Json<RefreshResponse> {
    if state.is_syncing.load(Ordering::SeqCst) {
        return Json(RefreshResponse {
            started: false,
            already_running: true,
        });
    }

    let db = state.db.clone();
    let mut config = state.config.clone();
    if let Some(pid) = params.profile_id.as_deref() {
        if let Ok(Some(profile)) = state.db.get_profile(pid).await {
            if let Some(region) = profile.region {
                config.region = region;
            }
            if let Some(days) = profile.new_release_days {
                config.new_release_days = days;
            }
        }
    }
    let flag = state.is_syncing.clone();
    tokio::spawn(async move {
        if let Err(e) = sync::run_sync(&db, &config, flag).await {
            tracing::error!("Manual sync failed: {e}");
        }
    });

    Json(RefreshResponse {
        started: true,
        already_running: false,
    })
}

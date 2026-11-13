use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

use crate::models::{Profile, ProfileHiddenState};
use crate::routes::AppState;

pub async fn list_profiles(
    State(state): State<AppState>,
) -> Result<Json<Vec<Profile>>, (StatusCode, String)> {
    let profiles = state
        .db
        .list_profiles()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(profiles))
}

#[derive(Debug, Deserialize)]
pub struct CreateProfileReq {
    pub name: String,
}

pub async fn create_profile(
    State(state): State<AppState>,
    Json(body): Json<CreateProfileReq>,
) -> Result<(StatusCode, Json<Profile>), (StatusCode, String)> {
    let name = body.name.trim();
    if name.is_empty() || name.len() > 40 {
        return Err((
            StatusCode::BAD_REQUEST,
            "Profile name must be between 1 and 40 characters".into(),
        ));
    }
    let profile = state.db.create_profile(name).await.map_err(|e| {
        (
            StatusCode::CONFLICT,
            format!("Could not create profile: {e}"),
        )
    })?;
    Ok((StatusCode::CREATED, Json(profile)))
}

#[derive(Debug, Deserialize)]
pub struct UpdateProfileSettingsReq {
    pub region: Option<String>,
    pub new_release_days: Option<u32>,
}

pub async fn update_profile_settings(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateProfileSettingsReq>,
) -> Result<Json<Profile>, (StatusCode, String)> {
    let region = match body.region {
        Some(r) => {
            let trimmed = r.trim().to_uppercase();
            if trimmed.is_empty() {
                None
            } else if trimmed.len() != 2 || !trimmed.chars().all(|c| c.is_ascii_uppercase()) {
                return Err((
                    StatusCode::BAD_REQUEST,
                    format!(
                        "Invalid region '{trimmed}': must be a 2-letter ISO 3166-1 alpha-2 code (e.g. US, CA, GB)"
                    ),
                ));
            } else {
                Some(trimmed)
            }
        }
        None => None,
    };

    if let Some(days) = body.new_release_days {
        if !(1..=365).contains(&days) {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("Invalid new_release_days {days}: must be between 1 and 365"),
            ));
        }
    }

    let updated = state
        .db
        .update_profile_settings(&id, region, body.new_release_days)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Profile not found".into()))?;

    Ok(Json(updated))
}

pub async fn delete_profile(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    if id == "default" {
        return Err((
            StatusCode::BAD_REQUEST,
            "Cannot delete the default profile".into(),
        ));
    }
    let deleted = state
        .db
        .delete_profile(&id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((StatusCode::NOT_FOUND, "Profile not found".into()))
    }
}

#[derive(Debug, Deserialize)]
pub struct InteractionUpdateReq {
    pub seen: Option<bool>,
    pub watched: Option<bool>,
    pub watchlist: Option<bool>,
    pub not_interested: Option<bool>,
}

pub async fn update_interaction(
    State(state): State<AppState>,
    Path((profile_id, program_id)): Path<(String, String)>,
    Json(body): Json<InteractionUpdateReq>,
) -> Result<StatusCode, (StatusCode, String)> {
    state
        .db
        .upsert_interaction(
            &profile_id,
            &program_id,
            body.seen,
            body.watched,
            body.watchlist,
            body.not_interested,
        )
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_hidden_state(
    State(state): State<AppState>,
    Path(profile_id): Path<String>,
) -> Result<Json<ProfileHiddenState>, (StatusCode, String)> {
    let hidden = state
        .db
        .get_hidden_state(&profile_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(hidden))
}

#[derive(Debug, Deserialize)]
pub struct HideGenreReq {
    pub genre: String,
}

pub async fn hide_genre(
    State(state): State<AppState>,
    Path(profile_id): Path<String>,
    Json(body): Json<HideGenreReq>,
) -> Result<StatusCode, (StatusCode, String)> {
    let genre = body.genre.trim();
    if genre.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Genre cannot be empty".into()));
    }
    state
        .db
        .hide_genre(&profile_id, genre)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn unhide_genre(
    State(state): State<AppState>,
    Path((profile_id, genre)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, String)> {
    state
        .db
        .unhide_genre(&profile_id, &genre)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

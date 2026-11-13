pub mod profiles;
pub mod programs;
pub mod sync;

use axum::{
    extract::State,
    routing::{delete, get, post},
    Json, Router,
};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};

use crate::config::{provider_meta_by_id, AppConfig};
use crate::db::Database;
use crate::models::PublicConfig;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub config: AppConfig,
    pub is_syncing: Arc<AtomicBool>,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

async fn public_config(State(state): State<AppState>) -> Json<PublicConfig> {
    let providers = state
        .config
        .providers
        .iter()
        .filter_map(|id| provider_meta_by_id(id))
        .collect();

    Json(PublicConfig {
        region: state.config.region.clone(),
        timezone: state.config.timezone.clone(),
        new_release_days: state.config.new_release_days,
        refresh_interval_hours: state.config.refresh_interval_hours,
        providers,
    })
}

pub fn build_router(state: AppState) -> Router {
    let static_dir = PathBuf::from(&state.config.static_dir);
    let index_file = static_dir.join("index.html");

    let api = Router::new()
        .route("/health", get(health))
        .route("/config/public", get(public_config))
        .route("/programs", get(programs::list_programs))
        .route("/programs/{id}", get(programs::get_program))
        .route("/genres", get(programs::list_genres))
        .route(
            "/profiles",
            get(profiles::list_profiles).post(profiles::create_profile),
        )
        .route(
            "/profiles/{id}",
            delete(profiles::delete_profile).patch(profiles::update_profile_settings),
        )
        .route(
            "/profiles/{id}/interactions/{program_id}",
            post(profiles::update_interaction),
        )
        .route("/profiles/{id}/hidden", get(profiles::get_hidden_state))
        .route("/profiles/{id}/hidden-genres", post(profiles::hide_genre))
        .route(
            "/profiles/{id}/hidden-genres/{genre}",
            delete(profiles::unhide_genre),
        )
        .route("/sync/status", get(sync::get_sync_status))
        .route("/sync/refresh", post(sync::trigger_refresh));

    let spa = ServeDir::new(&static_dir).not_found_service(ServeFile::new(index_file));

    Router::new()
        .nest("/api", api)
        .fallback_service(spa)
        .layer(CorsLayer::permissive())
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    async fn test_app_with_state() -> (Router, AppState, PathBuf) {
        let tmp = std::env::temp_dir().join(format!("cz_test_{}", uuid::Uuid::new_v4()));
        let db = Database::connect_and_init(tmp.to_str().unwrap())
            .await
            .unwrap();

        let raw = crate::config::RawConfig::default();
        let mut config = AppConfig {
            port: raw.port,
            data_dir: tmp.to_string_lossy().to_string(),
            static_dir: tmp.to_string_lossy().to_string(),
            region: raw.region,
            timezone: raw.timezone,
            new_release_days: raw.new_release_days,
            refresh_interval_hours: raw.refresh_interval_hours,
            providers: raw.providers,
            sources: raw.sources,
        };
        // Disable live HTTP sources for deterministic offline route tests
        config.sources.tvmaze.enabled = false;
        config.sources.tmdb.enabled = false;
        config.sources.omdb.enabled = false;
        config.sources.watchmode.enabled = false;

        let state = AppState {
            db,
            config,
            is_syncing: Arc::new(AtomicBool::new(false)),
        };
        let router = build_router(state.clone());
        (router, state, tmp)
    }

    async fn test_app() -> Router {
        test_app_with_state().await.0
    }

    async fn seed_programs(db: &Database) {
        let today = chrono::Utc::now().date_naive();
        let d1 = (today - chrono::Duration::days(1))
            .format("%Y-%m-%d")
            .to_string();
        let d2 = (today - chrono::Duration::days(2))
            .format("%Y-%m-%d")
            .to_string();
        let d3 = (today - chrono::Duration::days(3))
            .format("%Y-%m-%d")
            .to_string();

        sqlx::query(
            r#"
            INSERT INTO programs (
                id, dedup_key, title, media_type, release_kind, release_date, release_year,
                episode_info, synopsis, poster_url, backdrop_url, genres_json, providers_json,
                primary_url, rating, ratings_json, sources_json, imdb_id, tmdb_id, tvmaze_id, updated_at
            ) VALUES
            ('p1', 'k1', 'Alpha Movie', 'movie', 'new_title', ?, 2026, NULL, 'Space adventure', NULL, NULL, '["Sci-Fi","Action"]', '[{"id":"netflix","name":"Netflix","url":"https://netflix.com/1","is_direct":true}]', 'https://netflix.com/1', 8.8, '[]', '["TMDB"]', 'tt1', 1, NULL, '2026-11-10T00:00:00Z'),
            ('p2', 'k2', 'Beta Series', 'series', 'new_episode', ?, 2026, 'S02E01', 'Funny comedy', NULL, NULL, '["Comedy"]', '[{"id":"hulu","name":"Hulu","url":"https://hulu.com/2","is_direct":false}]', 'https://hulu.com/2', 7.2, '[]', '["TVmaze"]', 'tt2', NULL, 2, '2026-11-09T00:00:00Z'),
            ('p3', 'k3', 'Gamma Drama', 'series', 'new_title', ?, 2026, NULL, 'Deep mystery', NULL, NULL, '["Drama"]', '[{"id":"max","name":"Max","url":"https://max.com/3","is_direct":false}]', 'https://max.com/3', 9.1, '[]', '["TMDB"]', 'tt3', 3, NULL, '2026-11-08T00:00:00Z')
            "#,
        )
        .bind(&d1)
        .bind(&d2)
        .bind(&d3)
        .execute(&db.pool)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn health_endpoint_returns_ok() {
        let app = test_app().await;
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn public_config_returns_providers_and_region() {
        let (app, _state, tmp) = test_app_with_state().await;
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/config/public")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        let cfg: PublicConfig = serde_json::from_slice(&body).unwrap();
        assert_eq!(cfg.region, "US");
        assert!(!cfg.providers.is_empty());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[tokio::test]
    async fn patch_profile_validates_region_and_days() {
        let app = test_app().await;
        let bad_region = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/profiles/default")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"region":"USA","new_release_days":14}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(bad_region.status(), StatusCode::BAD_REQUEST);

        let bad_days = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/profiles/default")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"region":"US","new_release_days":0}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(bad_days.status(), StatusCode::BAD_REQUEST);

        let bad_days_high = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/profiles/default")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"new_release_days":366}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(bad_days_high.status(), StatusCode::BAD_REQUEST);

        let not_found = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/profiles/missing")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"region":"","new_release_days":14}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(not_found.status(), StatusCode::NOT_FOUND);

        let ok = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/profiles/default")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"region":"gb","new_release_days":14}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(ok.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn profile_lifecycle_and_validation() {
        let (app, _state, tmp) = test_app_with_state().await;

        // List profiles
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/profiles")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // Invalid empty name
        let bad_empty = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/profiles")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"name":"   "}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(bad_empty.status(), StatusCode::BAD_REQUEST);

        // Invalid >40 char name -> 400
        let too_long = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/profiles")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(r#"{{"name":"{}"}}"#, "a".repeat(41))))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(too_long.status(), StatusCode::BAD_REQUEST);

        // Create profile
        let created_res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/profiles")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"name":"Bob"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(created_res.status(), StatusCode::CREATED);
        let bytes = axum::body::to_bytes(created_res.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: crate::models::Profile = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(created.name, "Bob");

        // Duplicate name -> 409 CONFLICT
        let dup_res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/profiles")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"name":"Bob"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(dup_res.status(), StatusCode::CONFLICT);

        // Delete default -> 400
        let del_def = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/profiles/default")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(del_def.status(), StatusCode::BAD_REQUEST);

        // Delete nonexistent -> 404
        let del_missing = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/profiles/nonexistent")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(del_missing.status(), StatusCode::NOT_FOUND);

        // Delete created -> 204
        let del_ok = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/profiles/{}", created.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(del_ok.status(), StatusCode::NO_CONTENT);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[tokio::test]
    async fn interactions_hidden_genres_and_programs_filters() {
        let (app, state, tmp) = test_app_with_state().await;
        seed_programs(&state.db).await;

        // Interaction: mark p1 as seen + watchlist, p2 as watched, p3 as not_interested
        let r1 = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/profiles/default/interactions/p1")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"seen":true,"watchlist":true}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r1.status(), StatusCode::NO_CONTENT);

        let r2 = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/profiles/default/interactions/p2")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"watched":true}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r2.status(), StatusCode::NO_CONTENT);

        let r3 = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/profiles/default/interactions/p3")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"not_interested":true}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r3.status(), StatusCode::NO_CONTENT);

        // Empty genre -> 400
        let bad_genre = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/profiles/default/hidden-genres")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"genre":"  "}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(bad_genre.status(), StatusCode::BAD_REQUEST);

        // Hide Comedy
        let hide_comedy = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/profiles/default/hidden-genres")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"genre":"Comedy"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(hide_comedy.status(), StatusCode::NO_CONTENT);

        // GET hidden state
        let hidden_res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/profiles/default/hidden")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(hidden_res.status(), StatusCode::OK);
        let hidden_bytes = axum::body::to_bytes(hidden_res.into_body(), usize::MAX)
            .await
            .unwrap();
        let hidden_state: crate::models::ProfileHiddenState =
            serde_json::from_slice(&hidden_bytes).unwrap();
        assert_eq!(hidden_state.hidden_titles.len(), 1);
        assert_eq!(hidden_state.hidden_genres, vec!["Comedy"]);

        // GET /api/genres excludes Comedy
        let genres_res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/genres?profile_id=default")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(genres_res.status(), StatusCode::OK);
        let genres_bytes = axum::body::to_bytes(genres_res.into_body(), usize::MAX)
            .await
            .unwrap();
        let genres: Vec<String> = serde_json::from_slice(&genres_bytes).unwrap();
        assert!(!genres.contains(&"Comedy".to_string()));

        // Unhide Comedy
        let unhide = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/profiles/default/hidden-genres/Comedy")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(unhide.status(), StatusCode::NO_CONTENT);

        // Filter programs: default excludes p3 (not_interested)
        let list_res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/programs?q=adventure&provider=netflix&media_type=movie&release_kind=new_title&genre=sci-fi&status=watchlist&min_rating=8.0&sort=rating&order=asc")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(list_res.status(), StatusCode::OK);
        let list_bytes = axum::body::to_bytes(list_res.into_body(), usize::MAX)
            .await
            .unwrap();
        let list: Vec<crate::models::ProgramDto> = serde_json::from_slice(&list_bytes).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "p1");

        // include_hidden=true + sort=title&order=asc + status=watched
        let watched_res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/programs?include_hidden=true&status=watched&sort=title&order=asc")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(watched_res.status(), StatusCode::OK);
        let watched_bytes = axum::body::to_bytes(watched_res.into_body(), usize::MAX)
            .await
            .unwrap();
        let watched: Vec<crate::models::ProgramDto> =
            serde_json::from_slice(&watched_bytes).unwrap();
        assert_eq!(watched.len(), 1);
        assert_eq!(watched[0].id, "p2");

        // status=unseen & status=seen
        let unseen_res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/programs?include_hidden=true&status=unseen")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(unseen_res.status(), StatusCode::OK);
        let seen_res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/programs?status=seen")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(seen_res.status(), StatusCode::OK);

        // Non-matching filters & default sort (release_date desc)
        let miss_q = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/programs?q=zzzz_nonexistent")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(miss_q.status(), StatusCode::OK);

        let miss_filters = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/programs?provider=peacock&media_type=movie&release_kind=new_episode&genre=horror&status=watchlist&min_rating=9.9")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(miss_filters.status(), StatusCode::OK);

        let default_sort = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/programs?include_hidden=true&provider=all&media_type=all&release_kind=all&genre=all")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(default_sort.status(), StatusCode::OK);

        // GET /api/programs/:id (found & 404)
        let get_one = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/programs/p1?profile_id=default")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(get_one.status(), StatusCode::OK);

        let get_missing = app
            .oneshot(
                Request::builder()
                    .uri("/api/programs/missing")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(get_missing.status(), StatusCode::NOT_FOUND);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[tokio::test]
    async fn sync_status_and_refresh_routes() {
        let (app, state, tmp) = test_app_with_state().await;

        // Status with profile_id
        let status_res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/sync/status?profile_id=default")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(status_res.status(), StatusCode::OK);

        // When already syncing -> already_running: true
        state
            .is_syncing
            .store(true, std::sync::atomic::Ordering::SeqCst);
        let busy_res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/sync/refresh")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(busy_res.status(), StatusCode::OK);
        let busy_bytes = axum::body::to_bytes(busy_res.into_body(), usize::MAX)
            .await
            .unwrap();
        let busy_json: serde_json::Value = serde_json::from_slice(&busy_bytes).unwrap();
        assert_eq!(busy_json["started"], false);
        assert_eq!(busy_json["already_running"], true);

        // Reset flag and trigger actual offline refresh (all sources disabled)
        state
            .is_syncing
            .store(false, std::sync::atomic::Ordering::SeqCst);
        let start_res = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/sync/refresh?profile_id=default")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(start_res.status(), StatusCode::OK);
        let start_bytes = axum::body::to_bytes(start_res.into_body(), usize::MAX)
            .await
            .unwrap();
        let start_json: serde_json::Value = serde_json::from_slice(&start_bytes).unwrap();
        assert_eq!(start_json["started"], true);
        assert_eq!(start_json["already_running"], false);

        // Yield briefly so spawned task completes cleanly before tmp cleanup
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let _ = std::fs::remove_dir_all(&tmp);
    }
}

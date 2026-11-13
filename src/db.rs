use anyhow::Result;
use chrono::Utc;
use sqlx::{sqlite::SqliteConnectOptions, sqlite::SqlitePoolOptions, Row, SqlitePool};
use std::path::Path;
use std::str::FromStr;

use crate::models::{
    HiddenItem, MediaType, Profile, ProfileHiddenState, ProgramDto, ProviderLink, ReleaseKind,
    SourceRating, SourceStatus,
};

#[derive(Clone)]
pub struct Database {
    pub pool: SqlitePool,
}

impl Database {
    pub async fn connect_and_init(data_dir: &str) -> Result<Self> {
        std::fs::create_dir_all(data_dir)?;
        let db_path = Path::new(data_dir).join("channelzero.db");
        let db_url = format!("sqlite://{}", db_path.to_string_lossy());

        let options = SqliteConnectOptions::from_str(&db_url)?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .foreign_keys(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;

        let db = Self { pool };
        db.migrate().await?;
        db.ensure_default_profile().await?;
        Ok(db)
    }

    async fn migrate(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS programs (
                id TEXT PRIMARY KEY,
                dedup_key TEXT UNIQUE NOT NULL,
                title TEXT NOT NULL,
                media_type TEXT NOT NULL,
                release_kind TEXT NOT NULL,
                release_date TEXT NOT NULL,
                release_year INTEGER,
                episode_info TEXT,
                synopsis TEXT,
                poster_url TEXT,
                backdrop_url TEXT,
                genres_json TEXT NOT NULL,
                providers_json TEXT NOT NULL,
                primary_url TEXT NOT NULL,
                rating REAL,
                ratings_json TEXT NOT NULL,
                sources_json TEXT NOT NULL,
                imdb_id TEXT,
                tmdb_id INTEGER,
                tvmaze_id INTEGER,
                updated_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_programs_release_date ON programs(release_date DESC);
            CREATE INDEX IF NOT EXISTS idx_programs_imdb ON programs(imdb_id);
            CREATE INDEX IF NOT EXISTS idx_programs_tmdb ON programs(tmdb_id);
            CREATE INDEX IF NOT EXISTS idx_programs_tvmaze ON programs(tvmaze_id);

            CREATE TABLE IF NOT EXISTS profiles (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                region TEXT,
                new_release_days INTEGER,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS profile_interactions (
                profile_id TEXT NOT NULL,
                program_id TEXT NOT NULL,
                seen INTEGER NOT NULL DEFAULT 0,
                watched INTEGER NOT NULL DEFAULT 0,
                watchlist INTEGER NOT NULL DEFAULT 0,
                not_interested INTEGER NOT NULL DEFAULT 0,
                updated_at TEXT NOT NULL,
                PRIMARY KEY (profile_id, program_id),
                FOREIGN KEY (profile_id) REFERENCES profiles(id) ON DELETE CASCADE,
                FOREIGN KEY (program_id) REFERENCES programs(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS profile_hidden_genres (
                profile_id TEXT NOT NULL,
                genre TEXT NOT NULL,
                created_at TEXT NOT NULL,
                PRIMARY KEY (profile_id, genre),
                FOREIGN KEY (profile_id) REFERENCES profiles(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS source_status (
                name TEXT PRIMARY KEY,
                tier TEXT NOT NULL,
                enabled INTEGER NOT NULL,
                configured INTEGER NOT NULL,
                last_sync_at TEXT,
                last_status TEXT NOT NULL,
                last_error TEXT,
                items_fetched INTEGER NOT NULL DEFAULT 0
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Safe migrations for existing databases
        let _ = sqlx::query("ALTER TABLE profiles ADD COLUMN region TEXT")
            .execute(&self.pool)
            .await;
        let _ = sqlx::query("ALTER TABLE profiles ADD COLUMN new_release_days INTEGER")
            .execute(&self.pool)
            .await;

        Ok(())
    }

    async fn ensure_default_profile(&self) -> Result<()> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM profiles")
            .fetch_one(&self.pool)
            .await?;

        if count.0 == 0 {
            let now = Utc::now().to_rfc3339();
            sqlx::query("INSERT INTO profiles (id, name, created_at) VALUES (?, ?, ?)")
                .bind("default")
                .bind("Default")
                .bind(now)
                .execute(&self.pool)
                .await?;
        }
        Ok(())
    }

    pub async fn list_profiles(&self) -> Result<Vec<Profile>> {
        let rows = sqlx::query(
            "SELECT id, name, region, new_release_days, created_at FROM profiles ORDER BY created_at ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| Profile {
                id: r.get("id"),
                name: r.get("name"),
                region: r.get("region"),
                new_release_days: r
                    .get::<Option<i64>, _>("new_release_days")
                    .map(|v| v as u32),
                created_at: r.get("created_at"),
            })
            .collect())
    }

    pub async fn get_profile(&self, profile_id: &str) -> Result<Option<Profile>> {
        let row = sqlx::query(
            "SELECT id, name, region, new_release_days, created_at FROM profiles WHERE id = ?",
        )
        .bind(profile_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| Profile {
            id: r.get("id"),
            name: r.get("name"),
            region: r.get("region"),
            new_release_days: r
                .get::<Option<i64>, _>("new_release_days")
                .map(|v| v as u32),
            created_at: r.get("created_at"),
        }))
    }

    pub async fn max_profile_new_release_days(&self) -> Result<Option<u32>> {
        let row = sqlx::query("SELECT MAX(new_release_days) AS max_days FROM profiles")
            .fetch_one(&self.pool)
            .await?;

        Ok(row
            .get::<Option<i64>, _>("max_days")
            .filter(|&v| v > 0)
            .map(|v| v as u32))
    }

    pub async fn update_profile_settings(
        &self,
        profile_id: &str,
        region: Option<String>,
        new_release_days: Option<u32>,
    ) -> Result<Option<Profile>> {
        let res = sqlx::query("UPDATE profiles SET region = ?, new_release_days = ? WHERE id = ?")
            .bind(region)
            .bind(new_release_days.map(|v| v as i64))
            .bind(profile_id)
            .execute(&self.pool)
            .await?;

        if res.rows_affected() == 0 {
            return Ok(None);
        }
        self.get_profile(profile_id).await
    }

    pub async fn create_profile(&self, name: &str) -> Result<Profile> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        sqlx::query("INSERT INTO profiles (id, name, created_at) VALUES (?, ?, ?)")
            .bind(&id)
            .bind(name.trim())
            .bind(&now)
            .execute(&self.pool)
            .await?;
        Ok(Profile {
            id,
            name: name.trim().to_string(),
            region: None,
            new_release_days: None,
            created_at: now,
        })
    }

    pub async fn delete_profile(&self, profile_id: &str) -> Result<bool> {
        if profile_id == "default" {
            return Ok(false);
        }
        let res = sqlx::query("DELETE FROM profiles WHERE id = ?")
            .bind(profile_id)
            .execute(&self.pool)
            .await?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn upsert_interaction(
        &self,
        profile_id: &str,
        program_id: &str,
        seen: Option<bool>,
        watched: Option<bool>,
        watchlist: Option<bool>,
        not_interested: Option<bool>,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();

        // Fetch existing if any
        let existing = sqlx::query(
            "SELECT seen, watched, watchlist, not_interested FROM profile_interactions WHERE profile_id = ? AND program_id = ?",
        )
        .bind(profile_id)
        .bind(program_id)
        .fetch_optional(&self.pool)
        .await?;

        let (cur_seen, cur_watched, cur_watchlist, cur_ni) = match existing {
            Some(row) => (
                row.get::<i64, _>("seen") != 0,
                row.get::<i64, _>("watched") != 0,
                row.get::<i64, _>("watchlist") != 0,
                row.get::<i64, _>("not_interested") != 0,
            ),
            None => (false, false, false, false),
        };

        let new_seen = seen.unwrap_or(cur_seen);
        let new_watched = watched.unwrap_or(cur_watched);
        let new_watchlist = watchlist.unwrap_or(cur_watchlist);
        let new_ni = not_interested.unwrap_or(cur_ni);

        sqlx::query(
            r#"
            INSERT INTO profile_interactions (profile_id, program_id, seen, watched, watchlist, not_interested, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(profile_id, program_id) DO UPDATE SET
                seen = excluded.seen,
                watched = excluded.watched,
                watchlist = excluded.watchlist,
                not_interested = excluded.not_interested,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(profile_id)
        .bind(program_id)
        .bind(if new_seen { 1 } else { 0 })
        .bind(if new_watched { 1 } else { 0 })
        .bind(if new_watchlist { 1 } else { 0 })
        .bind(if new_ni { 1 } else { 0 })
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn hide_genre(&self, profile_id: &str, genre: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT OR IGNORE INTO profile_hidden_genres (profile_id, genre, created_at) VALUES (?, ?, ?)",
        )
        .bind(profile_id)
        .bind(genre.trim())
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn unhide_genre(&self, profile_id: &str, genre: &str) -> Result<()> {
        sqlx::query(
            "DELETE FROM profile_hidden_genres WHERE profile_id = ? AND LOWER(genre) = LOWER(?)",
        )
        .bind(profile_id)
        .bind(genre.trim())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_hidden_state(&self, profile_id: &str) -> Result<ProfileHiddenState> {
        let title_rows = sqlx::query(
            r#"
            SELECT p.id, p.title, p.media_type, p.poster_url, pi.updated_at
            FROM profile_interactions pi
            JOIN programs p ON p.id = pi.program_id
            WHERE pi.profile_id = ? AND pi.not_interested = 1
            ORDER BY pi.updated_at DESC
            "#,
        )
        .bind(profile_id)
        .fetch_all(&self.pool)
        .await?;

        let hidden_titles = title_rows
            .into_iter()
            .map(|r| HiddenItem {
                program_id: r.get("id"),
                title: r.get("title"),
                media_type: r.get("media_type"),
                poster_url: r.get("poster_url"),
                updated_at: r.get("updated_at"),
            })
            .collect();

        let genre_rows = sqlx::query(
            "SELECT genre FROM profile_hidden_genres WHERE profile_id = ? ORDER BY genre ASC",
        )
        .bind(profile_id)
        .fetch_all(&self.pool)
        .await?;

        let hidden_genres = genre_rows
            .into_iter()
            .map(|r| r.get::<String, _>("genre"))
            .collect();

        Ok(ProfileHiddenState {
            hidden_titles,
            hidden_genres,
        })
    }

    pub async fn upsert_source_status(&self, status: &SourceStatus) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO source_status (name, tier, enabled, configured, last_sync_at, last_status, last_error, items_fetched)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(name) DO UPDATE SET
                tier = excluded.tier,
                enabled = excluded.enabled,
                configured = excluded.configured,
                last_sync_at = excluded.last_sync_at,
                last_status = excluded.last_status,
                last_error = excluded.last_error,
                items_fetched = excluded.items_fetched
            "#,
        )
        .bind(&status.name)
        .bind(&status.tier)
        .bind(if status.enabled { 1 } else { 0 })
        .bind(if status.configured { 1 } else { 0 })
        .bind(&status.last_sync_at)
        .bind(&status.last_status)
        .bind(&status.last_error)
        .bind(status.items_fetched)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_source_statuses(&self) -> Result<Vec<SourceStatus>> {
        let rows = sqlx::query(
            "SELECT name, tier, enabled, configured, last_sync_at, last_status, last_error, items_fetched FROM source_status ORDER BY name ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| SourceStatus {
                name: r.get("name"),
                tier: r.get("tier"),
                enabled: r.get::<i64, _>("enabled") != 0,
                configured: r.get::<i64, _>("configured") != 0,
                last_sync_at: r.get("last_sync_at"),
                last_status: r.get("last_status"),
                last_error: r.get("last_error"),
                items_fetched: r.get("items_fetched"),
            })
            .collect())
    }

    pub async fn count_programs(&self) -> Result<i64> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM programs")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.0)
    }

    pub async fn get_programs_for_profile(
        &self,
        profile_id: &str,
        min_release_date: &str,
    ) -> Result<Vec<ProgramDto>> {
        let rows = sqlx::query(
            r#"
            SELECT
                p.id, p.title, p.media_type, p.release_kind, p.release_date, p.release_year,
                p.episode_info, p.synopsis, p.poster_url, p.backdrop_url,
                p.genres_json, p.providers_json, p.primary_url, p.rating, p.ratings_json,
                p.sources_json, p.imdb_id, p.tmdb_id, p.tvmaze_id,
                COALESCE(pi.seen, 0) AS seen,
                COALESCE(pi.watched, 0) AS watched,
                COALESCE(pi.watchlist, 0) AS watchlist,
                COALESCE(pi.not_interested, 0) AS not_interested,
                pi.updated_at AS last_interacted_at
            FROM programs p
            LEFT JOIN profile_interactions pi
                ON pi.program_id = p.id AND pi.profile_id = ?
            WHERE p.release_date >= ?
            ORDER BY p.release_date DESC
            "#,
        )
        .bind(profile_id)
        .bind(min_release_date)
        .fetch_all(&self.pool)
        .await?;

        let mut results = Vec::with_capacity(rows.len());
        for r in rows {
            let genres: Vec<String> =
                serde_json::from_str(r.get::<&str, _>("genres_json")).unwrap_or_default();
            let providers: Vec<ProviderLink> =
                serde_json::from_str(r.get::<&str, _>("providers_json")).unwrap_or_default();
            let ratings: Vec<SourceRating> =
                serde_json::from_str(r.get::<&str, _>("ratings_json")).unwrap_or_default();
            let sources: Vec<String> =
                serde_json::from_str(r.get::<&str, _>("sources_json")).unwrap_or_default();

            results.push(ProgramDto {
                id: r.get("id"),
                title: r.get("title"),
                media_type: MediaType::from_str_lossy(r.get("media_type")),
                release_kind: ReleaseKind::from_str_lossy(r.get("release_kind")),
                release_date: r.get("release_date"),
                release_year: r.get("release_year"),
                episode_info: r.get("episode_info"),
                synopsis: r.get("synopsis"),
                poster_url: r.get("poster_url"),
                backdrop_url: r.get("backdrop_url"),
                genres,
                providers,
                primary_url: r.get("primary_url"),
                rating: r.get("rating"),
                ratings,
                sources,
                imdb_id: r.get("imdb_id"),
                tmdb_id: r.get("tmdb_id"),
                tvmaze_id: r.get("tvmaze_id"),
                seen: r.get::<i64, _>("seen") != 0,
                watched: r.get::<i64, _>("watched") != 0,
                watchlist: r.get::<i64, _>("watchlist") != 0,
                not_interested: r.get::<i64, _>("not_interested") != 0,
                last_interacted_at: r.get("last_interacted_at"),
            });
        }

        Ok(results)
    }

    pub async fn get_program_by_id(
        &self,
        profile_id: &str,
        id: &str,
    ) -> Result<Option<ProgramDto>> {
        let row = sqlx::query(
            r#"
            SELECT
                p.id, p.title, p.media_type, p.release_kind, p.release_date, p.release_year,
                p.episode_info, p.synopsis, p.poster_url, p.backdrop_url,
                p.genres_json, p.providers_json, p.primary_url, p.rating, p.ratings_json,
                p.sources_json, p.imdb_id, p.tmdb_id, p.tvmaze_id,
                COALESCE(pi.seen, 0) AS seen,
                COALESCE(pi.watched, 0) AS watched,
                COALESCE(pi.watchlist, 0) AS watchlist,
                COALESCE(pi.not_interested, 0) AS not_interested,
                pi.updated_at AS last_interacted_at
            FROM programs p
            LEFT JOIN profile_interactions pi
                ON pi.program_id = p.id AND pi.profile_id = ?
            WHERE p.id = ?
            "#,
        )
        .bind(profile_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| ProgramDto {
            id: r.get("id"),
            title: r.get("title"),
            media_type: MediaType::from_str_lossy(r.get("media_type")),
            release_kind: ReleaseKind::from_str_lossy(r.get("release_kind")),
            release_date: r.get("release_date"),
            release_year: r.get("release_year"),
            episode_info: r.get("episode_info"),
            synopsis: r.get("synopsis"),
            poster_url: r.get("poster_url"),
            backdrop_url: r.get("backdrop_url"),
            genres: serde_json::from_str(r.get::<&str, _>("genres_json")).unwrap_or_default(),
            providers: serde_json::from_str(r.get::<&str, _>("providers_json")).unwrap_or_default(),
            primary_url: r.get("primary_url"),
            rating: r.get("rating"),
            ratings: serde_json::from_str(r.get::<&str, _>("ratings_json")).unwrap_or_default(),
            sources: serde_json::from_str(r.get::<&str, _>("sources_json")).unwrap_or_default(),
            imdb_id: r.get("imdb_id"),
            tmdb_id: r.get("tmdb_id"),
            tvmaze_id: r.get("tvmaze_id"),
            seen: r.get::<i64, _>("seen") != 0,
            watched: r.get::<i64, _>("watched") != 0,
            watchlist: r.get::<i64, _>("watchlist") != 0,
            not_interested: r.get::<i64, _>("not_interested") != 0,
            last_interacted_at: r.get("last_interacted_at"),
        }))
    }

    pub async fn get_available_genres(
        &self,
        profile_id: &str,
        min_release_date: &str,
    ) -> Result<Vec<String>> {
        let rows = sqlx::query("SELECT genres_json FROM programs WHERE release_date >= ?")
            .bind(min_release_date)
            .fetch_all(&self.pool)
            .await?;

        let hidden = self.get_hidden_state(profile_id).await?;
        let hidden_lower: std::collections::HashSet<String> = hidden
            .hidden_genres
            .into_iter()
            .map(|g| g.to_lowercase())
            .collect();

        let mut set = std::collections::BTreeSet::new();
        for r in rows {
            let genres: Vec<String> =
                serde_json::from_str(r.get::<&str, _>("genres_json")).unwrap_or_default();
            for g in genres {
                if !hidden_lower.contains(&g.to_lowercase()) {
                    set.insert(g);
                }
            }
        }

        Ok(set.into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_db() -> Database {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let db = Database { pool };
        db.migrate().await.unwrap();
        db.ensure_default_profile().await.unwrap();
        db
    }

    #[tokio::test]
    async fn profile_settings_update_and_fallback() {
        let db = test_db().await;
        let def = db.get_profile("default").await.unwrap().unwrap();
        assert_eq!(def.region, None);
        assert_eq!(def.new_release_days, None);
        assert_eq!(db.max_profile_new_release_days().await.unwrap(), None);

        let updated = db
            .update_profile_settings("default", Some("GB".into()), Some(14))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.region.as_deref(), Some("GB"));
        assert_eq!(updated.new_release_days, Some(14));
        assert_eq!(db.max_profile_new_release_days().await.unwrap(), Some(14));

        let second = db.create_profile("Kid").await.unwrap();
        db.update_profile_settings(&second.id, None, Some(180))
            .await
            .unwrap();
        assert_eq!(db.max_profile_new_release_days().await.unwrap(), Some(180));

        db.delete_profile(&second.id).await.unwrap();
        assert_eq!(db.max_profile_new_release_days().await.unwrap(), Some(14));

        let reset = db
            .update_profile_settings("default", None, None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(reset.region, None);
        assert_eq!(reset.new_release_days, None);
    }

    #[tokio::test]
    async fn available_genres_excludes_hidden() {
        let db = test_db().await;
        sqlx::query(
            r#"
            INSERT INTO programs (
                id, dedup_key, title, media_type, release_kind, release_date,
                genres_json, providers_json, primary_url, ratings_json, sources_json, updated_at
            ) VALUES
            ('p1', 'k1', 'Show A', 'series', 'new_title', '2026-11-10', '["Drama","Sci-Fi"]', '[]', 'https://example.com', '[]', '["TVmaze"]', '2026-11-10T00:00:00Z'),
            ('p2', 'k2', 'Show B', 'series', 'new_title', '2026-11-11', '["Comedy","Drama"]', '[]', 'https://example.com', '[]', '["TVmaze"]', '2026-11-11T00:00:00Z')
            "#,
        )
        .execute(&db.pool)
        .await
        .unwrap();

        let genres = db
            .get_available_genres("default", "2026-10-01")
            .await
            .unwrap();
        assert_eq!(genres, vec!["Comedy", "Drama", "Sci-Fi"]);

        db.hide_genre("default", "Drama").await.unwrap();
        let filtered = db
            .get_available_genres("default", "2026-10-01")
            .await
            .unwrap();
        assert_eq!(filtered, vec!["Comedy", "Sci-Fi"]);

        db.unhide_genre("default", "drama").await.unwrap();
        let restored = db
            .get_available_genres("default", "2026-10-01")
            .await
            .unwrap();
        assert_eq!(restored, vec!["Comedy", "Drama", "Sci-Fi"]);
    }

    #[tokio::test]
    async fn connect_and_init_and_profile_crud() {
        let tmp = std::env::temp_dir().join(format!("cz_db_{}", uuid::Uuid::new_v4()));
        let db = Database::connect_and_init(tmp.to_str().unwrap())
            .await
            .unwrap();

        let profiles = db.list_profiles().await.unwrap();
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].id, "default");

        let created = db.create_profile("  Alice  ").await.unwrap();
        assert_eq!(created.name, "Alice");
        assert_eq!(db.list_profiles().await.unwrap().len(), 2);

        // Cannot delete default profile
        assert!(!db.delete_profile("default").await.unwrap());
        // Deleting unknown profile returns false
        assert!(!db.delete_profile("nonexistent").await.unwrap());
        // Deleting created profile returns true
        assert!(db.delete_profile(&created.id).await.unwrap());
        assert_eq!(db.list_profiles().await.unwrap().len(), 1);

        // Updating nonexistent profile returns None
        assert!(db
            .update_profile_settings("nonexistent", Some("US".into()), Some(10))
            .await
            .unwrap()
            .is_none());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[tokio::test]
    async fn interactions_hidden_state_and_programs_queries() {
        let db = test_db().await;
        assert_eq!(db.count_programs().await.unwrap(), 0);

        sqlx::query(
            r#"
            INSERT INTO programs (
                id, dedup_key, title, media_type, release_kind, release_date, release_year,
                episode_info, synopsis, poster_url, backdrop_url, genres_json, providers_json,
                primary_url, rating, ratings_json, sources_json, imdb_id, tmdb_id, tvmaze_id, updated_at
            ) VALUES
            ('p1', 'k1', 'Movie One', 'movie', 'new_title', '2026-11-10', 2026, NULL, 'Great movie', 'https://img/p1.jpg', NULL, '["Action"]', '[{"id":"netflix","name":"Netflix","url":"https://netflix.com","is_direct":false}]', 'https://netflix.com', 8.5, '[{"source":"TMDB","score":"8.5/10","normalized_10":8.5}]', '["TMDB"]', 'tt1234567', 100, NULL, '2026-11-10T00:00:00Z'),
            ('p2', 'k2', 'Old Show', 'series', 'new_episode', '2025-01-01', 2025, 'S01E01', NULL, NULL, NULL, '["Drama"]', '[]', 'https://example.com', NULL, '[]', '["TVmaze"]', NULL, NULL, 200, '2025-01-01T00:00:00Z')
            "#,
        )
        .execute(&db.pool)
        .await
        .unwrap();

        assert_eq!(db.count_programs().await.unwrap(), 2);

        // Partial interaction updates preserve previous flags
        db.upsert_interaction("default", "p1", Some(true), None, None, None)
            .await
            .unwrap();
        db.upsert_interaction("default", "p1", None, Some(true), Some(true), Some(true))
            .await
            .unwrap();

        let p1 = db
            .get_program_by_id("default", "p1")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(p1.title, "Movie One");
        assert_eq!(p1.media_type, MediaType::Movie);
        assert!(p1.seen);
        assert!(p1.watched);
        assert!(p1.watchlist);
        assert!(p1.not_interested);
        assert!(p1.last_interacted_at.is_some());

        assert!(db
            .get_program_by_id("default", "missing")
            .await
            .unwrap()
            .is_none());

        let recent = db
            .get_programs_for_profile("default", "2026-01-01")
            .await
            .unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].id, "p1");

        db.hide_genre("default", "Action").await.unwrap();
        let hidden = db.get_hidden_state("default").await.unwrap();
        assert_eq!(hidden.hidden_titles.len(), 1);
        assert_eq!(hidden.hidden_titles[0].program_id, "p1");
        assert_eq!(hidden.hidden_genres, vec!["Action"]);
    }

    #[tokio::test]
    async fn source_status_upsert_and_list() {
        let db = test_db().await;
        let status = SourceStatus {
            name: "TVmaze".into(),
            tier: "Anonymous / Free".into(),
            enabled: true,
            configured: true,
            last_sync_at: Some("2026-11-13T00:00:00Z".into()),
            last_status: "ok".into(),
            last_error: None,
            items_fetched: 42,
        };
        db.upsert_source_status(&status).await.unwrap();

        let updated = SourceStatus {
            items_fetched: 99,
            last_status: "error".into(),
            last_error: Some("rate limited".into()),
            ..status
        };
        db.upsert_source_status(&updated).await.unwrap();

        let list = db.get_source_statuses().await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].items_fetched, 99);
        assert_eq!(list[0].last_status, "error");
        assert_eq!(list[0].last_error.as_deref(), Some("rate limited"));
    }
}

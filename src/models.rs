use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MediaType {
    Movie,
    Series,
}

impl MediaType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MediaType::Movie => "movie",
            MediaType::Series => "series",
        }
    }

    pub fn from_str_lossy(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "movie" => MediaType::Movie,
            _ => MediaType::Series,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseKind {
    NewTitle,
    NewEpisode,
}

impl ReleaseKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReleaseKind::NewTitle => "new_title",
            ReleaseKind::NewEpisode => "new_episode",
        }
    }

    pub fn from_str_lossy(s: &str) -> Self {
        match s {
            "new_episode" => ReleaseKind::NewEpisode,
            _ => ReleaseKind::NewTitle,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderLink {
    pub id: String,
    pub name: String,
    pub url: String,
    pub is_direct: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceRating {
    pub source: String,
    pub score: String,
    pub normalized_10: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramDto {
    pub id: String,
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
    pub primary_url: String,
    pub rating: Option<f64>,
    pub ratings: Vec<SourceRating>,
    pub sources: Vec<String>,
    pub imdb_id: Option<String>,
    pub tmdb_id: Option<i64>,
    pub tvmaze_id: Option<i64>,
    // Profile-specific state
    pub seen: bool,
    pub watched: bool,
    pub watchlist: bool,
    pub not_interested: bool,
    pub last_interacted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub region: Option<String>,
    pub new_release_days: Option<u32>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HiddenItem {
    pub program_id: String,
    pub title: String,
    pub media_type: String,
    pub poster_url: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileHiddenState {
    pub hidden_titles: Vec<HiddenItem>,
    pub hidden_genres: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceStatus {
    pub name: String,
    pub tier: String,
    pub enabled: bool,
    pub configured: bool,
    pub last_sync_at: Option<String>,
    pub last_status: String,
    pub last_error: Option<String>,
    pub items_fetched: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatusResponse {
    pub last_sync_at: Option<String>,
    pub is_syncing: bool,
    pub total_programs: i64,
    pub refresh_interval_hours: u32,
    pub new_release_days: u32,
    pub region: String,
    pub sources: Vec<SourceStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderMeta {
    pub id: String,
    pub name: String,
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicConfig {
    pub region: String,
    pub timezone: String,
    pub new_release_days: u32,
    pub refresh_interval_hours: u32,
    pub providers: Vec<ProviderMeta>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_type_roundtrip_and_lossy() {
        assert_eq!(MediaType::Movie.as_str(), "movie");
        assert_eq!(MediaType::Series.as_str(), "series");
        assert_eq!(MediaType::from_str_lossy("movie"), MediaType::Movie);
        assert_eq!(MediaType::from_str_lossy("MOVIE"), MediaType::Movie);
        assert_eq!(MediaType::from_str_lossy("series"), MediaType::Series);
        assert_eq!(MediaType::from_str_lossy("unknown"), MediaType::Series);

        let json = serde_json::to_string(&MediaType::Movie).unwrap();
        assert_eq!(json, "\"movie\"");
        let parsed: MediaType = serde_json::from_str("\"series\"").unwrap();
        assert_eq!(parsed, MediaType::Series);
    }

    #[test]
    fn release_kind_roundtrip_and_lossy() {
        assert_eq!(ReleaseKind::NewTitle.as_str(), "new_title");
        assert_eq!(ReleaseKind::NewEpisode.as_str(), "new_episode");
        assert_eq!(
            ReleaseKind::from_str_lossy("new_episode"),
            ReleaseKind::NewEpisode
        );
        assert_eq!(
            ReleaseKind::from_str_lossy("new_title"),
            ReleaseKind::NewTitle
        );
        assert_eq!(ReleaseKind::from_str_lossy("other"), ReleaseKind::NewTitle);

        let json = serde_json::to_string(&ReleaseKind::NewEpisode).unwrap();
        assert_eq!(json, "\"new_episode\"");
        let parsed: ReleaseKind = serde_json::from_str("\"new_title\"").unwrap();
        assert_eq!(parsed, ReleaseKind::NewTitle);
    }
}

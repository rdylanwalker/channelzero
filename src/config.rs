use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use tracing::warn;

use crate::models::ProviderMeta;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleSourceConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Default for SimpleSourceConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KeyedSourceConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SourcesConfig {
    #[serde(default)]
    pub tvmaze: SimpleSourceConfig,
    #[serde(default)]
    pub tmdb: KeyedSourceConfig,
    #[serde(default)]
    pub omdb: KeyedSourceConfig,
    #[serde(default)]
    pub watchmode: KeyedSourceConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawConfig {
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_region")]
    pub region: String,
    #[serde(default = "default_timezone")]
    pub timezone: String,
    #[serde(default = "default_new_release_days")]
    pub new_release_days: u32,
    #[serde(default = "default_refresh_interval_hours")]
    pub refresh_interval_hours: u32,
    #[serde(default = "default_providers")]
    pub providers: Vec<String>,
    #[serde(default)]
    pub sources: SourcesConfig,
}

impl Default for RawConfig {
    fn default() -> Self {
        Self {
            port: default_port(),
            region: default_region(),
            timezone: default_timezone(),
            new_release_days: default_new_release_days(),
            refresh_interval_hours: default_refresh_interval_hours(),
            providers: default_providers(),
            sources: SourcesConfig::default(),
        }
    }
}

fn default_true() -> bool {
    true
}
fn default_port() -> u16 {
    8080
}
fn default_region() -> String {
    "US".to_string()
}
fn default_timezone() -> String {
    "UTC".to_string()
}
fn default_new_release_days() -> u32 {
    30
}
fn default_refresh_interval_hours() -> u32 {
    6
}
fn default_providers() -> Vec<String> {
    vec![
        "netflix".into(),
        "max".into(),
        "disney_plus".into(),
        "prime_video".into(),
        "hulu".into(),
        "apple_tv_plus".into(),
        "peacock".into(),
        "paramount_plus".into(),
    ]
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub port: u16,
    pub data_dir: String,
    pub static_dir: String,
    pub region: String,
    pub timezone: String,
    pub new_release_days: u32,
    pub refresh_interval_hours: u32,
    pub providers: Vec<String>,
    pub sources: SourcesConfig,
}

pub fn supported_providers() -> Vec<ProviderMeta> {
    vec![
        ProviderMeta {
            id: "netflix".into(),
            name: "Netflix".into(),
            color: "#E50914".into(),
        },
        ProviderMeta {
            id: "max".into(),
            name: "Max".into(),
            color: "#002BE7".into(),
        },
        ProviderMeta {
            id: "disney_plus".into(),
            name: "Disney+".into(),
            color: "#113CCF".into(),
        },
        ProviderMeta {
            id: "prime_video".into(),
            name: "Prime Video".into(),
            color: "#00A8E1".into(),
        },
        ProviderMeta {
            id: "hulu".into(),
            name: "Hulu".into(),
            color: "#1CE783".into(),
        },
        ProviderMeta {
            id: "apple_tv_plus".into(),
            name: "Apple TV+".into(),
            color: "#1D1D1F".into(),
        },
        ProviderMeta {
            id: "peacock".into(),
            name: "Peacock".into(),
            color: "#F4A900".into(),
        },
        ProviderMeta {
            id: "paramount_plus".into(),
            name: "Paramount+".into(),
            color: "#0064FF".into(),
        },
    ]
}

pub fn provider_meta_by_id(id: &str) -> Option<ProviderMeta> {
    supported_providers().into_iter().find(|p| p.id == id)
}

/// Map TMDB watch provider IDs (US & major global) to canonical ChannelZero provider IDs.
pub fn tmdb_provider_id_to_slug(tmdb_id: i64) -> Option<&'static str> {
    match tmdb_id {
        8 | 1796 => Some("netflix"),
        1899 | 384 => Some("max"),
        337 => Some("disney_plus"),
        9 | 10 | 119 | 2100 => Some("prime_video"),
        15 => Some("hulu"),
        350 | 2 => Some("apple_tv_plus"),
        386 | 387 => Some("peacock"),
        531 | 1853 | 582 => Some("paramount_plus"),
        _ => None,
    }
}

/// Canonical ChannelZero provider ID to TMDB watch provider IDs for discover filtering.
pub fn slug_to_tmdb_provider_ids(slug: &str) -> &'static [i64] {
    match slug {
        "netflix" => &[8],
        "max" => &[1899, 384],
        "disney_plus" => &[337],
        "prime_video" => &[9, 119],
        "hulu" => &[15],
        "apple_tv_plus" => &[350],
        "peacock" => &[386, 387],
        "paramount_plus" => &[531, 1853],
        _ => &[],
    }
}

/// Map TVmaze network/webChannel names to canonical ChannelZero provider IDs.
pub fn tvmaze_channel_to_slug(name: &str) -> Option<&'static str> {
    let lower = name.to_lowercase();
    if lower.contains("netflix") {
        Some("netflix")
    } else if lower == "hbo" || lower == "max" || lower.contains("hbo max") {
        Some("max")
    } else if lower.contains("disney+") || lower.contains("disney plus") {
        Some("disney_plus")
    } else if lower.contains("prime video") || lower.contains("amazon") {
        Some("prime_video")
    } else if lower.contains("hulu") {
        Some("hulu")
    } else if lower.contains("apple tv") {
        Some("apple_tv_plus")
    } else if lower.contains("peacock") {
        Some("peacock")
    } else if lower.contains("paramount+") || lower.contains("paramount plus") {
        Some("paramount_plus")
    } else {
        None
    }
}

pub fn provider_search_url(slug: &str, title: &str) -> String {
    let encoded: String = title
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            b' ' => "+".to_string(),
            _ => format!("%{:02X}", b),
        })
        .collect();

    match slug {
        "netflix" => format!("https://www.netflix.com/search?q={encoded}"),
        "max" => format!("https://play.max.com/search?q={encoded}"),
        "disney_plus" => format!("https://www.disneyplus.com/search?q={encoded}"),
        "prime_video" => format!("https://www.amazon.com/s?k={encoded}&i=instant-video"),
        "hulu" => format!("https://www.hulu.com/search?q={encoded}"),
        "apple_tv_plus" => format!("https://tv.apple.com/search?term={encoded}"),
        "peacock" => format!("https://www.peacocktv.com/watch/search?q={encoded}"),
        "paramount_plus" => format!("https://www.paramountplus.com/search/?q={encoded}"),
        _ => format!("https://www.themoviedb.org/search?query={encoded}"),
    }
}

fn read_secret_env(var_name: &str) -> Option<String> {
    if let Ok(val) = std::env::var(var_name) {
        let trimmed = val.trim().to_string();
        if !trimmed.is_empty() {
            return Some(trimmed);
        }
    }
    let file_var = format!("{var_name}_FILE");
    if let Ok(path) = std::env::var(&file_var) {
        let trimmed_path = path.trim();
        if !trimmed_path.is_empty() {
            match fs::read_to_string(trimmed_path) {
                Ok(content) => {
                    let secret = content.trim().to_string();
                    if !secret.is_empty() {
                        return Some(secret);
                    }
                }
                Err(e) => {
                    warn!("Failed to read secret file {trimmed_path} from {file_var}: {e}");
                }
            }
        }
    }
    None
}

fn parse_bool_env(var_name: &str) -> Option<bool> {
    std::env::var(var_name)
        .ok()
        .and_then(|v| match v.trim().to_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Some(true),
            "0" | "false" | "no" | "off" => Some(false),
            _ => None,
        })
}

impl AppConfig {
    pub fn load_and_validate() -> Result<Self> {
        let config_path =
            std::env::var("CONFIG_FILE").unwrap_or_else(|_| "/config/config.json".to_string());
        let mut raw = if Path::new(&config_path).exists() {
            let content = fs::read_to_string(&config_path)
                .with_context(|| format!("Failed to read config file at {config_path}"))?;
            serde_json::from_str::<RawConfig>(&content)
                .with_context(|| format!("Invalid JSON in config file {config_path}"))?
        } else {
            RawConfig::default()
        };

        // Environment variable overrides
        if let Ok(p) = std::env::var("PORT") {
            if let Ok(parsed) = p.trim().parse::<u16>() {
                raw.port = parsed;
            }
        }
        if let Ok(r) = std::env::var("REGION") {
            let trimmed = r.trim().to_uppercase();
            if !trimmed.is_empty() {
                raw.region = trimmed;
            }
        }
        if let Ok(tz) = std::env::var("TZ") {
            let trimmed = tz.trim().to_string();
            if !trimmed.is_empty() {
                raw.timezone = trimmed;
            }
        }
        if let Ok(days) = std::env::var("NEW_RELEASE_DAYS") {
            if let Ok(parsed) = days.trim().parse::<u32>() {
                raw.new_release_days = parsed;
            }
        }
        if let Ok(hours) = std::env::var("REFRESH_INTERVAL_HOURS") {
            if let Ok(parsed) = hours.trim().parse::<u32>() {
                raw.refresh_interval_hours = parsed;
            }
        }
        if let Ok(provs) = std::env::var("PROVIDERS") {
            let list: Vec<String> = provs
                .split(',')
                .map(|s| s.trim().to_lowercase())
                .filter(|s| !s.is_empty())
                .collect();
            if !list.is_empty() {
                raw.providers = list;
            }
        }

        // Source overrides & secrets
        if let Some(b) = parse_bool_env("TVMAZE_ENABLED") {
            raw.sources.tvmaze.enabled = b;
        }
        if let Some(key) = read_secret_env("TMDB_API_KEY") {
            raw.sources.tmdb.api_key = Some(key);
            if parse_bool_env("TMDB_ENABLED").is_none() {
                raw.sources.tmdb.enabled = true;
            }
        }
        if let Some(b) = parse_bool_env("TMDB_ENABLED") {
            raw.sources.tmdb.enabled = b;
        }

        if let Some(key) = read_secret_env("OMDB_API_KEY") {
            raw.sources.omdb.api_key = Some(key);
            if parse_bool_env("OMDB_ENABLED").is_none() {
                raw.sources.omdb.enabled = true;
            }
        }
        if let Some(b) = parse_bool_env("OMDB_ENABLED") {
            raw.sources.omdb.enabled = b;
        }

        if let Some(key) = read_secret_env("WATCHMODE_API_KEY") {
            raw.sources.watchmode.api_key = Some(key);
            if parse_bool_env("WATCHMODE_ENABLED").is_none() {
                raw.sources.watchmode.enabled = true;
            }
        }
        if let Some(b) = parse_bool_env("WATCHMODE_ENABLED") {
            raw.sources.watchmode.enabled = b;
        }

        // Validate against schema rules
        Self::validate_raw(&raw)?;

        let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| "/data".to_string());
        let static_dir = std::env::var("STATIC_DIR").unwrap_or_else(|_| "/app/static".to_string());

        Ok(Self {
            port: raw.port,
            data_dir,
            static_dir,
            region: raw.region,
            timezone: raw.timezone,
            new_release_days: raw.new_release_days,
            refresh_interval_hours: raw.refresh_interval_hours,
            providers: raw.providers,
            sources: raw.sources,
        })
    }

    fn validate_raw(raw: &RawConfig) -> Result<()> {
        if raw.port == 0 {
            bail!("Invalid port: must be between 1 and 65535");
        }
        if raw.region.len() != 2 || !raw.region.chars().all(|c| c.is_ascii_uppercase()) {
            bail!(
                "Invalid region '{}': must be a 2-letter uppercase ISO 3166-1 alpha-2 code (e.g. US, CA, GB)",
                raw.region
            );
        }
        if raw.timezone.parse::<chrono_tz::Tz>().is_err() {
            bail!(
                "Invalid timezone '{}': must be a valid IANA timezone (e.g. UTC, America/New_York)",
                raw.timezone
            );
        }
        if !(1..=365).contains(&raw.new_release_days) {
            bail!(
                "Invalid new_release_days {}: must be between 1 and 365",
                raw.new_release_days
            );
        }
        if !(1..=168).contains(&raw.refresh_interval_hours) {
            bail!(
                "Invalid refresh_interval_hours {}: must be between 1 and 168",
                raw.refresh_interval_hours
            );
        }
        if raw.providers.is_empty() {
            bail!("Invalid providers: at least one streaming provider must be configured");
        }

        let valid_ids: HashSet<String> = supported_providers().into_iter().map(|p| p.id).collect();
        let mut seen = HashSet::new();
        for p in &raw.providers {
            if !valid_ids.contains(p) {
                bail!(
                    "Unsupported provider '{}'. Supported providers: {:?}",
                    p,
                    valid_ids
                );
            }
            if !seen.insert(p.clone()) {
                bail!("Duplicate provider '{}' in configuration", p);
            }
        }

        if raw.sources.tmdb.enabled && raw.sources.tmdb.api_key.as_deref().unwrap_or("").is_empty()
        {
            warn!("TMDB is enabled in config, but TMDB_API_KEY is not set; TMDB sync will be skipped until configured.");
        }
        if raw.sources.omdb.enabled && raw.sources.omdb.api_key.as_deref().unwrap_or("").is_empty()
        {
            warn!("OMDb is enabled in config, but OMDB_API_KEY is not set; OMDb enrichment will be skipped until configured.");
        }
        if raw.sources.watchmode.enabled
            && raw
                .sources
                .watchmode
                .api_key
                .as_deref()
                .unwrap_or("")
                .is_empty()
        {
            warn!("Watchmode is enabled in config, but WATCHMODE_API_KEY is not set; Watchmode enrichment will be skipped until configured.");
        }

        Ok(())
    }

    pub fn provider_priority(&self, slug: &str) -> usize {
        self.providers
            .iter()
            .position(|p| p == slug)
            .unwrap_or(usize::MAX)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn clear_test_env() {
        for k in [
            "CONFIG_FILE",
            "PORT",
            "REGION",
            "TZ",
            "NEW_RELEASE_DAYS",
            "REFRESH_INTERVAL_HOURS",
            "PROVIDERS",
            "TVMAZE_ENABLED",
            "TMDB_API_KEY",
            "TMDB_API_KEY_FILE",
            "TMDB_ENABLED",
            "OMDB_API_KEY",
            "OMDB_API_KEY_FILE",
            "OMDB_ENABLED",
            "WATCHMODE_API_KEY",
            "WATCHMODE_API_KEY_FILE",
            "WATCHMODE_ENABLED",
            "DATA_DIR",
            "STATIC_DIR",
        ] {
            std::env::remove_var(k);
        }
    }

    #[test]
    fn default_config_is_valid() {
        let raw = RawConfig::default();
        assert!(AppConfig::validate_raw(&raw).is_ok());
    }

    #[test]
    fn invalid_region_fails_validation() {
        let raw = RawConfig {
            region: "USA".into(),
            ..Default::default()
        };
        assert!(AppConfig::validate_raw(&raw).is_err());

        let lower = RawConfig {
            region: "us".into(),
            ..Default::default()
        };
        assert!(AppConfig::validate_raw(&lower).is_err());
    }

    #[test]
    fn invalid_provider_fails_validation() {
        let raw = RawConfig {
            providers: vec!["unknown_service".into()],
            ..Default::default()
        };
        assert!(AppConfig::validate_raw(&raw).is_err());
    }

    #[test]
    fn validation_catches_port_tz_days_hours_and_duplicates() {
        assert!(AppConfig::validate_raw(&RawConfig {
            port: 0,
            ..Default::default()
        })
        .is_err());

        assert!(AppConfig::validate_raw(&RawConfig {
            timezone: "Mars/Olympus".into(),
            ..Default::default()
        })
        .is_err());

        assert!(AppConfig::validate_raw(&RawConfig {
            new_release_days: 0,
            ..Default::default()
        })
        .is_err());
        assert!(AppConfig::validate_raw(&RawConfig {
            new_release_days: 366,
            ..Default::default()
        })
        .is_err());

        assert!(AppConfig::validate_raw(&RawConfig {
            refresh_interval_hours: 0,
            ..Default::default()
        })
        .is_err());
        assert!(AppConfig::validate_raw(&RawConfig {
            refresh_interval_hours: 169,
            ..Default::default()
        })
        .is_err());

        assert!(AppConfig::validate_raw(&RawConfig {
            providers: vec![],
            ..Default::default()
        })
        .is_err());

        assert!(AppConfig::validate_raw(&RawConfig {
            providers: vec!["netflix".into(), "netflix".into()],
            ..Default::default()
        })
        .is_err());

        // Enabled keyed sources without keys still pass validation (with warnings)
        let mut warn_raw = RawConfig::default();
        warn_raw.sources.tmdb.enabled = true;
        warn_raw.sources.omdb.enabled = true;
        warn_raw.sources.watchmode.enabled = true;
        assert!(AppConfig::validate_raw(&warn_raw).is_ok());
    }

    #[test]
    fn provider_meta_and_mappings() {
        assert_eq!(provider_meta_by_id("netflix").unwrap().name, "Netflix");
        assert!(provider_meta_by_id("nonexistent").is_none());

        assert_eq!(tmdb_provider_id_to_slug(8), Some("netflix"));
        assert_eq!(tmdb_provider_id_to_slug(1899), Some("max"));
        assert_eq!(tmdb_provider_id_to_slug(337), Some("disney_plus"));
        assert_eq!(tmdb_provider_id_to_slug(9), Some("prime_video"));
        assert_eq!(tmdb_provider_id_to_slug(15), Some("hulu"));
        assert_eq!(tmdb_provider_id_to_slug(350), Some("apple_tv_plus"));
        assert_eq!(tmdb_provider_id_to_slug(386), Some("peacock"));
        assert_eq!(tmdb_provider_id_to_slug(531), Some("paramount_plus"));
        assert_eq!(tmdb_provider_id_to_slug(99999), None);

        assert_eq!(slug_to_tmdb_provider_ids("netflix"), &[8]);
        assert_eq!(slug_to_tmdb_provider_ids("max"), &[1899, 384]);
        assert_eq!(slug_to_tmdb_provider_ids("disney_plus"), &[337]);
        assert_eq!(slug_to_tmdb_provider_ids("prime_video"), &[9, 119]);
        assert_eq!(slug_to_tmdb_provider_ids("hulu"), &[15]);
        assert_eq!(slug_to_tmdb_provider_ids("apple_tv_plus"), &[350]);
        assert_eq!(slug_to_tmdb_provider_ids("peacock"), &[386, 387]);
        assert_eq!(slug_to_tmdb_provider_ids("paramount_plus"), &[531, 1853]);
        assert!(slug_to_tmdb_provider_ids("unknown").is_empty());

        assert_eq!(tvmaze_channel_to_slug("Netflix"), Some("netflix"));
        assert_eq!(tvmaze_channel_to_slug("HBO"), Some("max"));
        assert_eq!(tvmaze_channel_to_slug("HBO Max"), Some("max"));
        assert_eq!(tvmaze_channel_to_slug("Disney+"), Some("disney_plus"));
        assert_eq!(
            tvmaze_channel_to_slug("Amazon Prime Video"),
            Some("prime_video")
        );
        assert_eq!(tvmaze_channel_to_slug("Hulu"), Some("hulu"));
        assert_eq!(tvmaze_channel_to_slug("Apple TV+"), Some("apple_tv_plus"));
        assert_eq!(tvmaze_channel_to_slug("Peacock"), Some("peacock"));
        assert_eq!(tvmaze_channel_to_slug("Paramount+"), Some("paramount_plus"));
        assert_eq!(tvmaze_channel_to_slug("NBC"), None);
    }

    #[test]
    fn provider_search_urls_and_priority() {
        assert_eq!(
            provider_search_url("netflix", "The Bear: S1!"),
            "https://www.netflix.com/search?q=The+Bear%3A+S1%21"
        );
        assert!(provider_search_url("max", "Dune").contains("play.max.com"));
        assert!(provider_search_url("disney_plus", "Andor").contains("disneyplus.com"));
        assert!(provider_search_url("prime_video", "Fallout").contains("amazon.com"));
        assert!(provider_search_url("hulu", "Shogun").contains("hulu.com"));
        assert!(provider_search_url("apple_tv_plus", "Severance").contains("tv.apple.com"));
        assert!(provider_search_url("peacock", "Poker Face").contains("peacocktv.com"));
        assert!(provider_search_url("paramount_plus", "Halo").contains("paramountplus.com"));
        assert!(provider_search_url("other", "Test").contains("themoviedb.org"));

        let _guard = ENV_LOCK.lock().unwrap();
        clear_test_env();
        let cfg = AppConfig::load_and_validate().unwrap();
        assert_eq!(cfg.provider_priority("netflix"), 0);
        assert_eq!(cfg.provider_priority("hulu"), 4);
        assert_eq!(cfg.provider_priority("unknown"), usize::MAX);
    }

    #[test]
    fn load_and_validate_with_config_file_and_env_overrides() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_test_env();

        let tmp = std::env::temp_dir().join(format!("cz_cfg_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&tmp).unwrap();
        let cfg_path = tmp.join("config.json");
        std::fs::write(
            &cfg_path,
            r#"{"port": 9090, "region": "CA", "timezone": "America/Toronto", "new_release_days": 14, "refresh_interval_hours": 12, "providers": ["netflix", "hulu"]}"#,
        )
        .unwrap();

        let secret_path = tmp.join("tmdb_key.txt");
        std::fs::write(&secret_path, "  secret-from-file \n").unwrap();

        std::env::set_var("CONFIG_FILE", &cfg_path);
        std::env::set_var("PORT", "8181");
        std::env::set_var("REGION", "gb");
        std::env::set_var("TZ", "Europe/London");
        std::env::set_var("NEW_RELEASE_DAYS", "21");
        std::env::set_var("REFRESH_INTERVAL_HOURS", "24");
        std::env::set_var("PROVIDERS", "hulu, max");
        std::env::set_var("TVMAZE_ENABLED", "false");
        std::env::set_var("TMDB_API_KEY_FILE", &secret_path);
        std::env::set_var("OMDB_API_KEY", "omdb-secret");
        std::env::set_var("OMDB_ENABLED", "0");
        std::env::set_var("WATCHMODE_API_KEY", "wm-secret");
        std::env::set_var("DATA_DIR", "/tmp/cz_data");
        std::env::set_var("STATIC_DIR", "/tmp/cz_static");

        let cfg = AppConfig::load_and_validate().unwrap();
        assert_eq!(cfg.port, 8181);
        assert_eq!(cfg.region, "GB");
        assert_eq!(cfg.timezone, "Europe/London");
        assert_eq!(cfg.new_release_days, 21);
        assert_eq!(cfg.refresh_interval_hours, 24);
        assert_eq!(cfg.providers, vec!["hulu", "max"]);
        assert!(!cfg.sources.tvmaze.enabled);
        assert!(cfg.sources.tmdb.enabled);
        assert_eq!(
            cfg.sources.tmdb.api_key.as_deref(),
            Some("secret-from-file")
        );
        assert!(!cfg.sources.omdb.enabled);
        assert_eq!(cfg.sources.omdb.api_key.as_deref(), Some("omdb-secret"));
        assert!(cfg.sources.watchmode.enabled);
        assert_eq!(cfg.sources.watchmode.api_key.as_deref(), Some("wm-secret"));
        assert_eq!(cfg.data_dir, "/tmp/cz_data");
        assert_eq!(cfg.static_dir, "/tmp/cz_static");

        // Test parse_bool_env variants and missing secret file
        std::env::set_var("TVMAZE_ENABLED", "yes");
        assert_eq!(parse_bool_env("TVMAZE_ENABLED"), Some(true));
        std::env::set_var("TVMAZE_ENABLED", "off");
        assert_eq!(parse_bool_env("TVMAZE_ENABLED"), Some(false));
        std::env::set_var("TVMAZE_ENABLED", "maybe");
        assert_eq!(parse_bool_env("TVMAZE_ENABLED"), None);

        std::env::set_var("MISSING_KEY_FILE", tmp.join("does_not_exist.txt"));
        assert_eq!(read_secret_env("MISSING_KEY"), None);

        // Invalid JSON config file returns error
        std::fs::write(&cfg_path, "{ not valid json").unwrap();
        assert!(AppConfig::load_and_validate().is_err());

        clear_test_env();
        let _ = std::fs::remove_dir_all(&tmp);
    }
}

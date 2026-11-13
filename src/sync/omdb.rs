use reqwest::Client;
use serde::Deserialize;

use crate::models::SourceRating;

#[derive(Debug, Deserialize)]
struct OmdbResponse {
    #[serde(rename = "Response")]
    response: String,
    #[serde(rename = "imdbRating")]
    imdb_rating: Option<String>,
    #[serde(rename = "Ratings")]
    ratings: Option<Vec<OmdbRatingEntry>>,
}

#[derive(Debug, Deserialize)]
struct OmdbRatingEntry {
    #[serde(rename = "Source")]
    source: String,
    #[serde(rename = "Value")]
    value: String,
}

pub async fn fetch_omdb_ratings(
    client: &Client,
    api_key: &str,
    imdb_id: &str,
) -> Option<Vec<SourceRating>> {
    let url = format!("https://www.omdbapi.com/?i={imdb_id}&apikey={api_key}");
    let resp = client.get(&url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let data: OmdbResponse = resp.json().await.ok()?;
    if data.response != "True" {
        return None;
    }

    let mut out = Vec::new();
    if let Some(imdb) = data.imdb_rating {
        if imdb != "N/A" {
            let norm = imdb.parse::<f64>().ok();
            out.push(SourceRating {
                source: "IMDb".into(),
                score: format!("{imdb}/10"),
                normalized_10: norm,
            });
        }
    }

    if let Some(entries) = data.ratings {
        for entry in entries {
            if entry.source == "Rotten Tomatoes" {
                let norm = entry
                    .value
                    .trim_end_matches('%')
                    .parse::<f64>()
                    .ok()
                    .map(|v| v / 10.0);
                out.push(SourceRating {
                    source: "Rotten Tomatoes".into(),
                    score: entry.value,
                    normalized_10: norm,
                });
            } else if entry.source == "Metacritic" {
                let norm = entry
                    .value
                    .split('/')
                    .next()
                    .and_then(|s| s.parse::<f64>().ok())
                    .map(|v| v / 10.0);
                out.push(SourceRating {
                    source: "Metacritic".into(),
                    score: entry.value,
                    normalized_10: norm,
                });
            }
        }
    }

    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

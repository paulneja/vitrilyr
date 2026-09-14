use crate::{cache, lrc, state::Track};
use anyhow::{Context, Result, bail};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{sync::Mutex, time::Instant};

#[derive(Clone, Debug)]
pub enum Lyrics {
    Synced(Vec<lrc::LyricLine>),
    Plain(String),
    Instrumental,
    Missing,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    #[serde(default)]
    pub instrumental: bool,
    pub plain_lyrics: Option<String>,
    pub synced_lyrics: Option<String>,
}

impl Record {
    pub fn lyrics(&self) -> Lyrics {
        if self.instrumental {
            return Lyrics::Instrumental;
        }
        if let Some(raw) = &self.synced_lyrics {
            let lines = lrc::parse(raw);
            if !lines.is_empty() {
                return Lyrics::Synced(lines);
            }
        }
        if let Some(raw) = &self.plain_lyrics
            && !raw.trim().is_empty()
        {
            return Lyrics::Plain(raw.clone());
        }
        Lyrics::Missing
    }
}

#[derive(Serialize, Deserialize)]
struct Cached {
    version: u8,
    fetched: u64,
    record: Option<Record>,
}

#[derive(Clone)]
pub struct Provider {
    client: reqwest::Client,
    next_request: Arc<Mutex<Instant>>,
    root: PathBuf,
}

pub struct Artwork {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

impl Provider {
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: reqwest::Client::builder()
                .user_agent(concat!(
                    "Vitrilyr/",
                    env!("CARGO_PKG_VERSION"),
                    " (+https://github.com/paulneja/vitrilyr)"
                ))
                .connect_timeout(Duration::from_secs(5))
                .timeout(Duration::from_secs(12))
                .build()?,
            next_request: Arc::new(Mutex::new(Instant::now())),
            root: cache::root(),
        })
    }

    pub async fn lyrics(&self, track: &Track, force: bool) -> Result<Lyrics> {
        if track.title.is_empty() || track.artists.is_empty() {
            return Ok(Lyrics::Missing);
        }
        let path = self
            .root
            .join("lyrics")
            .join(format!("{}.json", track.key()));
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let cached = cache::read(&path, 2_000_000)
            .await
            .and_then(|b| serde_json::from_slice::<Cached>(&b).ok())
            .filter(|c| c.version == 1);
        if !force && let Some(cached) = &cached {
            let ttl = if cached.record.is_some() {
                30 * 86400
            } else {
                6 * 3600
            };
            if now.saturating_sub(cached.fetched) < ttl {
                return Ok(cached
                    .record
                    .as_ref()
                    .map(Record::lyrics)
                    .unwrap_or(Lyrics::Missing));
            }
        }
        let result = self.fetch(track).await;
        match result {
            Ok(record) => {
                let lyrics = record
                    .as_ref()
                    .map(Record::lyrics)
                    .unwrap_or(Lyrics::Missing);
                let data = serde_json::to_vec(&Cached {
                    version: 1,
                    fetched: now,
                    record,
                })?;
                if let Err(error) = cache::write(&path, &data).await {
                    tracing::warn!(%error,"Lyrics cache write failed");
                }
                Ok(lyrics)
            }
            Err(error) => {
                if let Some(record) = cached.and_then(|c| c.record) {
                    tracing::warn!(%error,"Using stale lyrics while offline");
                    Ok(record.lyrics())
                } else {
                    Err(error)
                }
            }
        }
    }

    async fn fetch(&self, track: &Track) -> Result<Option<Record>> {
        let mut next = self.next_request.lock().await;
        if *next > Instant::now() + Duration::from_secs(12) {
            bail!("LRCLIB is limiting requests. Try again later.");
        }
        tokio::time::sleep_until(*next).await;
        *next = Instant::now() + Duration::from_millis(350);
        let mut query = vec![
            ("track_name", track.title.clone()),
            ("artist_name", track.artists[0].clone()),
        ];
        if !track.album.is_empty() {
            query.push(("album_name", track.album.clone()));
        }
        if track.length_us > 0 {
            query.push(("duration", track.duration().as_secs_f64().to_string()));
        }
        let response = self
            .client
            .get("https://lrclib.net/api/get")
            .query(&query)
            .send()
            .await?;
        *next = Instant::now() + Duration::from_millis(350);
        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let seconds = response
                .headers()
                .get("retry-after")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(60)
                .min(86400);
            *next = Instant::now() + Duration::from_secs(seconds);
            bail!("LRCLIB: wait {seconds} seconds before retrying");
        }
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let bytes = bounded(response.error_for_status()?, 2_000_000).await?;
        Ok(Some(serde_json::from_slice(&bytes)?))
    }

    pub async fn artwork(&self, uri: &str) -> Result<Option<Artwork>> {
        if uri.is_empty() {
            return Ok(None);
        }
        let url = url::Url::parse(uri)?;
        let path = self.root.join("artwork").join(cache::key(uri.as_bytes()));
        if let Some(bytes) = cache::read(&path, 10_000_000).await
            && let Ok(art) = decode(bytes).await
        {
            return Ok(Some(art));
        }
        let bytes = match url.scheme() {
            "https" | "http" => {
                bounded(
                    self.client.get(url).send().await?.error_for_status()?,
                    10_000_000,
                )
                .await?
            }
            "file" => {
                let local = url
                    .to_file_path()
                    .map_err(|_| anyhow::anyhow!("Invalid artwork file URI"))?;
                cache::read(&local, 10_000_000)
                    .await
                    .context("Artwork file unavailable or too large")?
            }
            _ => bail!("Unsupported artwork URI"),
        };
        let art = decode(bytes.clone()).await?;
        if let Err(error) = cache::write(&path, &bytes).await {
            tracing::warn!(%error,"Artwork cache write failed");
        }
        Ok(Some(art))
    }
}

async fn bounded(response: reqwest::Response, max: usize) -> Result<Vec<u8>> {
    if response.content_length().is_some_and(|n| n > max as u64) {
        bail!("Response too large");
    }
    let mut bytes = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if bytes.len() + chunk.len() > max {
            bail!("Response too large");
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

async fn decode(bytes: Vec<u8>) -> Result<Artwork> {
    tokio::task::spawn_blocking(move || {
        let mut reader =
            image::ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format()?;
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(8192);
        limits.max_image_height = Some(8192);
        limits.max_alloc = Some(128 * 1024 * 1024);
        reader.limits(limits);
        let image = reader
            .decode()?
            .resize_to_fill(64, 64, image::imageops::FilterType::Triangle)
            .into_rgba8();
        Ok(Artwork {
            width: image.width(),
            height: image.height(),
            rgba: image.into_raw(),
        })
    })
    .await?
}

#[cfg(test)]
mod tests {
    use super::*;
    fn offline_provider(root: PathBuf) -> Provider {
        Provider {
            client: reqwest::Client::builder()
                .proxy(reqwest::Proxy::all("http://127.0.0.1:1").unwrap())
                .timeout(Duration::from_millis(100))
                .build()
                .unwrap(),
            next_request: Arc::new(Mutex::new(Instant::now())),
            root,
        }
    }
    fn track() -> Track {
        Track {
            title: "Test".into(),
            artists: vec!["Artist".into()],
            ..Track::default()
        }
    }
    #[tokio::test]
    async fn offline_uses_stale_success_but_not_corrupt_cache() {
        let dir = tempfile::tempdir().unwrap();
        let provider = offline_provider(dir.path().to_owned());
        let track = track();
        let path = provider
            .root
            .join("lyrics")
            .join(format!("{}.json", track.key()));
        let cached = Cached {
            version: 1,
            fetched: 0,
            record: Some(Record {
                synced_lyrics: Some("[00:01.00]Original test line".into()),
                ..Record::default()
            }),
        };
        cache::write(&path, &serde_json::to_vec(&cached).unwrap())
            .await
            .unwrap();
        assert!(matches!(
            provider.lyrics(&track, false).await.unwrap(),
            Lyrics::Synced(_)
        ));
        cache::write(&path, b"corrupt JSON").await.unwrap();
        assert!(provider.lyrics(&track, false).await.is_err());
    }
    #[tokio::test]
    async fn negative_cache_avoids_network_and_bad_artwork_is_recoverable() {
        let dir = tempfile::tempdir().unwrap();
        let provider = offline_provider(dir.path().to_owned());
        let track = track();
        let path = provider
            .root
            .join("lyrics")
            .join(format!("{}.json", track.key()));
        let cached = Cached {
            version: 1,
            fetched: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            record: None,
        };
        cache::write(&path, &serde_json::to_vec(&cached).unwrap())
            .await
            .unwrap();
        assert!(matches!(
            provider.lyrics(&track, false).await.unwrap(),
            Lyrics::Missing
        ));
        assert!(provider.artwork("").await.unwrap().is_none());
        assert!(decode(b"not an image".to_vec()).await.is_err());
    }
    #[test]
    fn honest_fallback_modes() {
        let plain = Record {
            synced_lyrics: Some("broken".into()),
            plain_lyrics: Some("A static verse".into()),
            ..Record::default()
        };
        assert!(matches!(plain.lyrics(), Lyrics::Plain(_)));
        assert!(matches!(Record::default().lyrics(), Lyrics::Missing));
        assert!(matches!(
            Record {
                instrumental: true,
                ..plain
            }
            .lyrics(),
            Lyrics::Instrumental
        ));
    }
}

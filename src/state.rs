use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Track {
    pub id: String,
    pub title: String,
    pub artists: Vec<String>,
    pub album: String,
    pub length_us: u64,
    pub art_url: String,
}

impl Track {
    pub fn artist(&self) -> String {
        self.artists.join(", ")
    }
    pub fn key(&self) -> String {
        crate::cache::key(
            &serde_json::to_vec(&(
                &self.title,
                &self.artists,
                &self.album,
                self.length_us / 1_000_000,
            ))
            .unwrap_or_default(),
        )
    }
    pub fn duration(&self) -> Duration {
        Duration::from_micros(self.length_us)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub enum Playback {
    Playing,
    Paused,
    #[default]
    Stopped,
}

#[derive(Clone, Debug)]
pub struct Snapshot {
    pub track: Track,
    pub playback: Playback,
    pub position: Duration,
    pub sampled_at: Instant,
    pub rate: f64,
    pub volume: Option<f64>,
    pub can_control: bool,
    pub can_play: bool,
    pub can_pause: bool,
    pub can_seek: bool,
    pub can_next: bool,
    pub can_previous: bool,
}

impl Snapshot {
    pub fn position_at(&self, now: Instant) -> Duration {
        let elapsed = if self.playback == Playback::Playing {
            now.saturating_duration_since(self.sampled_at)
                .mul_f64(self.rate.clamp(0.0, 16.0))
        } else {
            Duration::ZERO
        };
        let position = self.position.saturating_add(elapsed);
        if self.track.length_us > 0 {
            position.min(self.track.duration())
        } else {
            position
        }
    }
}

#[derive(Clone, Debug)]
pub enum MediaCommand {
    PlayPause,
    Next,
    Previous,
    Seek(f64),
    SetPosition(String, f64),
    Volume(f64),
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clock_pauses_clamps_and_accepts_seek_updates() {
        let now = Instant::now();
        let mut s = Snapshot {
            track: Track {
                length_us: 60_000_000,
                ..Track::default()
            },
            playback: Playback::Playing,
            position: Duration::from_secs(10),
            sampled_at: now,
            rate: 1.0,
            volume: None,
            can_control: true,
            can_play: true,
            can_pause: true,
            can_seek: true,
            can_next: true,
            can_previous: true,
        };
        assert_eq!(
            s.position_at(now + Duration::from_secs(3)),
            Duration::from_secs(13)
        );
        s.playback = Playback::Paused;
        assert_eq!(
            s.position_at(now + Duration::from_secs(3)),
            Duration::from_secs(10)
        );
        s.position = Duration::from_secs(2);
        assert_eq!(s.position_at(now), Duration::from_secs(2));
        s.playback = Playback::Playing;
        assert_eq!(
            s.position_at(now + Duration::from_secs(90)),
            Duration::from_secs(60)
        );
    }
}

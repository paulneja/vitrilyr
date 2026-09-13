use crate::state::{MediaCommand, Playback, Snapshot, Track};
use anyhow::{Context, Result};
use futures_util::StreamExt;
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};
use tokio::sync::mpsc;
use zbus::{
    Connection, Proxy,
    proxy::CacheProperties,
    zvariant::{OwnedObjectPath, OwnedValue},
};

const PATH: &str = "/org/mpris/MediaPlayer2";
const PLAYER: &str = "org.mpris.MediaPlayer2.Player";
type Properties = HashMap<String, OwnedValue>;

#[derive(Debug)]
pub enum PlayerEvent {
    State(Box<Snapshot>),
    Unavailable,
    Error(String),
}

fn text(values: &Properties, key: &str) -> String {
    values
        .get(key)
        .and_then(|v| <&str>::try_from(v).ok())
        .unwrap_or_default()
        .into()
}
fn boolean(values: &Properties, key: &str) -> bool {
    values
        .get(key)
        .and_then(|v| bool::try_from(v).ok())
        .unwrap_or(false)
}
fn number(values: &Properties, key: &str) -> Option<f64> {
    values
        .get(key)
        .and_then(|v| f64::try_from(v).ok())
        .filter(|n| n.is_finite())
}
fn micros(values: &Properties, key: &str) -> u64 {
    values
        .get(key)
        .and_then(|v| {
            u64::try_from(v)
                .ok()
                .or_else(|| i64::try_from(v).ok().map(|n| n.max(0) as u64))
        })
        .unwrap_or(0)
}

pub fn snapshot(mut values: Properties, sampled_at: Instant) -> Snapshot {
    let metadata = values
        .remove("Metadata")
        .and_then(|v| Properties::try_from(v).ok())
        .unwrap_or_default();
    let artists = metadata
        .get("xesam:artist")
        .and_then(|v| v.try_clone().ok())
        .and_then(|v| Vec::<String>::try_from(v).ok())
        .unwrap_or_default();
    let id = metadata
        .get("mpris:trackid")
        .and_then(|v| <&zbus::zvariant::ObjectPath>::try_from(v).ok())
        .map(ToString::to_string)
        .unwrap_or_else(|| text(&metadata, "mpris:trackid"));
    Snapshot {
        track: Track {
            id,
            title: text(&metadata, "xesam:title"),
            artists,
            album: text(&metadata, "xesam:album"),
            length_us: micros(&metadata, "mpris:length"),
            art_url: text(&metadata, "mpris:artUrl"),
        },
        playback: match text(&values, "PlaybackStatus").as_str() {
            "Playing" => Playback::Playing,
            "Paused" => Playback::Paused,
            _ => Playback::Stopped,
        },
        position: Duration::from_micros(micros(&values, "Position")),
        sampled_at,
        rate: number(&values, "Rate").unwrap_or(1.0).clamp(0.0, 16.0),
        volume: number(&values, "Volume"),
        can_control: boolean(&values, "CanControl"),
        can_play: boolean(&values, "CanPlay"),
        can_pause: boolean(&values, "CanPause"),
        can_seek: boolean(&values, "CanSeek"),
        can_next: boolean(&values, "CanGoNext"),
        can_previous: boolean(&values, "CanGoPrevious"),
    }
}

async fn proxy<'a>(
    connection: &'a Connection,
    destination: &'a str,
    interface: &'a str,
) -> Result<Proxy<'a>> {
    Ok(Proxy::new(connection, destination, PATH, interface).await?)
}

async fn discover(connection: &Connection) -> Result<Option<String>> {
    let dbus = zbus::fdo::DBusProxy::new(connection).await?;
    let mut names: Vec<String> = dbus
        .list_names()
        .await?
        .iter()
        .map(ToString::to_string)
        .filter(|name| {
            name.to_ascii_lowercase()
                .starts_with("org.mpris.mediaplayer2.spotify")
        })
        .collect();
    names.sort();
    Ok(names.into_iter().next())
}

async fn refresh(properties: &Proxy<'_>) -> Result<Snapshot> {
    let sampled_at = Instant::now();
    let values: Properties = properties.call("GetAll", &(PLAYER,)).await?;
    Ok(snapshot(values, sampled_at))
}

async fn control(player: &Proxy<'_>, command: MediaCommand) -> Result<()> {
    match command {
        MediaCommand::PlayPause => player.call::<_, _, ()>("PlayPause", &()).await?,
        MediaCommand::Next => player.call::<_, _, ()>("Next", &()).await?,
        MediaCommand::Previous => player.call::<_, _, ()>("Previous", &()).await?,
        MediaCommand::Seek(seconds) => {
            player
                .call::<_, _, ()>(
                    "Seek",
                    &((seconds.clamp(-86400.0, 86400.0) * 1_000_000.0) as i64,),
                )
                .await?
        }
        MediaCommand::SetPosition(id, seconds) => {
            let path = OwnedObjectPath::try_from(id).context("Track cannot be seeked")?;
            player
                .call::<_, _, ()>(
                    "SetPosition",
                    &(path, (seconds.max(0.0) * 1_000_000.0) as i64),
                )
                .await?;
        }
        MediaCommand::Volume(volume) => {
            player
                .set_property("Volume", volume.clamp(0.0, 1.0))
                .await?
        }
    }
    Ok(())
}

pub async fn watch(
    commands: &mut mpsc::UnboundedReceiver<MediaCommand>,
    emit: &(impl Fn(PlayerEvent) + Send + Sync),
) -> Result<()> {
    let connection = Connection::session().await?;
    watch_on(&connection, commands, emit).await
}

pub async fn watch_on(
    connection: &Connection,
    commands: &mut mpsc::UnboundedReceiver<MediaCommand>,
    emit: &(impl Fn(PlayerEvent) + Send + Sync),
) -> Result<()> {
    let dbus = zbus::fdo::DBusProxy::new(connection).await?;
    let mut owners = dbus.receive_name_owner_changed().await?;
    loop {
        let Some(name) = discover(connection).await? else {
            emit(PlayerEvent::Unavailable);
            tokio::select! {
                event = owners.next() => { event.context("D-Bus disconnected")?; }
                command = commands.recv() => {
                    command.context("Command channel closed")?;
                    emit(PlayerEvent::Error("Spotify no esta disponible".into()));
                }
                _ = tokio::time::sleep(Duration::from_secs(5)) => {}
            }
            continue;
        };
        let player = zbus::proxy::Builder::<Proxy<'_>>::new(connection)
            .destination(name.as_str())?
            .path(PATH)?
            .interface(PLAYER)?
            .cache_properties(CacheProperties::No)
            .build()
            .await?;
        let properties = proxy(connection, &name, "org.freedesktop.DBus.Properties").await?;
        let mut changes = properties.receive_signal("PropertiesChanged").await?;
        let mut seeks = player.receive_signal("Seeked").await?;
        let mut timer = tokio::time::interval(Duration::from_secs(2));
        timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            let result = tokio::time::timeout(Duration::from_secs(3), refresh(&properties)).await;
            match result {
                Ok(Ok(s)) => emit(PlayerEvent::State(Box::new(s))),
                _ => {
                    emit(PlayerEvent::Unavailable);
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    break;
                }
            }
            tokio::select! {
                change = changes.next() => { if change.is_none() { break; } }
                seek = seeks.next() => { if seek.is_none() { break; } }
                _ = timer.tick() => {}
                owner = owners.next() => {
                    let Some(owner) = owner else { anyhow::bail!("D-Bus disconnected"); };
                    if owner.args().is_ok_and(|args| args.name().as_str() == name) { break; }
                }
                command = commands.recv() => {
                    let command = command.context("Command channel closed")?;
                    match tokio::time::timeout(Duration::from_secs(3), control(&player, command)).await {
                        Ok(Ok(())) => {},
                        Ok(Err(error)) => emit(PlayerEvent::Error(format!("Spotify: {error}"))),
                        Err(_) => emit(PlayerEvent::Error("Spotify no responde".into())),
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn spotify_unsigned_length_and_string_track_id() {
        let mut metadata = Properties::new();
        metadata.insert("mpris:length".into(), OwnedValue::from(156_281_000u64));
        metadata.insert(
            "mpris:trackid".into(),
            zbus::zvariant::Value::from("/com/spotify/track/example")
                .try_to_owned()
                .unwrap(),
        );
        let mut values = Properties::new();
        values.insert("Metadata".into(), OwnedValue::from(metadata));
        let s = snapshot(values, Instant::now());
        assert_eq!(s.track.length_us, 156_281_000);
        assert_eq!(s.track.id, "/com/spotify/track/example");
    }
    #[test]
    fn malformed_metadata_is_an_idle_snapshot() {
        let mut values = Properties::new();
        values.insert("Metadata".into(), OwnedValue::from(42i32));
        values.insert("Position".into(), OwnedValue::from(-50i64));
        let s = snapshot(values, Instant::now());
        assert_eq!(s.track, Track::default());
        assert_eq!(s.position, Duration::ZERO);
        assert!(!s.can_seek);
    }
}

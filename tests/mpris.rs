use lyricglass::{
    player::{self, PlayerEvent},
    state::{MediaCommand, Playback, Snapshot},
};
use std::{
    collections::HashMap,
    io::{BufRead, BufReader},
    process::{Child, Command, Stdio},
    time::Duration,
};
use tokio::sync::mpsc;
use zbus::{
    Connection,
    object_server::SignalEmitter,
    zvariant::{OwnedObjectPath, OwnedValue, Value},
};

const NAME: &str = "org.mpris.MediaPlayer2.spotify.LyricGlassTest";
const PATH: &str = "/org/mpris/MediaPlayer2";

struct Bus(Child);
impl Drop for Bus {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

struct Spotify {
    playing: bool,
    position: i64,
    title: String,
    volume: f64,
}
impl Default for Spotify {
    fn default() -> Self {
        Self {
            playing: true,
            position: 10_000_000,
            title: "First song".into(),
            volume: 0.5,
        }
    }
}

#[zbus::interface(name = "org.mpris.MediaPlayer2.Player")]
impl Spotify {
    #[zbus(property)]
    fn playback_status(&self) -> &str {
        if self.playing { "Playing" } else { "Paused" }
    }
    #[zbus(property)]
    fn metadata(&self) -> HashMap<String, OwnedValue> {
        let mut result = HashMap::new();
        result.insert(
            "xesam:title".into(),
            Value::from(self.title.as_str()).try_to_owned().unwrap(),
        );
        result.insert(
            "xesam:artist".into(),
            Value::from(vec!["Test artist"]).try_to_owned().unwrap(),
        );
        result.insert(
            "xesam:album".into(),
            Value::from("Test album").try_to_owned().unwrap(),
        );
        result.insert("mpris:length".into(), OwnedValue::from(180_000_000i64));
        result.insert(
            "mpris:trackid".into(),
            Value::from(OwnedObjectPath::try_from("/track/test").unwrap())
                .try_to_owned()
                .unwrap(),
        );
        result
    }
    #[zbus(property)]
    fn position(&self) -> i64 {
        self.position
    }
    #[zbus(property)]
    fn rate(&self) -> f64 {
        1.0
    }
    #[zbus(property)]
    fn volume(&self) -> f64 {
        self.volume
    }
    #[zbus(property)]
    fn set_volume(&mut self, value: f64) {
        self.volume = value;
    }
    #[zbus(property)]
    fn can_control(&self) -> bool {
        true
    }
    #[zbus(property)]
    fn can_play(&self) -> bool {
        true
    }
    #[zbus(property)]
    fn can_pause(&self) -> bool {
        true
    }
    #[zbus(property)]
    fn can_seek(&self) -> bool {
        true
    }
    #[zbus(property)]
    fn can_go_next(&self) -> bool {
        true
    }
    #[zbus(property)]
    fn can_go_previous(&self) -> bool {
        true
    }
    async fn play_pause(
        &mut self,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<()> {
        self.playing = !self.playing;
        Ok(self.playback_status_changed(&emitter).await?)
    }
    async fn next(
        &mut self,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<()> {
        self.title = "Second song".into();
        self.position = 0;
        Ok(self.metadata_changed(&emitter).await?)
    }
    async fn previous(
        &mut self,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<()> {
        self.title = "First song".into();
        self.position = 0;
        Ok(self.metadata_changed(&emitter).await?)
    }
    async fn seek(
        &mut self,
        offset: i64,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<()> {
        self.position = (self.position + offset).max(0);
        Ok(Self::seeked(&emitter, self.position).await?)
    }
    async fn set_position(
        &mut self,
        track: OwnedObjectPath,
        position: i64,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<()> {
        assert_eq!(track.as_str(), "/track/test");
        self.position = position;
        Ok(Self::seeked(&emitter, self.position).await?)
    }
    #[zbus(signal)]
    async fn seeked(emitter: &SignalEmitter<'_>, position: i64) -> zbus::Result<()>;
}

async fn state(
    events: &mut mpsc::UnboundedReceiver<PlayerEvent>,
    predicate: impl Fn(&Snapshot) -> bool,
) -> Snapshot {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match events.recv().await.expect("watcher stopped") {
                PlayerEvent::State(s) if predicate(&s) => return *s,
                PlayerEvent::Error(error) => panic!("{error}"),
                _ => {}
            }
        }
    })
    .await
    .expect("expected state did not arrive")
}
async fn unavailable(events: &mut mpsc::UnboundedReceiver<PlayerEvent>) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while !matches!(events.recv().await, Some(PlayerEvent::Unavailable)) {}
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn spotify_lifecycle_and_transport_on_private_bus() {
    let child = Command::new("dbus-daemon")
        .args(["--session", "--nofork", "--print-address=1"])
        .stdout(Stdio::piped())
        .spawn()
        .expect("Install dbus to run integration tests");
    let mut bus = Bus(child);
    let mut address = String::new();
    BufReader::new(bus.0.stdout.take().unwrap())
        .read_line(&mut address)
        .unwrap();
    assert!(!address.trim().is_empty(), "Private D-Bus could not start");
    let client = zbus::connection::Builder::address(address.trim())
        .unwrap()
        .build()
        .await
        .unwrap();
    let (commands, mut incoming) = mpsc::unbounded_channel();
    let (emit, mut events) = mpsc::unbounded_channel();
    let watcher = tokio::spawn(async move {
        player::watch_on(&client, &mut incoming, &|event| {
            let _ = emit.send(event);
        })
        .await
    });
    unavailable(&mut events).await;
    let server: Connection = zbus::connection::Builder::address(address.trim())
        .unwrap()
        .name(NAME)
        .unwrap()
        .serve_at(PATH, Spotify::default())
        .unwrap()
        .build()
        .await
        .unwrap();
    let playing = state(&mut events, |s| s.playback == Playback::Playing).await;
    assert_eq!(playing.track.title, "First song");
    assert_eq!(playing.track.artists, ["Test artist"]);
    assert_eq!(playing.track.id, "/track/test");
    assert_eq!(playing.track.length_us, 180_000_000);
    commands.send(MediaCommand::PlayPause).unwrap();
    state(&mut events, |s| s.playback == Playback::Paused).await;
    commands.send(MediaCommand::Seek(25.0)).unwrap();
    state(&mut events, |s| s.position.as_secs() == 35).await;
    commands.send(MediaCommand::Seek(-30.0)).unwrap();
    state(&mut events, |s| s.position.as_secs() == 5).await;
    commands
        .send(MediaCommand::SetPosition("/track/test".into(), 90.0))
        .unwrap();
    state(&mut events, |s| s.position.as_secs() == 90).await;
    commands.send(MediaCommand::Volume(0.25)).unwrap();
    state(&mut events, |s| s.volume == Some(0.25)).await;
    commands.send(MediaCommand::Next).unwrap();
    state(&mut events, |s| s.track.title == "Second song").await;
    commands.send(MediaCommand::Previous).unwrap();
    state(&mut events, |s| s.track.title == "First song").await;
    server.release_name(NAME).await.unwrap();
    unavailable(&mut events).await;
    server.request_name(NAME).await.unwrap();
    state(&mut events, |s| s.track.title == "First song").await;
    watcher.abort();
}

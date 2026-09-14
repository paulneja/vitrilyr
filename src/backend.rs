use lyricglass::{
    config::{self, Config},
    lyrics::{Artwork, Lyrics, Provider},
    player::{self, PlayerEvent},
    state::{MediaCommand, Track},
};
use std::{sync::mpsc, time::Duration};
use tokio::sync::mpsc as async_mpsc;

pub enum Event {
    Shutdown,
    Player(PlayerEvent),
    Lyrics(u64, Result<Lyrics, String>),
    Art(u64, Option<Artwork>),
    Notice(String),
    Shortcuts(lyricglass::config::Shortcuts),
    Startup(bool),
    Imported(Box<Config>),
}
pub enum Preference {
    Save(Config),
    Install(Config),
    Startup(Config),
    Import(std::path::PathBuf),
    Export(std::path::PathBuf, Config),
    Flush(mpsc::Sender<()>),
}
pub struct Load {
    pub generation: u64,
    pub track: Track,
    pub force: bool,
}

pub struct Backend {
    pub media: async_mpsc::UnboundedSender<MediaCommand>,
    pub loads: async_mpsc::UnboundedSender<Load>,
    pub preferences: async_mpsc::UnboundedSender<Preference>,
}

impl Backend {
    pub fn flush_preferences(&self) -> anyhow::Result<()> {
        let (sender, receiver) = mpsc::channel();
        self.preferences.send(Preference::Flush(sender))?;
        receiver.recv_timeout(Duration::from_secs(3))?;
        Ok(())
    }
    pub fn start() -> anyhow::Result<(Self, mpsc::Receiver<Event>)> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()?;
        let provider = Provider::new()?;
        let (events, receiver) = mpsc::channel();
        let (media, mut commands) = async_mpsc::unbounded_channel();
        let (loads, mut requests) = async_mpsc::unbounded_channel::<Load>();
        let (preferences, mut changes) = async_mpsc::unbounded_channel::<Preference>();
        std::thread::Builder::new()
            .name("lyricglass-worker".into())
            .spawn(move || {
                runtime.block_on(async move {
                    let signal_events = events.clone();
                    tokio::spawn(async move {
                        use tokio::signal::unix::{SignalKind, signal};
                        match (
                            signal(SignalKind::terminate()),
                            signal(SignalKind::interrupt()),
                        ) {
                            (Ok(mut term), Ok(mut interrupt)) => {
                                tokio::select! { _ = term.recv() => {}, _ = interrupt.recv() => {} }
                                let _ = signal_events.send(Event::Shutdown);
                            }
                            (Err(error), _) | (_, Err(error)) => {
                                tracing::warn!(%error, "Could not register shutdown signals");
                            }
                        }
                    });
                    let player_events = events.clone();
                    tokio::spawn(async move {
                        while !commands.is_closed() {
                            if let Err(error) = player::watch(&mut commands, &|event| {
                                let _ = player_events.send(Event::Player(event));
                            })
                            .await
                            {
                                tracing::warn!(%error,"MPRIS connection interrupted");
                                let _ = player_events.send(Event::Player(PlayerEvent::Unavailable));
                                tokio::time::sleep(Duration::from_secs(3)).await;
                            }
                        }
                    });
                    let settings_events = events.clone();
                    tokio::spawn(async move {
                        let mut pending = None;
                        while let Some(mut change) = match pending.take() {
                            Some(change) => Some(change),
                            None => changes.recv().await,
                        } {
                            // Coalesce slider motion into one atomic settings write.
                            if matches!(change, Preference::Save(_)) {
                                tokio::time::sleep(Duration::from_millis(180)).await;
                                while let Ok(next) = changes.try_recv() {
                                    if !matches!(next, Preference::Save(_)) {
                                        pending = Some(next);
                                        break;
                                    }
                                    change = next;
                                }
                            }
                            let (config, install) = match change {
                                Preference::Flush(done) => {
                                    let _ = done.send(());
                                    continue;
                                }
                                Preference::Import(path) => {
                                    match Config::read_from(&path).await {
                                        Ok(config) => {
                                            let _ = settings_events
                                                .send(Event::Imported(Box::new(config)));
                                        }
                                        Err(error) => {
                                            let _ = settings_events
                                                .send(Event::Notice(error.to_string()));
                                        }
                                    }
                                    continue;
                                }
                                Preference::Export(path, config) => {
                                    if let Err(error) = config.save().await {
                                        let _ =
                                            settings_events.send(Event::Notice(error.to_string()));
                                    }
                                    let notice = match config.export(&path).await {
                                        Ok(()) => "Settings exported".into(),
                                        Err(error) => error.to_string(),
                                    };
                                    let _ = settings_events.send(Event::Notice(notice));
                                    continue;
                                }
                                Preference::Save(c) => (c, false),
                                Preference::Install(c) => (c, true),
                                Preference::Startup(c) => {
                                    let enabled = c.autostart;
                                    if let Err(error) = tokio::task::spawn_blocking(move || {
                                        lyricglass::startup::set_enabled(enabled)
                                    })
                                    .await
                                    .unwrap_or_else(|e| Err(e.into()))
                                    {
                                        let _ = settings_events.send(Event::Notice(format!(
                                            "Could not update startup: {error}"
                                        )));
                                        continue;
                                    }
                                    let _ = settings_events.send(Event::Startup(enabled));
                                    (c, false)
                                }
                            };
                            if install {
                                let keys = config.shortcuts.clone();
                                match tokio::task::spawn_blocking(move || {
                                    config::install_shortcuts(&keys)
                                })
                                .await
                                {
                                    Ok(Ok(())) => {
                                        let _ = settings_events
                                            .send(Event::Shortcuts(config.shortcuts.clone()));
                                        let _ = settings_events.send(Event::Notice(
                                            "Shortcuts enabled in Niri".into(),
                                        ));
                                    }
                                    result => {
                                        let _ = settings_events.send(Event::Notice(format!(
                                            "Could not enable shortcuts: {result:?}"
                                        )));
                                        continue;
                                    }
                                }
                            }
                            if let Err(error) = config.save().await {
                                let _ = settings_events.send(Event::Notice(format!(
                                    "Could not save settings: {error}"
                                )));
                            }
                        }
                    });
                    let mut task: Option<tokio::task::JoinHandle<()>> = None;
                    while let Some(load) = requests.recv().await {
                        if let Some(task) = task.take() {
                            task.abort();
                        }
                        let provider = provider.clone();
                        let events = events.clone();
                        task = Some(tokio::spawn(async move {
                            let lyrics = async {
                                let result = provider
                                    .lyrics(&load.track, load.force)
                                    .await
                                    .map_err(|e| e.to_string());
                                let _ = events.send(Event::Lyrics(load.generation, result));
                            };
                            let artwork = async {
                                let art = match provider.artwork(&load.track.art_url).await {
                                    Ok(art) => art,
                                    Err(error) => {
                                        tracing::debug!(%error,"Artwork unavailable");
                                        None
                                    }
                                };
                                let _ = events.send(Event::Art(load.generation, art));
                            };
                            tokio::join!(lyrics, artwork);
                        }));
                    }
                    if let Some(task) = task {
                        task.abort();
                    }
                });
            })?;
        Ok((
            Self {
                media,
                loads,
                preferences,
            },
            receiver,
        ))
    }
}

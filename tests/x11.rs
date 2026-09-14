use std::{
    io::{BufRead, BufReader, Read},
    path::Path,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};
use x11rb::{
    connection::Connection,
    protocol::{
        shape::ConnectionExt as _,
        xproto::{self, ConnectionExt as _},
        xtest::ConnectionExt as _,
    },
};

struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
struct Session {
    display: String,
    bus: String,
    config: tempfile::TempDir,
}
impl Session {
    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_lyricglass"));
        command
            .env_remove("WAYLAND_DISPLAY")
            .env_remove("NIRI_SOCKET")
            .env("DISPLAY", &self.display)
            .env("GDK_BACKEND", "x11")
            .env("GSK_RENDERER", "cairo")
            .env("GTK_A11Y", "none")
            .env("GIO_USE_VFS", "local")
            .env("DBUS_SESSION_BUS_ADDRESS", &self.bus)
            .env("XDG_CONFIG_HOME", self.config.path());
        command
    }
    fn cli(&self, args: &[&str]) -> String {
        let mut child = Process(
            self.command()
                .args(args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap(),
        );
        let deadline = Instant::now() + Duration::from_secs(10);
        let status = loop {
            if let Some(status) = child.0.try_wait().unwrap() {
                break status;
            }
            assert!(Instant::now() < deadline, "Command timed out: {args:?}");
            thread::sleep(Duration::from_millis(10));
        };
        let mut stdout = String::new();
        let mut stderr = String::new();
        child
            .0
            .stdout
            .take()
            .unwrap()
            .read_to_string(&mut stdout)
            .unwrap();
        child
            .0
            .stderr
            .take()
            .unwrap()
            .read_to_string(&mut stderr)
            .unwrap();
        assert!(status.success(), "{args:?}: {stderr}");
        stdout
    }
    fn wait_owner(&self) {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let connection = zbus::connection::Builder::address(self.bus.as_str())
                .unwrap()
                .build()
                .await
                .unwrap();
            let bus = zbus::fdo::DBusProxy::new(&connection).await.unwrap();
            let deadline = Instant::now() + Duration::from_secs(8);
            loop {
                if bus
                    .name_has_owner("io.github.lyricglass.LyricGlass".try_into().unwrap())
                    .await
                    .unwrap()
                {
                    break;
                }
                assert!(
                    Instant::now() < deadline,
                    "Application did not register on D-Bus"
                );
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        });
    }
    fn status(&self) -> serde_json::Value {
        serde_json::from_str(&self.cli(&["status"])).unwrap()
    }
    fn wait(&self, check: impl Fn(&serde_json::Value) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(8);
        loop {
            if check(&self.status()) {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "Window state did not settle: {}",
                self.status()
            );
            thread::sleep(Duration::from_millis(80));
        }
    }
    fn capture(&self, directory: &Path, name: &str, settings: bool) {
        thread::sleep(Duration::from_millis(150));
        let path = directory.join(name);
        self.cli(&[
            if settings {
                "capture-settings"
            } else {
                "capture"
            },
            path.to_str().unwrap(),
        ]);
        let image = image::open(&path).unwrap().to_rgba8();
        assert!(image.width() >= 240 && image.height() >= 50);
        let visible = image.pixels().filter(|p| p[3] > 0).count();
        assert!(visible > 1000, "Blank native rendering");
    }
}

#[test]
#[ignore = "Requires Xvfb; launches an isolated native X11 session"]
fn native_x11_layouts_languages_shortcuts_and_backups() {
    let mut server = Process(
        Command::new("Xvfb")
            .args([
                "-displayfd",
                "1",
                "-screen",
                "0",
                "1280x800x24",
                "-nolisten",
                "tcp",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("Install Xvfb"),
    );
    let mut display = String::new();
    BufReader::new(server.0.stdout.take().unwrap())
        .read_line(&mut display)
        .unwrap();
    assert!(!display.trim().is_empty(), "Xvfb did not start");
    let bus_config = Path::new(env!("CARGO_MANIFEST_DIR")).join("compositor/preview-bus.conf");
    let mut daemon = Process(
        Command::new("dbus-daemon")
            .args(["--nofork", "--print-address=1", "--config-file"])
            .arg(bus_config)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let mut bus = String::new();
    BufReader::new(daemon.0.stdout.take().unwrap())
        .read_line(&mut bus)
        .unwrap();
    let session = Session {
        display: format!(":{}", display.trim()),
        bus: bus.trim().into(),
        config: tempfile::tempdir().unwrap(),
    };
    let config_dir = session.config.path().join("lyricglass");
    std::fs::create_dir_all(&config_dir).unwrap();
    let config = lyricglass::config::Config {
        autostart: false,
        free_position: true,
        x: 100,
        y: 100,
        animations: false,
        square_size: 240,
        ..Default::default()
    };
    std::fs::write(
        config_dir.join("config.json"),
        serde_json::to_vec(&config).unwrap(),
    )
    .unwrap();
    let mut app = Process(
        session
            .command()
            .arg("demo")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    thread::sleep(Duration::from_millis(600));
    session.wait_owner();
    session
        .wait(|s| s["backend"] == "X11" && s["visible"] == true && s["geometry"]["width"] == 580);
    let immediate = session.config.path().join("immediate.json");
    session.cli(&["style", "light"]);
    session.cli(&["export-config", immediate.to_str().unwrap()]);
    let exported: lyricglass::config::Config =
        serde_json::from_slice(&std::fs::read(&immediate).unwrap()).unwrap();
    assert_eq!(exported.theme, "light", "Export must use live preferences");
    let artifacts = std::env::var_os("LYRICGLASS_TEST_ARTIFACTS")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| session.config.path().join("captures"));
    std::fs::create_dir_all(&artifacts).unwrap();
    for style in ["glass", "dark", "light", "contrast", "minimal"] {
        session.cli(&["style", style]);
        session.wait(|s| s["theme"] == style);
        session.capture(&artifacts, &format!("x11-{style}.png"), false);
        session.cli(&["export-config", immediate.to_str().unwrap()]);
        let appearance: lyricglass::config::Config =
            serde_json::from_slice(&std::fs::read(&immediate).unwrap()).unwrap();
        std::fs::write(
            artifacts.join(format!("niri-{style}.kdl")),
            appearance.surface_effects(),
        )
        .unwrap();
        std::fs::write(
            artifacts.join(format!("liquid-{style}.kdl")),
            appearance.liquid_effects(),
        )
        .unwrap();
    }
    session.cli(&["style", "glass"]);
    for layout in ["square", "line", "normal"] {
        session.cli(&["layout", layout]);
        session.wait(|s| {
            s["layout"] == layout
                && (layout != "square"
                    || (s["geometry"]["width"] == 240 && s["geometry"]["height"] == 240))
        });
        session.capture(&artifacts, &format!("x11-{layout}.png"), false);
    }
    for language in ["en", "es", "zh"] {
        session.cli(&["language", language]);
        session.cli(&["settings"]);
        session.wait(|s| s["language"] == language);
        session.capture(&artifacts, &format!("settings-{language}.png"), true);
    }
    session.cli(&["language", "en"]);
    session.cli(&["position", "220", "260"]);
    session.wait(|s| s["geometry"]["x"] == 220 && s["geometry"]["y"] == 260);
    let (connection, screen) = x11rb::connect(Some(&session.display)).unwrap();
    let root = connection.setup().roots[screen].root;
    let atom = |name: &[u8]| {
        connection
            .intern_atom(false, name)
            .unwrap()
            .reply()
            .unwrap()
            .atom
    };
    let window = connection
        .query_tree(root)
        .unwrap()
        .reply()
        .unwrap()
        .children
        .into_iter()
        .find(|window| {
            connection
                .get_property(
                    false,
                    *window,
                    atom(b"_NET_WM_NAME"),
                    xproto::AtomEnum::ANY,
                    0,
                    128,
                )
                .unwrap()
                .reply()
                .unwrap()
                .value
                == b"LyricGlass"
        })
        .expect("Native X11 window");
    let geometry = connection.get_geometry(window).unwrap().reply().unwrap();
    assert_eq!((geometry.x, geometry.y), (220, 260));
    connection
        .configure_window(
            window,
            &xproto::ConfigureWindowAux::new().stack_mode(xproto::StackMode::ABOVE),
        )
        .unwrap();
    let pointer = |kind, detail, x, y| {
        connection
            .xtest_fake_input(kind, detail, 0, root, x, y, 0)
            .unwrap();
        connection.flush().unwrap();
        thread::sleep(Duration::from_millis(100));
    };
    pointer(xproto::MOTION_NOTIFY_EVENT, 0, 250, 76);
    pointer(xproto::BUTTON_PRESS_EVENT, 1, 250, 76);
    pointer(xproto::BUTTON_RELEASE_EVENT, 1, 250, 76);
    session.wait(|s| s["settings_page"] == "usage");
    session.capture(&artifacts, "settings-behavior-en.png", true);
    session.cli(&["language", "zh"]);
    session.wait(|s| s["language"] == "zh" && s["settings_page"] == "usage");
    session.capture(&artifacts, "settings-behavior-zh.png", true);
    session.cli(&["language", "en"]);
    let grip_x = geometry.x + geometry.width as i16 - 34;
    let grip_y = geometry.y + 44;
    pointer(xproto::MOTION_NOTIFY_EVENT, 0, grip_x, grip_y);
    pointer(xproto::BUTTON_PRESS_EVENT, 1, grip_x, grip_y);
    pointer(xproto::MOTION_NOTIFY_EVENT, 0, grip_x + 20, grip_y + 10);
    pointer(xproto::MOTION_NOTIFY_EVENT, 0, grip_x + 50, grip_y + 40);
    pointer(xproto::BUTTON_RELEASE_EVENT, 1, grip_x + 50, grip_y + 40);
    session.wait(|s| s["geometry"]["x"] == 270 && s["geometry"]["y"] == 300);
    let moved = connection.get_geometry(window).unwrap().reply().unwrap();
    assert_eq!((moved.x, moved.y), (270, 300));
    let state = connection
        .get_property(
            false,
            window,
            atom(b"_NET_WM_STATE"),
            xproto::AtomEnum::ATOM,
            0,
            16,
        )
        .unwrap()
        .reply()
        .unwrap();
    assert!(
        state
            .value32()
            .unwrap()
            .any(|value| value == atom(b"_NET_WM_STATE_ABOVE"))
    );
    let setup = connection.setup();
    let map = connection
        .get_keyboard_mapping(setup.min_keycode, setup.max_keycode - setup.min_keycode + 1)
        .unwrap()
        .reply()
        .unwrap();
    let f7 = setup.min_keycode
        + map
            .keysyms
            .chunks(usize::from(map.keysyms_per_keycode))
            .position(|row| row.contains(&0xffc4))
            .unwrap() as u8;
    for visible in [false, true] {
        connection
            .xtest_fake_input(xproto::KEY_PRESS_EVENT, f7, 0, root, 0, 0, 0)
            .unwrap();
        connection
            .xtest_fake_input(xproto::KEY_RELEASE_EVENT, f7, 0, root, 0, 0, 0)
            .unwrap();
        connection.flush().unwrap();
        session.wait(|s| s["visible"] == visible);
    }
    session.cli(&["game-mode"]);
    thread::sleep(Duration::from_millis(200));
    let region = connection
        .shape_get_rectangles(window, x11rb::protocol::shape::SK::INPUT)
        .unwrap()
        .reply()
        .unwrap();
    assert!(
        region.rectangles.is_empty(),
        "Click-through must have an empty input region"
    );
    session.cli(&["game-mode"]);
    thread::sleep(Duration::from_millis(300));
    let backup = session.config.path().join("settings.json");
    session.cli(&["export-config", backup.to_str().unwrap()]);
    let mut imported: lyricglass::config::Config =
        serde_json::from_slice(&std::fs::read(&backup).unwrap()).unwrap();
    imported.theme = "light".into();
    imported.language = "zh".into();
    imported.autostart = true;
    std::fs::write(&backup, serde_json::to_vec(&imported).unwrap()).unwrap();
    session.cli(&["import-config", backup.to_str().unwrap()]);
    session.wait(|s| s["theme"] == "light" && s["language"] == "zh");
    thread::sleep(Duration::from_millis(300));
    let saved: lyricglass::config::Config =
        serde_json::from_slice(&std::fs::read(config_dir.join("config.json")).unwrap()).unwrap();
    assert!(!saved.autostart, "Import must not enable login startup");
    assert_eq!((saved.x, saved.y), (270, 300));
    let mut inaccessible = saved;
    inaccessible.click_through = true;
    inaccessible.lock_position = true;
    inaccessible.lyrics_only = true;
    inaccessible.hide_idle = true;
    inaccessible.hide_paused = true;
    inaccessible.start_hidden = true;
    std::fs::write(&backup, serde_json::to_vec(&inaccessible).unwrap()).unwrap();
    session.cli(&["import-config", backup.to_str().unwrap()]);
    session.wait(|s| s["click_through"] == true);
    session.cli(&["hide"]);
    session.cli(&["recover"]);
    session.wait(|s| s["visible"] == true && s["click_through"] == false);
    session.cli(&["export-config", immediate.to_str().unwrap()]);
    let recovered: lyricglass::config::Config =
        serde_json::from_slice(&std::fs::read(&immediate).unwrap()).unwrap();
    assert_eq!(recovered.theme, "light");
    assert_eq!(recovered.language, "zh");
    assert!(!recovered.lock_position && !recovered.lyrics_only && !recovered.hide_paused);
    session.cli(&["position", "99999", "99999"]);
    let bounded = session.status();
    session.cli(&["style", "dark"]);
    session.cli(&["quit"]);
    assert!(app.0.wait().unwrap().success());
    let final_config: lyricglass::config::Config =
        serde_json::from_slice(&std::fs::read(config_dir.join("config.json")).unwrap()).unwrap();
    assert_eq!(final_config.theme, "dark", "Quit must flush pending writes");
    assert_eq!(
        i64::from(final_config.x),
        bounded["geometry"]["x"].as_i64().unwrap()
    );
    assert_eq!(
        i64::from(final_config.y),
        bounded["geometry"]["y"].as_i64().unwrap()
    );
    assert!(final_config.x < 1280 && final_config.y < 800);
    let mut restarted = Process(
        session
            .command()
            .arg("demo")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    session.wait_owner();
    session.wait(|s| s["theme"] == "dark" && s["demo"] == true);
    session.cli(&["style", "contrast"]);
    assert!(
        Command::new("kill")
            .args(["-TERM", &restarted.0.id().to_string()])
            .status()
            .unwrap()
            .success()
    );
    assert!(restarted.0.wait().unwrap().success());
    let terminated: lyricglass::config::Config =
        serde_json::from_slice(&std::fs::read(config_dir.join("config.json")).unwrap()).unwrap();
    assert_eq!(
        terminated.theme, "contrast",
        "Service stop must flush pending writes"
    );
    let mut oversized = vec![b' '; 128 * 1024];
    oversized.extend_from_slice(br#"{"theme":"light"}"#);
    std::fs::write(config_dir.join("config.json"), oversized).unwrap();
    session.cli(&["export-config", immediate.to_str().unwrap()]);
    let fallback: lyricglass::config::Config =
        serde_json::from_slice(&std::fs::read(&immediate).unwrap()).unwrap();
    assert_eq!(
        fallback.theme, "glass",
        "Startup must bound config reads too"
    );
}

use anyhow::{Context, Result, ensure};
use gio::prelude::*;
use std::{path::Path, process::Command};

pub fn desktop_exec(path: &Path) -> Result<String> {
    let path = path.to_str().context("Executable path is not UTF-8")?;
    ensure!(
        !path.contains(['\n', '\r', '\0']),
        "Invalid executable path"
    );
    let escaped = path
        .replace('\\', "\\\\\\\\")
        .replace('"', "\\\"")
        .replace('`', "\\`")
        .replace('$', "\\$")
        .replace('%', "%%");
    Ok(format!("\"{escaped}\""))
}

pub fn unit(executable: &Path) -> Result<String> {
    let path = executable
        .to_str()
        .context("Executable path is not UTF-8")?;
    ensure!(
        !path.contains(['\n', '\r', '\0']),
        "Invalid executable path"
    );
    let path = path
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('%', "%%")
        .replace('$', "$$");
    Ok(format!(
        "[Unit]\nDescription=LyricGlass music overlay\nPartOf=graphical-session.target\nAfter=graphical-session-pre.target\n\n[Service]\nExecStart=\"{path}\"\nRestart=on-failure\nRestartSec=3\n\n[Install]\nWantedBy=graphical-session.target\n"
    ))
}

pub fn set_enabled(enabled: bool) -> Result<()> {
    let base = directories::BaseDirs::new().context("No home directory")?;
    let executable = std::env::current_exe()?;
    let desktop = base
        .config_dir()
        .join("autostart/io.github.lyricglass.LyricGlass.desktop");
    let service = base.config_dir().join("systemd/user/lyricglass.service");
    if enabled {
        std::fs::create_dir_all(desktop.parent().unwrap())?;
        std::fs::create_dir_all(service.parent().unwrap())?;
        let contents = unit(&executable)?;
        if let Ok(old) = std::fs::read(&service)
            && old != contents.as_bytes()
            && !service.with_extension("service.backup").exists()
        {
            std::fs::write(service.with_extension("service.backup"), old)?;
        }
        std::fs::write(&service, contents)?;
        std::fs::write(
            &desktop,
            format!(
                "[Desktop Entry]\nType=Application\nName=LyricGlass\nComment=Music and lyrics at login\nExec={} autostart-run\nIcon=audio-x-generic\nTerminal=false\nX-GNOME-Autostart-enabled=true\n",
                desktop_exec(&executable)?
            ),
        )?;
        if systemd_available() {
            run_systemctl(&["daemon-reload"])?;
            run_systemctl(&["enable", "lyricglass.service"])?;
        }
    } else {
        if desktop.exists() {
            std::fs::remove_file(desktop)?;
        }
        if systemd_available() {
            run_systemctl(&["disable", "lyricglass.service"])?;
        }
    }
    Ok(())
}

pub fn run() -> Result<()> {
    if !crate::config::Config::load().autostart {
        return Ok(());
    }
    if systemd_available() {
        let variables: Vec<&str> = [
            "DISPLAY",
            "WAYLAND_DISPLAY",
            "XAUTHORITY",
            "XDG_CURRENT_DESKTOP",
            "XDG_SESSION_TYPE",
        ]
        .into_iter()
        .filter(|v| std::env::var_os(v).is_some())
        .collect();
        let mut args = vec!["import-environment"];
        args.extend(variables);
        if run_systemctl(&args).is_ok() && run_systemctl(&["start", "lyricglass.service"]).is_ok() {
            return Ok(());
        }
    }
    Command::new(std::env::current_exe()?).spawn()?;
    Ok(())
}

pub fn notify_config(config: &crate::config::Config) -> Result<()> {
    if std::env::var_os("DBUS_SESSION_BUS_ADDRESS").is_none() {
        return Ok(());
    }
    let app = gio::Application::new(
        Some("io.github.lyricglass.LyricGlass"),
        gio::ApplicationFlags::empty(),
    );
    app.register(None::<&gio::Cancellable>)?;
    if app.is_remote() {
        let json = serde_json::to_string(config)?;
        app.activate_action("apply-config", Some(&json.to_variant()));
        if let Some(connection) = app.dbus_connection() {
            connection.flush_sync(None::<&gio::Cancellable>)?;
        }
    }
    Ok(())
}

fn systemd_available() -> bool {
    Command::new("systemctl")
        .args(["--user", "show-environment"])
        .output()
        .is_ok_and(|o| o.status.success())
}
fn run_systemctl(args: &[&str]) -> Result<()> {
    let output = Command::new("systemctl")
        .arg("--user")
        .args(args)
        .output()?;
    ensure!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quotes_executable_paths_without_shell_interpolation() {
        assert_eq!(
            desktop_exec(Path::new("/home/a b/lyricglass")).unwrap(),
            "\"/home/a b/lyricglass\""
        );
        assert!(desktop_exec(Path::new("/bad\npath")).is_err());
        assert!(
            unit(Path::new("/home/100%/$app"))
                .unwrap()
                .contains("100%%/$$app")
        );
        assert!(
            !unit(Path::new("/usr/bin/lyricglass"))
                .unwrap()
                .contains("default.target")
        );
    }
}

use std::path::Path;
use std::process::Command;

pub fn apply_wallpaper(backend: &str, enabled: bool, image: &Path) -> Result<String, String> {
    if !enabled {
        return Ok("disabled".to_string());
    }

    let selected = select_backend(backend);
    match selected.as_str() {
        "noop" => Ok("noop".to_string()),
        "gnome" => apply_gnome_wallpaper(image).map(|_| "gnome".to_string()),
        "sway" => apply_sway_wallpaper(image).map(|_| "sway".to_string()),
        "feh" => apply_feh_wallpaper(image).map(|_| "feh".to_string()),
        other => Err(format!("unsupported wallpaper backend: {other}")),
    }
}

fn select_backend(requested: &str) -> String {
    let requested = requested.trim().to_ascii_lowercase();
    if requested != "auto" {
        return requested;
    }
    detect_backend()
}

fn detect_backend() -> String {
    let desktop = std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .to_ascii_lowercase();

    if std::env::var_os("SWAYSOCK").is_some() && has_command("swaymsg") {
        return "sway".to_string();
    }
    if desktop.contains("gnome") && has_command("gsettings") {
        return "gnome".to_string();
    }
    if has_command("feh") {
        return "feh".to_string();
    }
    "noop".to_string()
}

fn has_command(cmd: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {cmd} >/dev/null 2>&1"))
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn apply_gnome_wallpaper(image: &Path) -> Result<(), String> {
    let uri = format!("file://{}", image.display());
    run_cmd(
        "gsettings",
        &["set", "org.gnome.desktop.background", "picture-uri", &uri],
    )?;
    run_cmd(
        "gsettings",
        &[
            "set",
            "org.gnome.desktop.background",
            "picture-uri-dark",
            &uri,
        ],
    )?;
    Ok(())
}

fn apply_sway_wallpaper(image: &Path) -> Result<(), String> {
    let img = image.display().to_string();
    run_cmd("swaymsg", &["output", "*", "bg", &img, "fill"])
}

fn apply_feh_wallpaper(image: &Path) -> Result<(), String> {
    let img = image.display().to_string();
    run_cmd("feh", &["--bg-fill", &img])
}

fn run_cmd(cmd: &str, args: &[&str]) -> Result<(), String> {
    let status = Command::new(cmd)
        .args(args)
        .status()
        .map_err(|e| format!("failed to run {cmd}: {e}"))?;
    if !status.success() {
        return Err(format!("command failed: {} {}", cmd, args.join(" ")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::apply_wallpaper;
    use std::path::Path;

    #[test]
    fn disabled_mode_returns_status() {
        let status = apply_wallpaper("auto", false, Path::new("/tmp/demo.png"))
            .expect("disabled should be a successful no-op");
        assert_eq!(status, "disabled");
    }
}

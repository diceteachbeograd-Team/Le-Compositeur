use std::path::Path;
use std::process::Command;

pub fn apply_wallpaper(
    backend: &str,
    fit_mode: &str,
    enabled: bool,
    image: &Path,
) -> Result<String, String> {
    if !enabled {
        return Ok("disabled".to_string());
    }

    let selected = select_backend(backend);
    match selected.as_str() {
        "noop" => Ok("noop".to_string()),
        "macos" => apply_macos_wallpaper(image).map(|_| "macos".to_string()),
        "windows" => apply_windows_wallpaper(image, fit_mode).map(|_| "windows".to_string()),
        "gnome" => apply_gnome_wallpaper(image, fit_mode).map(|_| "gnome".to_string()),
        "sway" => apply_sway_wallpaper(image, fit_mode).map(|_| "sway".to_string()),
        "feh" => apply_feh_wallpaper(image, fit_mode).map(|_| "feh".to_string()),
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
    #[cfg(target_os = "macos")]
    {
        if has_command("osascript") {
            return "macos".to_string();
        }
        "noop".to_string()
    }

    #[cfg(target_os = "windows")]
    {
        if has_command("powershell") {
            return "windows".to_string();
        }
        if has_command("pwsh") {
            return "windows".to_string();
        }
        return "noop".to_string();
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
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
}

#[cfg(windows)]
fn has_command(cmd: &str) -> bool {
    Command::new("where")
        .arg(cmd)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(not(windows))]
fn has_command(cmd: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {cmd} >/dev/null 2>&1"))
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn apply_macos_wallpaper(image: &Path) -> Result<(), String> {
    if !image.exists() {
        return Err(format!(
            "macos wallpaper target does not exist: {}",
            image.display()
        ));
    }
    let img = image.display().to_string();
    run_cmd(
        "osascript",
        &[
            "-e",
            r#"on run argv
set imagePath to POSIX file (item 1 of argv)
tell application "System Events"
    tell every desktop
        set picture to imagePath
    end tell
end tell
end run"#,
            &img,
        ],
    )
    .map_err(annotate_macos_wallpaper_error)
}

fn apply_windows_wallpaper(image: &Path, fit_mode: &str) -> Result<(), String> {
    if !image.exists() {
        return Err(format!(
            "windows wallpaper target does not exist: {}",
            image.display()
        ));
    }
    let img = image
        .canonicalize()
        .unwrap_or_else(|_| image.to_path_buf())
        .display()
        .to_string();
    let (style, tile) = normalize_windows_fit_mode(fit_mode);
    run_cmd(
        windows_shell(),
        &[
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            r#"$path = [System.IO.Path]::GetFullPath($args[0]);
$style = $args[1];
$tile = $args[2];
Set-ItemProperty -Path 'HKCU:\Control Panel\Desktop' -Name WallpaperStyle -Value $style;
Set-ItemProperty -Path 'HKCU:\Control Panel\Desktop' -Name TileWallpaper -Value $tile;
Add-Type -TypeDefinition 'using System.Runtime.InteropServices; public class NativeMethods { [DllImport("user32.dll", SetLastError=true, CharSet=CharSet.Unicode)] public static extern bool SystemParametersInfo(int uAction, int uParam, string lpvParam, int fuWinIni); }';
if (-not [NativeMethods]::SystemParametersInfo(20, 0, $path, 3)) { throw 'SystemParametersInfo failed' }"#,
            &img,
            style,
            tile,
        ],
    )
}

fn windows_shell() -> &'static str {
    if has_command("powershell") {
        "powershell"
    } else {
        "pwsh"
    }
}

fn apply_gnome_wallpaper(image: &Path, fit_mode: &str) -> Result<(), String> {
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
    run_cmd(
        "gsettings",
        &[
            "set",
            "org.gnome.desktop.background",
            "picture-options",
            normalize_gnome_fit_mode(fit_mode),
        ],
    )?;
    Ok(())
}

fn apply_sway_wallpaper(image: &Path, fit_mode: &str) -> Result<(), String> {
    let img = image.display().to_string();
    run_cmd(
        "swaymsg",
        &["output", "*", "bg", &img, normalize_sway_fit_mode(fit_mode)],
    )
}

fn apply_feh_wallpaper(image: &Path, fit_mode: &str) -> Result<(), String> {
    let img = image.display().to_string();
    run_cmd("feh", &[normalize_feh_fit_mode(fit_mode), &img])
}

fn normalize_gnome_fit_mode(mode: &str) -> &'static str {
    match mode.trim().to_ascii_lowercase().as_str() {
        "scaled" => "scaled",
        "stretched" => "stretched",
        "spanned" => "spanned",
        "centered" => "centered",
        "wallpaper" => "wallpaper",
        _ => "zoom",
    }
}

fn normalize_sway_fit_mode(mode: &str) -> &'static str {
    match mode.trim().to_ascii_lowercase().as_str() {
        "scaled" => "fit",
        "stretched" => "stretch",
        "centered" => "center",
        "tiled" | "wallpaper" => "tile",
        _ => "fill",
    }
}

fn normalize_feh_fit_mode(mode: &str) -> &'static str {
    match mode.trim().to_ascii_lowercase().as_str() {
        "scaled" => "--bg-max",
        "stretched" => "--bg-scale",
        "centered" => "--bg-center",
        "tiled" | "wallpaper" => "--bg-tile",
        _ => "--bg-fill",
    }
}

fn normalize_windows_fit_mode(mode: &str) -> (&'static str, &'static str) {
    match mode.trim().to_ascii_lowercase().as_str() {
        "scaled" => ("6", "0"),
        "stretched" => ("2", "0"),
        "spanned" => ("22", "0"),
        "centered" => ("0", "0"),
        "wallpaper" | "tiled" => ("0", "1"),
        _ => ("10", "0"),
    }
}

fn run_cmd(cmd: &str, args: &[&str]) -> Result<(), String> {
    let output = Command::new(cmd)
        .args(args)
        .output()
        .map_err(|e| format!("failed to run {cmd}: {e}"))?;
    if !output.status.success() {
        let stderr = trim_multiline(&String::from_utf8_lossy(&output.stderr), 5);
        let stdout = trim_multiline(&String::from_utf8_lossy(&output.stdout), 3);
        let mut msg = format!("command failed: {} {}", cmd, args.join(" "));
        if !stderr.is_empty() {
            msg.push_str(&format!("\nstderr:\n{stderr}"));
        }
        if !stdout.is_empty() {
            msg.push_str(&format!("\nstdout:\n{stdout}"));
        }
        return Err(msg);
    }
    Ok(())
}

fn annotate_macos_wallpaper_error(err: String) -> String {
    let lower = err.to_ascii_lowercase();
    let looks_like_automation_denied = lower.contains("not authorized to send apple events")
        || lower.contains("(-1743)")
        || lower.contains("event not permitted")
        || lower.contains("osascript is not allowed");
    if !looks_like_automation_denied {
        return err;
    }
    format!(
        "{err}\n\nmacOS permission hint:\nAllow automation access for Le Compositeur (or Terminal/wc-cli) to control \"System Events\".\nPath: System Settings -> Privacy & Security -> Automation.",
    )
}

fn trim_multiline(raw: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = raw
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    if lines.is_empty() {
        return String::new();
    }
    if lines.len() <= max_lines {
        return lines.join("\n");
    }
    let start = lines.len().saturating_sub(max_lines);
    format!("...\n{}", lines[start..].join("\n"))
}

#[cfg(test)]
mod tests {
    use super::{annotate_macos_wallpaper_error, apply_wallpaper, normalize_windows_fit_mode};
    use std::path::Path;

    #[test]
    fn disabled_mode_returns_status() {
        let status = apply_wallpaper("auto", "zoom", false, Path::new("/tmp/demo.png"))
            .expect("disabled should be a successful no-op");
        assert_eq!(status, "disabled");
    }

    #[test]
    fn windows_fit_mode_mapping_is_stable() {
        assert_eq!(normalize_windows_fit_mode("zoom"), ("10", "0"));
        assert_eq!(normalize_windows_fit_mode("scaled"), ("6", "0"));
        assert_eq!(normalize_windows_fit_mode("stretched"), ("2", "0"));
        assert_eq!(normalize_windows_fit_mode("spanned"), ("22", "0"));
        assert_eq!(normalize_windows_fit_mode("centered"), ("0", "0"));
        assert_eq!(normalize_windows_fit_mode("wallpaper"), ("0", "1"));
        assert_eq!(normalize_windows_fit_mode("tiled"), ("0", "1"));
    }

    #[test]
    fn macos_permission_errors_get_actionable_hint() {
        let base = "execution error: Not authorized to send Apple events to System Events. (-1743)";
        let msg = annotate_macos_wallpaper_error(base.to_string());
        assert!(msg.contains("Privacy & Security -> Automation"));
        assert!(msg.contains("System Events"));
    }
}

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
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

pub fn save_wallpaper_state(backend: &str, state_path: &Path) -> Result<String, String> {
    let selected = select_backend(backend);
    if selected == "noop" || selected == "sway" || selected == "feh" {
        return Ok(format!("skipped:{selected}"));
    }

    let mut state = BTreeMap::<String, String>::new();
    state.insert("version".to_string(), "1".to_string());
    state.insert("backend".to_string(), selected.clone());
    match selected.as_str() {
        "macos" => capture_macos_wallpaper_state(&mut state)?,
        "windows" => capture_windows_wallpaper_state(&mut state)?,
        "gnome" => capture_gnome_wallpaper_state(&mut state)?,
        other => {
            return Err(format!(
                "unsupported wallpaper backend for state save: {other}"
            ));
        }
    }

    write_state_file(state_path, &state)?;
    Ok(selected)
}

pub fn restore_wallpaper_state(state_path: &Path) -> Result<String, String> {
    let state = read_state_file(state_path)?;
    let backend = state
        .get("backend")
        .map(|s| s.trim().to_ascii_lowercase())
        .ok_or_else(|| "wallpaper state missing backend field".to_string())?;

    match backend.as_str() {
        "macos" => restore_macos_wallpaper_state(&state)?,
        "windows" => restore_windows_wallpaper_state(&state)?,
        "gnome" => restore_gnome_wallpaper_state(&state)?,
        "noop" | "sway" | "feh" => {}
        other => {
            return Err(format!(
                "unsupported wallpaper backend for state restore: {other}"
            ));
        }
    }
    Ok(backend)
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

fn capture_macos_wallpaper_state(state: &mut BTreeMap<String, String>) -> Result<(), String> {
    let out = run_cmd_capture(
        "osascript",
        &[
            "-e",
            r#"tell application "System Events"
set currentPicture to picture of desktop 1
return POSIX path of currentPicture
end tell"#,
        ],
    )
    .map_err(annotate_macos_wallpaper_error)?;
    let picture = out.trim();
    if picture.is_empty() {
        return Err("macos wallpaper state capture returned empty picture path".to_string());
    }
    state.insert("picture".to_string(), picture.to_string());
    Ok(())
}

fn restore_macos_wallpaper_state(state: &BTreeMap<String, String>) -> Result<(), String> {
    let picture = state
        .get("picture")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "macos wallpaper state missing picture".to_string())?;
    let picture_path = PathBuf::from(picture);
    apply_macos_wallpaper(&picture_path)
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

fn capture_windows_wallpaper_state(state: &mut BTreeMap<String, String>) -> Result<(), String> {
    let out = run_cmd_capture(
        windows_shell(),
        &[
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            r#"$desktop = Get-ItemProperty -Path 'HKCU:\Control Panel\Desktop';
$wallpaper = [string]$desktop.Wallpaper;
$style = [string]$desktop.WallpaperStyle;
$tile = [string]$desktop.TileWallpaper;
Write-Output ("wallpaper=" + $wallpaper);
Write-Output ("wallpaper_style=" + $style);
Write-Output ("tile_wallpaper=" + $tile);"#,
        ],
    )?;
    for line in out.lines() {
        if let Some((key, value)) = line.split_once('=') {
            state.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    if state.get("wallpaper").is_none_or(|v| v.is_empty()) {
        return Err("windows wallpaper state missing wallpaper path".to_string());
    }
    Ok(())
}

fn restore_windows_wallpaper_state(state: &BTreeMap<String, String>) -> Result<(), String> {
    let wallpaper = state
        .get("wallpaper")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "windows wallpaper state missing wallpaper path".to_string())?;
    let style = state
        .get("wallpaper_style")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .unwrap_or("10");
    let tile = state
        .get("tile_wallpaper")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .unwrap_or("0");
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
            wallpaper,
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

fn capture_gnome_wallpaper_state(state: &mut BTreeMap<String, String>) -> Result<(), String> {
    let picture_uri = run_cmd_capture(
        "gsettings",
        &["get", "org.gnome.desktop.background", "picture-uri"],
    )?;
    let picture_uri_dark = run_cmd_capture(
        "gsettings",
        &["get", "org.gnome.desktop.background", "picture-uri-dark"],
    )?;
    let picture_options = run_cmd_capture(
        "gsettings",
        &["get", "org.gnome.desktop.background", "picture-options"],
    )?;
    state.insert(
        "picture_uri".to_string(),
        strip_quoted_gsettings_value(picture_uri.trim()).to_string(),
    );
    state.insert(
        "picture_uri_dark".to_string(),
        strip_quoted_gsettings_value(picture_uri_dark.trim()).to_string(),
    );
    state.insert(
        "picture_options".to_string(),
        strip_quoted_gsettings_value(picture_options.trim()).to_string(),
    );
    Ok(())
}

fn restore_gnome_wallpaper_state(state: &BTreeMap<String, String>) -> Result<(), String> {
    let picture_uri = state
        .get("picture_uri")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "gnome wallpaper state missing picture_uri".to_string())?;
    run_cmd(
        "gsettings",
        &[
            "set",
            "org.gnome.desktop.background",
            "picture-uri",
            picture_uri,
        ],
    )?;

    let picture_uri_dark = state
        .get("picture_uri_dark")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .unwrap_or(picture_uri);
    run_cmd(
        "gsettings",
        &[
            "set",
            "org.gnome.desktop.background",
            "picture-uri-dark",
            picture_uri_dark,
        ],
    )?;

    let picture_options = state
        .get("picture_options")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .unwrap_or("zoom");
    run_cmd(
        "gsettings",
        &[
            "set",
            "org.gnome.desktop.background",
            "picture-options",
            picture_options,
        ],
    )
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
    run_cmd_with_output(cmd, args).map(|_| ())
}

fn run_cmd_capture(cmd: &str, args: &[&str]) -> Result<String, String> {
    let output = run_cmd_with_output(cmd, args)?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn run_cmd_with_output(cmd: &str, args: &[&str]) -> Result<std::process::Output, String> {
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
    Ok(output)
}

fn write_state_file(path: &Path, state: &BTreeMap<String, String>) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create state dir {}: {err}", parent.display()))?;
    }
    let mut content = String::new();
    for (key, value) in state {
        content.push_str(key);
        content.push('=');
        content.push_str(&value.replace('\n', " "));
        content.push('\n');
    }
    fs::write(path, content)
        .map_err(|err| format!("failed to write wallpaper state {}: {err}", path.display()))
}

fn read_state_file(path: &Path) -> Result<BTreeMap<String, String>, String> {
    let raw = fs::read_to_string(path)
        .map_err(|err| format!("failed to read wallpaper state {}: {err}", path.display()))?;
    let mut state = BTreeMap::<String, String>::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = trimmed.split_once('=') {
            state.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    if state.is_empty() {
        return Err(format!(
            "wallpaper state file {} is empty or invalid",
            path.display()
        ));
    }
    Ok(state)
}

fn strip_quoted_gsettings_value(raw: &str) -> &str {
    raw.strip_prefix('\'')
        .and_then(|s| s.strip_suffix('\''))
        .unwrap_or(raw)
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
    use super::{
        annotate_macos_wallpaper_error, apply_wallpaper, normalize_windows_fit_mode,
        strip_quoted_gsettings_value,
    };
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

    #[test]
    fn gsettings_quote_strip_is_stable() {
        assert_eq!(
            strip_quoted_gsettings_value("'file:///tmp/demo.png'"),
            "file:///tmp/demo.png"
        );
        assert_eq!(strip_quoted_gsettings_value("zoom"), "zoom");
    }
}

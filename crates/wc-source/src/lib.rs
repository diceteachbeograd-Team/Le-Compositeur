use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy)]
pub enum ImageProvider {
    Generic,
    NasaApod,
    WikimediaApi,
    WallhavenApi,
}

#[derive(Debug, Clone, Copy)]
pub enum QuoteProvider {
    Generic,
    ZenQuotes,
    Quotable,
}

pub fn fetch_remote_image(endpoint: String, provider: ImageProvider) -> Result<PathBuf, String> {
    ensure_curl_available()?;
    if matches!(provider, ImageProvider::Generic)
        && let Some(path) = try_direct_image_download(&endpoint)?
    {
        return Ok(path);
    }

    let body = match fetch_text_via_curl(&endpoint) {
        Ok(body) => body,
        Err(err) => {
            if let Some(path) = fallback_image_for_endpoint(&endpoint)
                .and_then(|fallback| try_direct_image_download(fallback).ok().flatten())
            {
                return Ok(path);
            }
            return Err(err);
        }
    };

    let image_url = match provider {
        ImageProvider::NasaApod => parse_nasa_apod_image_url(&body)
            .or_else(|| fallback_image_for_endpoint(&endpoint).map(ToOwned::to_owned))
            .unwrap_or_else(|| endpoint.clone()),
        ImageProvider::WikimediaApi => parse_wikimedia_image_url(&body)
            .or_else(|| extract_image_url(&body))
            .or_else(|| extract_image_url(&unescape_json_like(&body)))
            .or_else(|| fallback_image_for_endpoint(&endpoint).map(ToOwned::to_owned))
            .unwrap_or_else(|| endpoint.clone()),
        ImageProvider::WallhavenApi => parse_wallhaven_image_url(&body)
            .or_else(|| extract_image_url(&body))
            .or_else(|| extract_image_url(&unescape_json_like(&body)))
            .or_else(|| fallback_image_for_endpoint(&endpoint).map(ToOwned::to_owned))
            .unwrap_or_else(|| endpoint.clone()),
        ImageProvider::Generic => extract_image_url(&body)
            .or_else(|| extract_image_url(&unescape_json_like(&body)))
            .or_else(|| fallback_image_for_endpoint(&endpoint).map(ToOwned::to_owned))
            .unwrap_or_else(|| endpoint.clone()),
    };

    let ext = guess_image_extension(&image_url);
    let cache_dir = cache_dir_for("images")?;
    let file_name = format!("remote-{}.{}", stable_hash(&image_url), ext);
    let target = cache_dir.join(file_name);
    if download_file_via_curl(&image_url, &target).is_err()
        && let Some(fallback) = fallback_image_for_endpoint(&endpoint)
    {
        download_file_via_curl(fallback, &target)?;
    }
    Ok(target)
}

fn try_direct_image_download(endpoint: &str) -> Result<Option<PathBuf>, String> {
    let ext = guess_image_extension(endpoint);
    let cache_dir = cache_dir_for("images")?;
    let file_name = format!("direct-{}.{}", stable_hash(endpoint), ext);
    let target = cache_dir.join(file_name);

    if download_file_via_curl(endpoint, &target).is_err() {
        return Ok(None);
    }

    if looks_like_image_file(&target)? {
        return Ok(Some(target));
    }

    let _ = fs::remove_file(&target);
    Ok(None)
}

pub fn fetch_remote_quote(endpoint: String, provider: QuoteProvider) -> Result<String, String> {
    ensure_curl_available()?;
    let body = fetch_text_via_curl(&endpoint)?;

    let quote = match provider {
        QuoteProvider::ZenQuotes => parse_zenquotes_payload(&body),
        QuoteProvider::Quotable => parse_quotable_payload(&body),
        QuoteProvider::Generic => parse_quote_from_payload(&body),
    };

    Ok(quote)
}

pub fn has_command(cmd: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {cmd} >/dev/null 2>&1"))
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn ensure_curl_available() -> Result<(), String> {
    if has_command("curl") {
        Ok(())
    } else {
        Err("curl is required for remote sources but is not available in PATH".to_string())
    }
}

fn fetch_text_via_curl(url: &str) -> Result<String, String> {
    let output = Command::new("curl")
        .args([
            "-fsSL",
            "--connect-timeout",
            "8",
            "--max-time",
            "25",
            "-H",
            "Cache-Control: no-cache",
            "-H",
            "Pragma: no-cache",
            url,
        ])
        .output()
        .map_err(|e| format!("failed to run curl: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "curl failed for {}: {}",
            url,
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    String::from_utf8(output.stdout).map_err(|e| format!("invalid UTF-8 from curl output: {e}"))
}

fn download_file_via_curl(url: &str, target: &Path) -> Result<(), String> {
    let status = Command::new("curl")
        .args([
            "-fsSL",
            "--connect-timeout",
            "8",
            "--max-time",
            "30",
            "-H",
            "Cache-Control: no-cache",
            "-H",
            "Pragma: no-cache",
            "-o",
        ])
        .arg(target)
        .arg(url)
        .status()
        .map_err(|e| format!("failed to run curl download: {e}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("curl download failed for {url}"))
    }
}

fn parse_nasa_apod_image_url(payload: &str) -> Option<String> {
    let media_type = json_field(payload, "media_type")?;
    if !media_type.trim().eq_ignore_ascii_case("image") {
        return None;
    }
    json_field(payload, "hdurl").or_else(|| json_field(payload, "url"))
}

fn parse_wikimedia_image_url(payload: &str) -> Option<String> {
    if let Some(url) = json_field(payload, "thumburl")
        && is_supported_image_url(&url)
    {
        return Some(url);
    }
    if let Some(url) = json_field(payload, "url")
        && is_supported_image_url(&url)
    {
        return Some(url);
    }
    extract_image_url(payload).map(|u| normalize_wikimedia_thumb_url(&u))
}

fn parse_wallhaven_image_url(payload: &str) -> Option<String> {
    if let Some(path) = json_field(payload, "path")
        && is_supported_image_url(&path)
    {
        return Some(path);
    }
    if let Some(url) = json_field(payload, "url")
        && is_supported_image_url(&url)
    {
        return Some(url);
    }
    None
}

fn normalize_wikimedia_thumb_url(url: &str) -> String {
    if !url.contains("upload.wikimedia.org") || !url.contains("/thumb/") {
        return url.to_string();
    }
    let without_thumb = url.replacen("/thumb/", "/", 1);
    if let Some(idx) = without_thumb.rfind('/') {
        let tail = &without_thumb[idx + 1..];
        if let Some(name_idx) = tail.find('.') {
            return format!("{}{}", &without_thumb[..idx + 1], &tail[name_idx + 1..]);
        }
    }
    without_thumb
}

fn parse_zenquotes_payload(payload: &str) -> String {
    let quote = json_field(payload, "q");
    let author = json_field(payload, "a");

    match (quote, author) {
        (Some(q), Some(a)) => format!("{} - {}", unescape_json_like(&q), unescape_json_like(&a)),
        (Some(q), None) => unescape_json_like(&q),
        _ => parse_quote_from_payload(payload),
    }
}

fn parse_quotable_payload(payload: &str) -> String {
    let content = json_field(payload, "content");
    let author = json_field(payload, "author");

    match (content, author) {
        (Some(c), Some(a)) => format!("{} - {}", unescape_json_like(&c), unescape_json_like(&a)),
        (Some(c), None) => unescape_json_like(&c),
        _ => parse_quote_from_payload(payload),
    }
}

fn extract_image_url(payload: &str) -> Option<String> {
    extract_urls(payload)
        .into_iter()
        .find(|url| is_supported_image_url(url))
}

fn fallback_image_for_endpoint(endpoint: &str) -> Option<&'static str> {
    if endpoint.contains("source.unsplash.com")
        || endpoint.contains("api.nasa.gov")
        || endpoint.contains("wallhaven.cc/api/")
    {
        return Some("https://picsum.photos/3840/2160.jpg");
    }
    None
}

fn extract_urls(payload: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = payload;

    while let Some(idx) = rest.find("https://") {
        let candidate = &rest[idx..];
        let end = candidate
            .find(|c: char| ['"', '\\', '\'', ' ', '\n', '\r', '\t', ')'].contains(&c))
            .unwrap_or(candidate.len());
        let url = &candidate[..end];
        if url.len() > "https://".len() {
            out.push(url.to_string());
        }
        rest = &candidate[end..];
    }

    out
}

fn is_supported_image_url(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    [".jpg", ".jpeg", ".png", ".webp", ".bmp"]
        .iter()
        .any(|ext| lower.contains(ext))
}

fn guess_image_extension(url: &str) -> &'static str {
    let lower = url.to_ascii_lowercase();
    if lower.contains(".png") {
        return "png";
    }
    if lower.contains(".webp") {
        return "webp";
    }
    if lower.contains(".bmp") {
        return "bmp";
    }
    if lower.contains(".jpeg") {
        return "jpeg";
    }
    "jpg"
}

fn looks_like_image_file(path: &Path) -> Result<bool, String> {
    let bytes = fs::read(path).map_err(|e| format!("failed to inspect downloaded file: {e}"))?;
    if bytes.len() < 12 {
        return Ok(false);
    }

    let is_jpeg = bytes[0] == 0xFF && bytes[1] == 0xD8;
    let is_png = bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
    let is_bmp = bytes.starts_with(b"BM");
    let is_gif = bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a");
    let is_webp = bytes.starts_with(b"RIFF") && bytes[8..12] == *b"WEBP";
    Ok(is_jpeg || is_png || is_bmp || is_gif || is_webp)
}

fn parse_quote_from_payload(payload: &str) -> String {
    if let Some(q) = json_field(payload, "q") {
        if let Some(a) = json_field(payload, "a") {
            return format!("{} - {}", unescape_json_like(&q), unescape_json_like(&a));
        }
        return unescape_json_like(&q);
    }

    if let Some(content) = json_field(payload, "content") {
        if let Some(author) = json_field(payload, "author") {
            return format!(
                "{} - {}",
                unescape_json_like(&content),
                unescape_json_like(&author)
            );
        }
        return unescape_json_like(&content);
    }

    if let Some(quote) = json_field(payload, "quote") {
        if let Some(author) = json_field(payload, "author") {
            return format!(
                "{} - {}",
                unescape_json_like(&quote),
                unescape_json_like(&author)
            );
        }
        return unescape_json_like(&quote);
    }

    if let Some(advice) = json_field(payload, "advice") {
        return unescape_json_like(&advice);
    }

    if let Some(text) = json_field(payload, "text") {
        return unescape_json_like(&text);
    }

    payload
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("Stay focused. Build step by step.")
        .to_string()
}

fn json_field(payload: &str, field: &str) -> Option<String> {
    let key = format!("\"{}\"", field);
    let idx = payload.find(&key)?;
    let after_key = &payload[idx + key.len()..];
    let colon = after_key.find(':')?;
    let after_colon = after_key[colon + 1..].trim_start();
    if !after_colon.starts_with('"') {
        return None;
    }

    let chars = after_colon[1..].chars();
    let mut value = String::new();
    let mut escaped = false;
    for ch in chars {
        if escaped {
            value.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == '"' {
            break;
        }
        value.push(ch);
    }
    Some(value)
}

fn unescape_json_like(input: &str) -> String {
    input
        .replace("\\n", " ")
        .replace("\\\"", "\"")
        .replace("\\/", "/")
        .trim()
        .to_string()
}

fn cache_dir_for(kind: &str) -> Result<PathBuf, String> {
    let mut candidates = Vec::new();
    if let Some(xdg) = std::env::var_os("XDG_CACHE_HOME") {
        candidates.push(PathBuf::from(xdg).join("wallpaper-composer").join(kind));
    }
    if let Some(home) = std::env::var_os("HOME") {
        candidates.push(
            PathBuf::from(home)
                .join(".cache")
                .join("wallpaper-composer")
                .join(kind),
        );
    }
    candidates.push(std::env::temp_dir().join("wallpaper-composer").join(kind));

    for dir in candidates {
        if fs::create_dir_all(&dir).is_ok() {
            return Ok(dir);
        }
    }

    Err("failed to create cache directory in XDG_CACHE_HOME, HOME, or /tmp".to_string())
}

fn stable_hash(input: &str) -> u64 {
    let mut h: u64 = 5381;
    for b in input.as_bytes() {
        h = ((h << 5).wrapping_add(h)).wrapping_add(*b as u64);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::{QuoteProvider, fetch_remote_quote, has_command, stable_hash};

    #[test]
    fn stable_hash_is_deterministic() {
        assert_eq!(stable_hash("abc"), stable_hash("abc"));
    }

    #[test]
    fn has_command_detects_shell() {
        assert!(has_command("sh"));
    }

    #[test]
    fn generic_quote_fallback_works_with_plain_text_url() {
        let path = std::env::temp_dir().join("wc-source-quote.txt");
        std::fs::write(&path, "Line one\nLine two\n").expect("temp quote file should be writable");
        let endpoint = format!("file://{}", path.display());
        let quote = fetch_remote_quote(endpoint, QuoteProvider::Generic)
            .expect("quote fetch should work via file://");
        assert_eq!(quote, "Line one");
        let _ = std::fs::remove_file(path);
    }
}

use once_cell::sync::Lazy;
use regex::Regex;

static SESSION_JSONL_TICKED: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"`~\/\.codex\/sessions\/[^`]*?\.jsonl`").expect("valid regex"));
static SESSION_JSONL_PLAIN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"~\/\.codex\/sessions\/\S*?\.jsonl").expect("valid regex"));
static LOOPBACK_URL: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"`?https?:\/\/(?:127\.0\.0\.1|localhost)(?::(?P<port>\d+))?(?:\/[^`\s]*)?`?")
        .expect("valid regex")
});
static LOOPBACK_HOST: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"`?(?:127\.0\.0\.1|localhost)(?::(?P<port>\d+))?`?").expect("valid regex")
});
static PRIVATE_URL: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"`?https?:\/\/(?:10(?:\.\d{1,3}){3}|192\.168(?:\.\d{1,3}){2}|172\.(?:1[6-9]|2\d|3[0-1])(?:\.\d{1,3}){2})(?::(?P<port>\d+))?(?:\/[^`\s]*)?`?")
        .expect("valid regex")
});
static PRIVATE_HOST: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"`?(?:10(?:\.\d{1,3}){3}|192\.168(?:\.\d{1,3}){2}|172\.(?:1[6-9]|2\d|3[0-1])(?:\.\d{1,3}){2})(?::(?P<port>\d+))?`?")
        .expect("valid regex")
});
static INLINE_CODE: Lazy<Regex> = Lazy::new(|| Regex::new(r"`([^`]+)`").expect("valid regex"));
static EXPLICIT_PATH: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(~|\./|\.\./|/)[^\s]*$").expect("valid regex"));

pub fn sanitize_mem9_content(value: &str) -> String {
    let text = SESSION_JSONL_TICKED
        .replace_all(value, "Codex session rollout JSONL file")
        .to_string();
    let text = SESSION_JSONL_PLAIN
        .replace_all(&text, "Codex session rollout JSONL file")
        .to_string();
    let text = LOOPBACK_URL
        .replace_all(&text, |caps: &regex::Captures| {
            replace_loopback(caps.name("port").map(|m| m.as_str()))
        })
        .to_string();
    let text = LOOPBACK_HOST
        .replace_all(&text, |caps: &regex::Captures| {
            replace_loopback(caps.name("port").map(|m| m.as_str()))
        })
        .to_string();
    let text = PRIVATE_URL
        .replace_all(&text, |caps: &regex::Captures| {
            replace_private(caps.name("port").map(|m| m.as_str()))
        })
        .to_string();
    let text = PRIVATE_HOST
        .replace_all(&text, |caps: &regex::Captures| {
            replace_private(caps.name("port").map(|m| m.as_str()))
        })
        .to_string();

    INLINE_CODE
        .replace_all(&text, |caps: &regex::Captures| {
            simplify_inline_code(caps.get(1).map(|m| m.as_str()).unwrap_or(""))
        })
        .to_string()
}

fn replace_loopback(port: Option<&str>) -> String {
    match port {
        Some(port) => format!("loopback address (port {port})"),
        None => "loopback address".to_string(),
    }
}

fn replace_private(port: Option<&str>) -> String {
    match port {
        Some(port) => format!("private network address (port {port})"),
        None => "private network address".to_string(),
    }
}

fn simplify_inline_code(code: &str) -> String {
    let lowered = code.to_lowercase();
    if lowered.contains("http://")
        || lowered.contains("https://")
        || lowered.contains("127.0.0.1")
        || lowered.contains("localhost")
    {
        return "related address".to_string();
    }
    if [
        "curl",
        "git",
        "npm",
        "npx",
        "pnpm",
        "node",
        "python",
        "codex",
        "gh ",
        "mvn",
        "launchctl",
        "@",
    ]
    .iter()
    .any(|token| lowered.contains(token))
    {
        return "related command".to_string();
    }
    if code.starts_with("--") {
        return "related command argument".to_string();
    }
    if EXPLICIT_PATH.is_match(code) {
        return "related path".to_string();
    }
    code.to_string()
}

#[cfg(test)]
mod tests {
    use super::sanitize_mem9_content;

    #[test]
    fn redacts_session_paths_and_loopback() {
        let text = "See `~/.codex/sessions/a/rollout-x.jsonl` and http://127.0.0.1:8081/api";
        let sanitized = sanitize_mem9_content(text);
        assert!(sanitized.contains("Codex session rollout JSONL file"));
        assert!(sanitized.contains("loopback address (port 8081)"));
    }

    #[test]
    fn redacts_private_networks() {
        let text = "The service listens on 192.168.64.1:5000";
        let sanitized = sanitize_mem9_content(text);
        assert!(sanitized.contains("private network address (port 5000)"));
    }

    #[test]
    fn keeps_non_path_inline_code_with_slashes() {
        let text = "Use `content/json` and `v1/api` as literal values.";
        let sanitized = sanitize_mem9_content(text);
        assert!(sanitized.contains("content/json"));
        assert!(sanitized.contains("v1/api"));
    }

    #[test]
    fn still_redacts_explicit_paths() {
        let text = "Read `~/project/file.txt` before continuing.";
        let sanitized = sanitize_mem9_content(text);
        assert!(sanitized.contains("related path"));
    }
}

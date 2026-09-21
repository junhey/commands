//! 提示符各模块的实现。
//!
//! git 信息优先读 `.git` 目录（零进程开销），仅在开启 `git.status_enabled`
//! 时调用一次 `git status --porcelain=v2 --branch`，结果 1 秒内复用。

use super::Context;
use crate::config::LangSpec;
use crate::style;
use crate::util::{self, EnvMap};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};

pub fn render_module(name: &str, ctx: &Context<'_>) -> Option<String> {
    match name {
        "line_break" => Some("\n".to_string()),
        "character" => Some(character(ctx)),
        "dir" | "directory" => Some(dir(ctx)),
        "git_branch" => git_branch(ctx),
        "git_status" => git_status(ctx),
        "languages" | "lang" => languages(ctx),
        "cmd_duration" => cmd_duration(ctx),
        "status" => status(ctx),
        "time" => time(ctx),
        "identity" => identity(ctx),
        "jobs" => jobs(ctx),
        "username" => {
            username(ctx).map(|u| format!("{} ", style::paint(&ctx.config.identity.style, &u)))
        }
        "hostname" => {
            hostname(ctx).map(|h| format!("{} ", style::paint(&ctx.config.identity.style, &h)))
        }
        other => ctx.config.lang(other).and_then(|spec| language(ctx, spec)),
    }
}

fn character(ctx: &Context<'_>) -> String {
    let cfg = &ctx.config.character;
    let (symbol, style_spec) = if ctx.status == 0 {
        (&cfg.success_symbol, &cfg.success_style)
    } else {
        (&cfg.error_symbol, &cfg.error_style)
    };
    format!("{} ", style::paint(style_spec, symbol))
}

fn dir(ctx: &Context<'_>) -> String {
    let cfg = &ctx.config.dir;
    let repo_root = if cfg.truncate_to_repo {
        git_root(ctx.cwd)
    } else {
        None
    };
    let display = match repo_root {
        Some(root) => {
            let name = root
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| util::contract_home(&root, &cfg.home_symbol));
            match ctx.cwd.strip_prefix(&root) {
                Ok(rest) if rest.as_os_str().is_empty() => name,
                Ok(rest) => format!("{name}/{}", rest.to_string_lossy().replace('\\', "/")),
                Err(_) => util::contract_home(ctx.cwd, &cfg.home_symbol),
            }
        }
        None => util::contract_home(ctx.cwd, &cfg.home_symbol),
    };
    let text = truncate_path(&display, cfg.truncation_length, &cfg.truncation_symbol);
    format!("{} ", style::paint(&cfg.style, &text))
}

fn truncate_path(path: &str, keep: usize, symbol: &str) -> String {
    if keep == 0 {
        return path.to_string();
    }
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if segments.len() <= keep {
        return path.to_string();
    }
    let tail = segments[segments.len() - keep..].join("/");
    format!("{symbol}{tail}")
}

fn git_branch(ctx: &Context<'_>) -> Option<String> {
    if !ctx.config.git.enabled {
        return None;
    }
    let root = git_root(ctx.cwd)?;
    let info = git_info(&root, ctx);
    let branch = info.branch?;
    let cfg = &ctx.config.git;
    let symbol = if info.detached {
        &cfg.detached_symbol
    } else {
        &cfg.branch_symbol
    };
    Some(format!(
        "on {} ",
        style::paint(&cfg.branch_style, &format!("{symbol}{branch}"))
    ))
}

fn git_status(ctx: &Context<'_>) -> Option<String> {
    let cfg = &ctx.config.git;
    if !cfg.enabled || !cfg.status_enabled {
        return None;
    }
    let root = git_root(ctx.cwd)?;
    let info = git_info(&root, ctx);
    let mut flags = String::new();
    if info.ahead > 0 {
        flags.push_str(&format!("⇡{}", info.ahead));
    }
    if info.behind > 0 {
        flags.push_str(&format!("⇣{}", info.behind));
    }
    if info.conflicted > 0 {
        flags.push_str(&format!("={}", info.conflicted));
    }
    if info.staged > 0 {
        flags.push_str(&format!("+{}", info.staged));
    }
    if info.modified > 0 {
        flags.push_str(&format!("!{}", info.modified));
    }
    if info.deleted > 0 {
        flags.push_str(&format!("✘{}", info.deleted));
    }
    if info.untracked > 0 {
        flags.push_str(&format!("?{}", info.untracked));
    }
    if flags.is_empty() {
        return None;
    }
    Some(format!("{} ", style::paint(&cfg.status_style, &flags)))
}

fn languages(ctx: &Context<'_>) -> Option<String> {
    if !ctx.config.languages.enabled {
        return None;
    }
    let mut out = String::new();
    for spec in &ctx.config.languages.entries {
        if let Some(text) = language(ctx, spec) {
            out.push_str(&text);
        }
    }
    (!out.is_empty()).then_some(out)
}

fn language(ctx: &Context<'_>, spec: &LangSpec) -> Option<String> {
    if !spec.enabled || !detect_language(ctx.cwd, spec) {
        return None;
    }
    let version = if ctx.config.languages.show_version {
        lang_version(spec, ctx.cwd, ctx.env)
    } else {
        None
    };
    let text = match version {
        Some(version) => format!("{}v{version}", spec.symbol),
        None => spec.symbol.trim_end().to_string(),
    };
    if text.is_empty() {
        return None;
    }
    Some(format!("{} ", style::paint(&spec.style, &text)))
}

fn detect_language(cwd: &Path, spec: &LangSpec) -> bool {
    let scan = scan_dir(cwd);
    if spec.files.iter().any(|f| scan.files.contains(f)) {
        return true;
    }
    spec.extensions
        .iter()
        .any(|ext| scan.extensions.contains(ext))
}

fn cmd_duration(ctx: &Context<'_>) -> Option<String> {
    let cfg = &ctx.config.cmd_duration;
    if !cfg.enabled || ctx.duration_ms < cfg.min_time_ms {
        return None;
    }
    let text = format!("{}{}", cfg.prefix, util::format_duration(ctx.duration_ms));
    Some(format!("{} ", style::paint(&cfg.style, &text)))
}

fn status(ctx: &Context<'_>) -> Option<String> {
    let cfg = &ctx.config.status;
    if !cfg.enabled || ctx.status == 0 {
        return None;
    }
    let text = format!("{}{}", cfg.symbol, ctx.status);
    Some(format!("{} ", style::paint(&cfg.style, &text)))
}

fn jobs(ctx: &Context<'_>) -> Option<String> {
    if ctx.jobs == 0 {
        return None;
    }
    Some(format!(
        "{} ",
        style::paint("bold blue", &format!("✦{}", ctx.jobs))
    ))
}

fn time(ctx: &Context<'_>) -> Option<String> {
    let cfg = &ctx.config.time;
    if !cfg.enabled {
        return None;
    }
    let secs = util::now_secs() as i64 + (cfg.utc_offset * 3600.0) as i64;
    let day = secs.rem_euclid(86_400);
    let (hour, minute, second) = (day / 3600, (day % 3600) / 60, day % 60);
    let text = if cfg.show_seconds {
        format!("{hour:02}:{minute:02}:{second:02}")
    } else {
        format!("{hour:02}:{minute:02}")
    };
    Some(format!("{} ", style::paint(&cfg.style, &text)))
}

fn identity(ctx: &Context<'_>) -> Option<String> {
    let cfg = &ctx.config.identity;
    let user = cfg.show_user.then(|| username(ctx)).flatten();
    let host = cfg.show_host.then(|| hostname(ctx)).flatten();
    let text = match (user, host) {
        (Some(user), Some(host)) => format!("{user}@{host}"),
        (Some(user), None) => user,
        (None, Some(host)) => host,
        (None, None) => return None,
    };
    Some(format!("{} ", style::paint(&cfg.style, &text)))
}

fn username(ctx: &Context<'_>) -> Option<String> {
    for key in ["USER", "USERNAME", "LOGNAME"] {
        if let Some(value) = util::env_get(ctx.env, key) {
            if !value.is_empty() {
                return Some(value.clone());
            }
        }
    }
    None
}

fn hostname(ctx: &Context<'_>) -> Option<String> {
    for key in ["HOSTNAME", "COMPUTERNAME"] {
        if let Some(value) = util::env_get(ctx.env, key) {
            if !value.is_empty() {
                return Some(value.clone());
            }
        }
    }
    std::fs::read_to_string("/etc/hostname")
        .ok()
        .map(|h| h.trim().to_string())
        .filter(|h| !h.is_empty())
}

// ── git ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
struct GitInfo {
    branch: Option<String>,
    detached: bool,
    ahead: u32,
    behind: u32,
    staged: u32,
    modified: u32,
    deleted: u32,
    untracked: u32,
    conflicted: u32,
}

/// 向上查找 git 仓库根目录。
pub fn git_root(start: &Path) -> Option<PathBuf> {
    let mut current = Some(start);
    while let Some(dir) = current {
        if dir.join(".git").exists() {
            return Some(dir.to_path_buf());
        }
        current = dir.parent();
    }
    None
}

fn git_dir(root: &Path) -> Option<PathBuf> {
    let candidate = root.join(".git");
    if candidate.is_dir() {
        return Some(candidate);
    }
    if candidate.is_file() {
        let content = std::fs::read_to_string(&candidate).ok()?;
        let path = PathBuf::from(content.trim().strip_prefix("gitdir:")?.trim());
        return Some(if path.is_absolute() {
            path
        } else {
            root.join(path)
        });
    }
    None
}

fn git_cache() -> &'static Mutex<HashMap<PathBuf, (u64, GitInfo)>> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, (u64, GitInfo)>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn git_info(root: &Path, ctx: &Context<'_>) -> GitInfo {
    let now = util::now_secs();
    if let Ok(cache) = git_cache().lock() {
        if let Some((stamp, info)) = cache.get(root) {
            if now.saturating_sub(*stamp) < 1 {
                return info.clone();
            }
        }
    }

    let mut info = GitInfo::default();
    if let Some((name, detached)) = read_head(root) {
        info.branch = Some(name);
        info.detached = detached;
    }
    if ctx.config.git.status_enabled {
        if let Some(text) = run_capture(
            "git",
            &["status", "--porcelain=v2", "--branch"],
            ctx.cwd,
            ctx.env,
            true,
        ) {
            parse_porcelain(&text, &mut info);
        }
    }

    if let Ok(mut cache) = git_cache().lock() {
        cache.insert(root.to_path_buf(), (now, info.clone()));
    }
    info
}

fn read_head(root: &Path) -> Option<(String, bool)> {
    let head = std::fs::read_to_string(git_dir(root)?.join("HEAD")).ok()?;
    let head = head.trim();
    match head.strip_prefix("ref: ") {
        Some(reference) => {
            let name = reference.rsplit('/').next().unwrap_or(reference);
            Some((name.to_string(), false))
        }
        None => Some((head.chars().take(7).collect(), true)),
    }
}

fn parse_porcelain(text: &str, info: &mut GitInfo) {
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("# branch.head ") {
            let rest = rest.trim();
            if rest == "(detached)" {
                info.detached = true;
            } else {
                info.branch = Some(rest.to_string());
                info.detached = false;
            }
        } else if let Some(rest) = line.strip_prefix("# branch.ab ") {
            for part in rest.split_whitespace() {
                if let Some(value) = part.strip_prefix('+') {
                    info.ahead = value.parse().unwrap_or(0);
                } else if let Some(value) = part.strip_prefix('-') {
                    info.behind = value.parse().unwrap_or(0);
                }
            }
        } else if line.starts_with("1 ") || line.starts_with("2 ") {
            if let Some(xy) = line.split_whitespace().nth(1) {
                let mut chars = xy.chars();
                let staged = chars.next().unwrap_or('.');
                let worktree = chars.next().unwrap_or('.');
                if staged != '.' {
                    if staged == 'D' {
                        info.deleted += 1;
                    } else {
                        info.staged += 1;
                    }
                }
                if worktree != '.' {
                    if worktree == 'D' {
                        info.deleted += 1;
                    } else {
                        info.modified += 1;
                    }
                }
            }
        } else if line.starts_with("? ") {
            info.untracked += 1;
        } else if line.starts_with("u ") {
            info.conflicted += 1;
        }
    }
}

// ── 目录扫描与语言版本 ──────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
struct DirScan {
    files: HashSet<String>,
    extensions: HashSet<String>,
}

fn scan_cache() -> &'static Mutex<HashMap<PathBuf, (u64, DirScan)>> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, (u64, DirScan)>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn scan_dir(cwd: &Path) -> DirScan {
    let now = util::now_secs();
    if let Ok(cache) = scan_cache().lock() {
        if let Some((stamp, scan)) = cache.get(cwd) {
            if now.saturating_sub(*stamp) < 2 {
                return scan.clone();
            }
        }
    }
    let mut scan = DirScan::default();
    if let Ok(entries) = std::fs::read_dir(cwd) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                scan.files.insert(name.to_string());
            }
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                scan.extensions.insert(ext.to_string());
            }
        }
    }
    if let Ok(mut cache) = scan_cache().lock() {
        cache.insert(cwd.to_path_buf(), (now, scan.clone()));
    }
    scan
}

fn version_cache() -> &'static Mutex<HashMap<String, Option<String>>> {
    static CACHE: OnceLock<Mutex<HashMap<String, Option<String>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn lang_version(spec: &LangSpec, cwd: &Path, env: &EnvMap) -> Option<String> {
    if spec.version_cmd.is_empty() {
        return None;
    }
    let key = spec.version_cmd.join(" ");
    if let Ok(cache) = version_cache().lock() {
        if let Some(cached) = cache.get(&key) {
            return cached.clone();
        }
    }

    let args: Vec<&str> = spec.version_cmd[1..].iter().map(String::as_str).collect();
    let mut output = run_capture(&spec.version_cmd[0], &args, cwd, env, false);
    if output.is_none() {
        // python3 / python 互为备选
        let fallback = match spec.version_cmd[0].as_str() {
            "python3" => Some("python"),
            "python" => Some("python3"),
            _ => None,
        };
        if let Some(program) = fallback {
            output = run_capture(program, &args, cwd, env, false);
        }
    }
    let version = output.as_deref().and_then(find_version);

    if let Ok(mut cache) = version_cache().lock() {
        cache.insert(key, version.clone());
    }
    version
}

/// 从命令输出里抽出第一个版本号，如 `rustc 1.95.0 (...)` -> `1.95.0`。
fn find_version(text: &str) -> Option<String> {
    for token in text.split(|c: char| c.is_whitespace() || c == '(' || c == ')' || c == '"') {
        // 允许 `go1.22.3`、`v24.15.0` 之类的前缀
        let Some(start) = token.find(|c: char| c.is_ascii_digit()) else {
            continue;
        };
        let cleaned: String = token[start..]
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '.')
            .collect();
        let cleaned = cleaned.trim_end_matches('.');
        if cleaned.contains('.') {
            return Some(cleaned.to_string());
        }
    }
    None
}

fn run_capture(
    program: &str,
    args: &[&str],
    cwd: &Path,
    env: &EnvMap,
    require_success: bool,
) -> Option<String> {
    let output = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .env_clear()
        .envs(env)
        .output()
        .ok()?;
    if require_success && !output.status.success() {
        return None;
    }
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    if text.trim().is_empty() {
        text = String::from_utf8_lossy(&output.stderr).into_owned();
    }
    (!text.trim().is_empty()).then_some(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncates_long_paths() {
        assert_eq!(truncate_path("~/a/b/c/d", 3, "…/"), "…/b/c/d");
        assert_eq!(truncate_path("~/a", 3, "…/"), "~/a");
        assert_eq!(truncate_path("~/a/b/c/d", 0, "…/"), "~/a/b/c/d");
    }

    #[test]
    fn extracts_versions() {
        assert_eq!(
            find_version("rustc 1.95.0 (59807616e)").as_deref(),
            Some("1.95.0")
        );
        assert_eq!(find_version("v24.15.0").as_deref(), Some("24.15.0"));
        assert_eq!(
            find_version("go version go1.22.3 windows/amd64").as_deref(),
            Some("1.22.3")
        );
        assert_eq!(find_version("no digits here"), None);
    }

    #[test]
    fn parses_porcelain_v2() {
        let text = "\
# branch.oid abc123
# branch.head feature/x
# branch.ab +2 -1
1 .M N... 100644 100644 100644 aaa bbb src/main.rs
1 M. N... 100644 100644 100644 aaa bbb src/lib.rs
1 D. N... 100644 100644 100644 aaa bbb old.rs
? untracked.txt
u UU N... 100644 100644 100644 100644 aaa bbb ccc conflict.rs
";
        let mut info = GitInfo::default();
        parse_porcelain(text, &mut info);
        assert_eq!(info.branch.as_deref(), Some("feature/x"));
        assert_eq!((info.ahead, info.behind), (2, 1));
        assert_eq!(info.modified, 1);
        assert_eq!(info.staged, 1);
        assert_eq!(info.deleted, 1);
        assert_eq!(info.untracked, 1);
        assert_eq!(info.conflicted, 1);
    }

    #[test]
    fn finds_repo_root_of_this_project() {
        let root = git_root(Path::new(env!("CARGO_MANIFEST_DIR")));
        assert!(root.is_some());
    }
}

//! 提示符各模块的实现。
//!
//! git 信息优先读 `.git` 目录（零进程开销），仅在开启 `git.status_enabled`
//! 时调用一次 `git status --porcelain=v2 --branch`，结果 1 秒内复用。

use super::Context;
use crate::config::{LangSpec, Visibility};
use crate::style;
use crate::util::{self, EnvMap};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};

/// 所有模块的正式名字（不含 `lang` / `directory` / `virtualenv` 这类别名）。
///
/// 加新模块时这里要一起加：`every_module_is_reachable` 测试会拿它对照
/// 默认 format 与 `$all`，把「实现了模块却没接进提示符」挡下来。
pub const MODULE_NAMES: &[&str] = &[
    "os",
    "username",
    "hostname",
    "dir",
    "git_branch",
    "git_state",
    "git_status",
    "git_commit",
    "venv",
    "languages",
    "package",
    "cmd_duration",
    "status",
    "time",
    "identity",
    "jobs",
    "shlvl",
    "container",
    "line_break",
    "character",
];

pub fn render_module(name: &str, ctx: &Context<'_>) -> Option<String> {
    match name {
        "line_break" => Some("\n".to_string()),
        "character" => Some(character(ctx)),
        "dir" | "directory" => Some(dir(ctx)),
        "git_branch" => git_branch(ctx),
        "git_status" => git_status(ctx),
        "git_state" => git_state(ctx),
        "git_commit" => git_commit(ctx),
        "languages" | "lang" => languages(ctx),
        "venv" | "virtualenv" => venv(ctx),
        "package" => package(ctx),
        "shlvl" => shlvl(ctx),
        "os" => os(ctx),
        "cmd_duration" => cmd_duration(ctx),
        "status" => status(ctx),
        "time" => time(ctx),
        "identity" => identity(ctx),
        "jobs" => jobs(ctx),
        "username" => render_username(ctx),
        "hostname" => render_hostname(ctx),
        "container" => container(ctx),
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

/// 正在进行中的 git 操作。冲突处理到一半时最需要它——
/// 只看分支名完全看不出自己卡在 rebase 中间。
fn git_state(ctx: &Context<'_>) -> Option<String> {
    let cfg = &ctx.config.git;
    if !cfg.enabled || !cfg.state_enabled {
        return None;
    }
    let dir = git_dir(&git_root(ctx.cwd)?)?;
    let text = read_git_state(&dir)?;
    Some(format!("{} ", style::paint(&cfg.state_style, &text)))
}

/// 纯文件判断，不调 git。各状态的标记文件见 git 的 `wt-status.c`。
fn read_git_state(dir: &Path) -> Option<String> {
    // rebase 有两种实现：交互式用 rebase-merge，`--apply` 用 rebase-apply。
    // rebase-apply 也被 `git am` 复用，用 applying 文件区分。
    for (sub, label) in [("rebase-merge", "REBASE"), ("rebase-apply", "REBASE")] {
        let path = dir.join(sub);
        if !path.is_dir() {
            continue;
        }
        let label = if sub == "rebase-apply" && path.join("applying").exists() {
            "AM"
        } else {
            label
        };
        let step = read_trimmed(&path.join("msgnum"));
        let total = read_trimmed(&path.join("end"));
        return Some(match (step, total) {
            (Some(step), Some(total)) => format!("{label} {step}/{total}"),
            _ => label.to_string(),
        });
    }
    for (file, label) in [
        ("MERGE_HEAD", "MERGING"),
        ("CHERRY_PICK_HEAD", "CHERRY-PICKING"),
        ("REVERT_HEAD", "REVERTING"),
        ("BISECT_LOG", "BISECTING"),
    ] {
        if dir.join(file).exists() {
            return Some(label.to_string());
        }
    }
    None
}

fn read_trimmed(path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let text = text.trim().to_string();
    (!text.is_empty()).then_some(text)
}

fn git_commit(ctx: &Context<'_>) -> Option<String> {
    let cfg = &ctx.config.git;
    if !cfg.enabled || !cfg.commit_enabled {
        return None;
    }
    let root = git_root(ctx.cwd)?;
    let hash = read_commit(&root, cfg.commit_length.max(1))?;
    Some(format!("{} ", style::paint(&cfg.commit_style, &hash)))
}

/// 读当前 commit 的短 hash。松散 ref 读不到时回落到 packed-refs——
/// 刚 clone 完的仓库分支 ref 都打包在一起，没有 .git/refs/heads/xxx 文件。
fn read_commit(root: &Path, length: usize) -> Option<String> {
    let dir = git_dir(root)?;
    let head = std::fs::read_to_string(dir.join("HEAD")).ok()?;
    let head = head.trim();
    let Some(reference) = head.strip_prefix("ref: ") else {
        // detached：HEAD 里直接就是 hash
        return Some(head.chars().take(length).collect());
    };
    if let Some(hash) = read_trimmed(&dir.join(reference)) {
        return Some(hash.chars().take(length).collect());
    }
    let packed = std::fs::read_to_string(dir.join("packed-refs")).ok()?;
    for line in packed.lines() {
        if line.starts_with('#') || line.starts_with('^') {
            continue;
        }
        if let Some((hash, name)) = line.split_once(' ') {
            if name.trim() == reference {
                return Some(hash.chars().take(length).collect());
            }
        }
    }
    None
}

fn languages(ctx: &Context<'_>) -> Option<String> {
    let cfg = &ctx.config.languages;
    if !cfg.enabled {
        return None;
    }
    let mut out = String::new();
    for spec in &cfg.entries {
        if let Some(text) = language(ctx, spec) {
            out.push_str(&text);
        }
    }
    // 前缀跟着内容一起出现，没检测到语言时不会留下孤立的 "via"。
    (!out.is_empty()).then(|| format!("{}{out}", cfg.prefix))
}

/// Python 虚拟环境。`$VIRTUAL_ENV` 指向目录，取它的名字；
/// 目录叫 .venv / venv / env 时没有辨识度，往上取一层父目录名。
fn venv(ctx: &Context<'_>) -> Option<String> {
    let cfg = &ctx.config.venv;
    if !cfg.enabled {
        return None;
    }
    let name = if let Some(conda) = util::env_get(ctx.env, "CONDA_DEFAULT_ENV") {
        (!conda.is_empty()).then(|| conda.clone())?
    } else {
        let path = util::env_get(ctx.env, "VIRTUAL_ENV").filter(|v| !v.is_empty())?;
        venv_name(Path::new(path))?
    };
    let text = format!("{}({name})", cfg.symbol);
    Some(format!("{} ", style::paint(&cfg.style, &text)))
}

fn venv_name(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_string_lossy().into_owned();
    if matches!(name.as_str(), ".venv" | "venv" | "env" | ".env") {
        if let Some(parent) = path.parent().and_then(|p| p.file_name()) {
            return Some(parent.to_string_lossy().into_owned());
        }
    }
    Some(name)
}

/// 当前项目的版本号。读文件而不是调包管理器——`npm version` 那类命令
/// 要几百毫秒，提示符等不起。
fn package(ctx: &Context<'_>) -> Option<String> {
    let cfg = &ctx.config.package;
    if !cfg.enabled {
        return None;
    }
    let version = package_version(ctx.cwd)?;
    let text = format!("{}v{version}", cfg.symbol);
    Some(format!("{} ", style::paint(&cfg.style, &text)))
}

fn package_version(cwd: &Path) -> Option<String> {
    if let Ok(text) = std::fs::read_to_string(cwd.join("package.json")) {
        // 顶层字段的提取统一放在 crate::json：package.json 里 "version"
        // 在 engines / 依赖信息里也有，必须按嵌套深度区分。
        if let Some(version) = crate::json::top_level_string(&text, "version") {
            return Some(version);
        }
    }
    for (file, section) in [
        ("Cargo.toml", "package"),
        ("pyproject.toml", "project"),
        ("pyproject.toml", "tool.poetry"),
    ] {
        if let Ok(text) = std::fs::read_to_string(cwd.join(file)) {
            if let Ok(table) = crate::config::toml::parse(&text) {
                if let Some(version) = table.string(&format!("{section}.version")) {
                    return Some(version);
                }
            }
        }
    }
    None
}

fn shlvl(ctx: &Context<'_>) -> Option<String> {
    let cfg = &ctx.config.shlvl;
    if !cfg.enabled {
        return None;
    }
    let level: usize = util::env_get(ctx.env, "SHLVL")?.trim().parse().ok()?;
    if level < cfg.threshold.max(1) {
        return None;
    }
    let text = format!("{}{level}", cfg.symbol);
    Some(format!("{} ", style::paint(&cfg.style, &text)))
}

fn os(ctx: &Context<'_>) -> Option<String> {
    let cfg = &ctx.config.os;
    if !cfg.enabled {
        return None;
    }
    // 用通用 Unicode 符号而不是 Nerd Font 图标：没装字体的终端不该看到方框。
    let symbol = match std::env::consts::OS {
        "macos" => "🍎",
        "windows" => "🪟",
        "linux" => "🐧",
        "freebsd" | "openbsd" | "netbsd" => "😈",
        _ => "💻",
    };
    Some(format!("{} ", style::paint(&cfg.style, symbol)))
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

/// `user@host` 紧凑写法。想要 `root in 🌐 box in` 那种分开的样式，
/// 用 `$username$hostname`。
fn identity(ctx: &Context<'_>) -> Option<String> {
    let cfg = &ctx.config.identity;
    let user = visible(cfg.show_user, ctx).then(|| username(ctx)).flatten();
    let host = visible(cfg.show_host, ctx).then(|| hostname(ctx)).flatten();
    let text = match (user, host) {
        (Some(user), Some(host)) => format!("{user}@{host}"),
        (Some(user), None) => user,
        (None, Some(host)) => host,
        (None, None) => return None,
    };
    Some(format!("{} ", style::paint(&cfg.style, &text)))
}

fn render_username(ctx: &Context<'_>) -> Option<String> {
    let cfg = &ctx.config.identity;
    if !visible(cfg.show_user, ctx) {
        return None;
    }
    let name = username(ctx)?;
    // root 换个颜色：它是需要被看见的状态，不该和普通用户长得一样。
    let spec = if name == "root" {
        &cfg.root_style
    } else {
        &cfg.style
    };
    Some(format!("{}{}", style::paint(spec, &name), cfg.suffix))
}

fn render_hostname(ctx: &Context<'_>) -> Option<String> {
    let cfg = &ctx.config.identity;
    if !visible(cfg.show_host, ctx) {
        return None;
    }
    let host = hostname(ctx)?;
    let text = format!("{}{host}", cfg.host_symbol);
    Some(format!("{}{}", style::paint(&cfg.style, &text), cfg.suffix))
}

/// `Auto` 的判定：只在身份值得说明的场合显示。
fn visible(visibility: Visibility, ctx: &Context<'_>) -> bool {
    match visibility {
        Visibility::Always => true,
        Visibility::Never => false,
        Visibility::Auto => is_root(ctx) || is_ssh(ctx) || container_name(ctx).is_some(),
    }
}

fn is_root(ctx: &Context<'_>) -> bool {
    // 不调 geteuid()：那需要 libc 的更多面，而 musl 静态链接下要尽量少碰系统。
    // 容器/远程场景里 USER 就是 root，够用了。
    username(ctx).as_deref() == Some("root")
}

fn is_ssh(ctx: &Context<'_>) -> bool {
    ["SSH_CONNECTION", "SSH_CLIENT", "SSH_TTY"]
        .iter()
        .any(|key| util::env_get(ctx.env, key).is_some_and(|v| !v.is_empty()))
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

fn container(ctx: &Context<'_>) -> Option<String> {
    let cfg = &ctx.config.container;
    if !cfg.enabled {
        return None;
    }
    let name = container_name(ctx)?;
    let text = if cfg.show_name && !name.is_empty() {
        format!("{} [{name}]", cfg.symbol)
    } else {
        cfg.symbol.clone()
    };
    Some(format!("{} ", style::paint(&cfg.style, &text)))
}

/// 识别容器/远程开发环境。只读环境变量和文件，不 fork 任何进程——
/// 提示符每次回车都要渲染，`systemd-detect-virt` 这类调用的代价不划算。
///
/// 顺序有讲究：Codespaces 和 devcontainer 内部同样存在 `/.dockerenv`，
/// 所以显式的环境变量必须排在文件探测之前，否则一律显示成 Docker。
fn container_name(ctx: &Context<'_>) -> Option<String> {
    let env = |key: &str| util::env_get(ctx.env, key).filter(|v| !v.is_empty());

    // 用户显式指定优先，也给无法自动识别的环境留一个出口。
    if let Some(name) = env("CMDS_CONTAINER") {
        return Some(name.clone());
    }
    if env("CODESPACES").is_some() {
        return Some("Codespaces".to_string());
    }
    if env("REMOTE_CONTAINERS").is_some()
        || env("REMOTE_CONTAINERS_IPC").is_some()
        || env("DEVCONTAINER").is_some()
    {
        return Some("Dev Container".to_string());
    }
    // OCI 运行时约定写入的 `container=` 变量（podman、lxc、systemd-nspawn）。
    if let Some(kind) = env("container") {
        return Some(title_case(kind));
    }
    if Path::new("/run/.containerenv").exists() {
        // podman 会往里写 name="…"，能拿到就显示具体容器名。
        let name = std::fs::read_to_string("/run/.containerenv")
            .ok()
            .and_then(|text| containerenv_name(&text));
        return Some(name.unwrap_or_else(|| "Podman".to_string()));
    }
    if Path::new("/.dockerenv").exists() {
        return Some("Docker".to_string());
    }
    if is_wsl() {
        return Some("WSL".to_string());
    }
    None
}

/// 解析 podman 的 `/run/.containerenv`，取 `name="…"`。
fn containerenv_name(text: &str) -> Option<String> {
    for line in text.lines() {
        if let Some(rest) = line.trim().strip_prefix("name=") {
            let name = rest.trim().trim_matches('"');
            if !name.is_empty() {
                return Some(name.to_string());
            }
        }
    }
    None
}

fn is_wsl() -> bool {
    std::fs::read_to_string("/proc/sys/kernel/osrelease")
        .map(|text| {
            let lower = text.to_lowercase();
            lower.contains("microsoft") || lower.contains("wsl")
        })
        .unwrap_or(false)
}

fn title_case(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
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

    // ── container / identity ───────────────────────────────────────────

    fn test_ctx<'a>(
        config: &'a crate::config::Config,
        cwd: &'a Path,
        env: &'a EnvMap,
    ) -> Context<'a> {
        Context {
            config,
            cwd,
            status: 0,
            duration_ms: 0,
            jobs: 0,
            env,
        }
    }

    fn env_of(pairs: &[(&str, &str)]) -> EnvMap {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn parses_podman_containerenv() {
        let text = "engine=\"podman-4.9.3\"\nname=\"my-dev-box\"\nid=\"abc\"\n";
        assert_eq!(containerenv_name(text).as_deref(), Some("my-dev-box"));
        assert_eq!(containerenv_name("engine=\"podman\"\n"), None);
        // 空的 name= 不该被当成容器名
        assert_eq!(containerenv_name("name=\"\"\n"), None);
    }

    #[test]
    fn explicit_env_outranks_dockerenv_file() {
        // Codespaces / devcontainer 里同样存在 /.dockerenv，所以显式变量必须优先，
        // 否则在 Codespaces 里会显示成 Docker。这里不依赖真实文件系统：
        // CMDS_CONTAINER 和 CODESPACES 都排在文件探测之前。
        crate::style::set_color_enabled(false);
        let config = crate::config::Config::default();
        let cwd = PathBuf::from(".");

        let env = env_of(&[("CODESPACES", "true")]);
        let ctx = test_ctx(&config, &cwd, &env);
        assert_eq!(container_name(&ctx).as_deref(), Some("Codespaces"));

        let env = env_of(&[("REMOTE_CONTAINERS", "true")]);
        let ctx = test_ctx(&config, &cwd, &env);
        assert_eq!(container_name(&ctx).as_deref(), Some("Dev Container"));

        // 用户显式指定盖过一切
        let env = env_of(&[("CMDS_CONTAINER", "prod-jump"), ("CODESPACES", "true")]);
        let ctx = test_ctx(&config, &cwd, &env);
        assert_eq!(container_name(&ctx).as_deref(), Some("prod-jump"));
    }

    #[test]
    fn renders_container_badge() {
        crate::style::set_color_enabled(false);
        let mut config = crate::config::Config::default();
        let cwd = PathBuf::from(".");
        let env = env_of(&[("container", "podman")]);

        let ctx = test_ctx(&config, &cwd, &env);
        assert_eq!(container(&ctx).as_deref(), Some("⬢ [Podman] "));

        config.container.show_name = false;
        let ctx = test_ctx(&config, &cwd, &env);
        assert_eq!(container(&ctx).as_deref(), Some("⬢ "));

        config.container.enabled = false;
        let ctx = test_ctx(&config, &cwd, &env);
        assert_eq!(container(&ctx), None);
    }

    #[test]
    fn auto_visibility_hides_local_user_but_shows_root() {
        crate::style::set_color_enabled(false);
        let config = crate::config::Config::default();
        let cwd = PathBuf::from(".");

        // 本地普通用户：提示符要保持干净
        let env = env_of(&[("USER", "junhey")]);
        let ctx = test_ctx(&config, &cwd, &env);
        assert_eq!(render_username(&ctx), None);
        assert_eq!(render_hostname(&ctx), None);

        // root：即使在本地也要显示
        let env = env_of(&[("USER", "root")]);
        let ctx = test_ctx(&config, &cwd, &env);
        assert_eq!(render_username(&ctx).as_deref(), Some("root in "));

        // SSH 会话：普通用户也显示
        let env = env_of(&[("USER", "junhey"), ("SSH_CONNECTION", "10.0.0.1 22")]);
        let ctx = test_ctx(&config, &cwd, &env);
        assert_eq!(render_username(&ctx).as_deref(), Some("junhey in "));

        // 容器里：普通用户也显示
        let env = env_of(&[("USER", "junhey"), ("CMDS_CONTAINER", "Docker")]);
        let ctx = test_ctx(&config, &cwd, &env);
        assert_eq!(render_username(&ctx).as_deref(), Some("junhey in "));
    }

    #[test]
    fn hostname_carries_symbol_and_connector() {
        crate::style::set_color_enabled(false);
        let mut config = crate::config::Config::default();
        config.identity.show_host = Visibility::Always;
        let cwd = PathBuf::from(".");
        let env = env_of(&[("HOSTNAME", "joonhe-1rnzdldo2a")]);
        let ctx = test_ctx(&config, &cwd, &env);
        assert_eq!(
            render_hostname(&ctx).as_deref(),
            Some("🌐 joonhe-1rnzdldo2a in ")
        );
    }

    /// 连接词由模块自己带，所以取不到值时不会在提示符上留下孤立的 "in"。
    #[test]
    fn connector_never_dangles() {
        crate::style::set_color_enabled(false);
        let mut config = crate::config::Config::default();
        config.identity.show_user = Visibility::Always;
        let cwd = PathBuf::from(".");
        // 一个没有 USER/USERNAME/LOGNAME 的环境
        let env = env_of(&[("PATH", "/usr/bin")]);
        let ctx = test_ctx(&config, &cwd, &env);
        assert_eq!(render_username(&ctx), None);
    }

    // ── git_state / git_commit / package / venv ────────────────────────

    fn sandbox(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "cmds-modules-{tag}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn reads_rebase_progress() {
        let root = sandbox("rebase");
        let git = root.join(".git");
        let rebase = git.join("rebase-merge");
        std::fs::create_dir_all(&rebase).unwrap();
        std::fs::write(rebase.join("msgnum"), b"2\n").unwrap();
        std::fs::write(rebase.join("end"), b"5\n").unwrap();
        assert_eq!(read_git_state(&git).as_deref(), Some("REBASE 2/5"));

        // 没有进度文件时只报状态，不该崩
        std::fs::remove_file(rebase.join("msgnum")).unwrap();
        assert_eq!(read_git_state(&git).as_deref(), Some("REBASE"));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn distinguishes_am_from_rebase() {
        let root = sandbox("am");
        let git = root.join(".git");
        let apply = git.join("rebase-apply");
        std::fs::create_dir_all(&apply).unwrap();
        // rebase-apply 被 git am 复用，靠 applying 文件区分
        std::fs::write(apply.join("applying"), b"").unwrap();
        std::fs::write(apply.join("msgnum"), b"1").unwrap();
        std::fs::write(apply.join("end"), b"3").unwrap();
        assert_eq!(read_git_state(&git).as_deref(), Some("AM 1/3"));

        std::fs::remove_file(apply.join("applying")).unwrap();
        assert_eq!(read_git_state(&git).as_deref(), Some("REBASE 1/3"));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn reads_merge_and_cherry_pick_states() {
        let root = sandbox("states");
        let git = root.join(".git");
        std::fs::create_dir_all(&git).unwrap();
        assert_eq!(read_git_state(&git), None);

        std::fs::write(git.join("MERGE_HEAD"), b"abc").unwrap();
        assert_eq!(read_git_state(&git).as_deref(), Some("MERGING"));
        std::fs::remove_file(git.join("MERGE_HEAD")).unwrap();

        std::fs::write(git.join("CHERRY_PICK_HEAD"), b"abc").unwrap();
        assert_eq!(read_git_state(&git).as_deref(), Some("CHERRY-PICKING"));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn reads_commit_from_loose_ref_and_packed_refs() {
        let root = sandbox("commit");
        let git = root.join(".git");
        std::fs::create_dir_all(git.join("refs/heads")).unwrap();
        std::fs::write(git.join("HEAD"), b"ref: refs/heads/main\n").unwrap();
        std::fs::write(
            git.join("refs/heads/main"),
            b"1234567890abcdef1234567890abcdef12345678\n",
        )
        .unwrap();
        assert_eq!(read_commit(&root, 7).as_deref(), Some("1234567"));

        // 松散 ref 不存在时要回落到 packed-refs（刚 clone 的仓库就是这样）
        std::fs::remove_file(git.join("refs/heads/main")).unwrap();
        std::fs::write(
            git.join("packed-refs"),
            b"# pack-refs with: peeled fully-peeled sorted \n\
              aaaaaaabbbbbbbcccccccdddddddeeeeeeefffffff refs/heads/main\n\
              ^0000000000000000000000000000000000000000\n",
        )
        .unwrap();
        assert_eq!(read_commit(&root, 7).as_deref(), Some("aaaaaaa"));

        // detached HEAD：内容直接是 hash
        std::fs::write(git.join("HEAD"), b"deadbeefdeadbeefdeadbeefdeadbeef\n").unwrap();
        assert_eq!(read_commit(&root, 8).as_deref(), Some("deadbeef"));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn reads_package_version_from_json_and_toml() {
        let root = sandbox("pkgver");
        std::fs::write(root.join("package.json"), br#"{"version": "3.1.4"}"#).unwrap();
        assert_eq!(package_version(&root).as_deref(), Some("3.1.4"));

        std::fs::remove_file(root.join("package.json")).unwrap();
        std::fs::write(
            root.join("Cargo.toml"),
            b"[package]\nname = \"x\"\nversion = \"0.9.1\"\n",
        )
        .unwrap();
        assert_eq!(package_version(&root).as_deref(), Some("0.9.1"));

        std::fs::remove_file(root.join("Cargo.toml")).unwrap();
        std::fs::write(
            root.join("pyproject.toml"),
            b"[tool.poetry]\nversion = \"7.7.7\"\n",
        )
        .unwrap();
        assert_eq!(package_version(&root).as_deref(), Some("7.7.7"));

        std::fs::remove_dir_all(&root).ok();
        assert_eq!(package_version(&root), None);
    }

    #[test]
    fn venv_name_skips_meaningless_directory_names() {
        // .venv 这种名字没有辨识度，往上取一层项目名
        assert_eq!(
            venv_name(Path::new("/home/me/my-project/.venv")).as_deref(),
            Some("my-project")
        );
        assert_eq!(
            venv_name(Path::new("/home/me/envs/scraper")).as_deref(),
            Some("scraper")
        );
    }

    #[test]
    fn shlvl_respects_threshold() {
        crate::style::set_color_enabled(false);
        let mut config = crate::config::Config::default();
        config.shlvl.enabled = true;
        let cwd = PathBuf::from(".");

        let env = env_of(&[("SHLVL", "1")]);
        let ctx = test_ctx(&config, &cwd, &env);
        assert_eq!(shlvl(&ctx), None, "顶层 shell 不必提示");

        let env = env_of(&[("SHLVL", "3")]);
        let ctx = test_ctx(&config, &cwd, &env);
        assert_eq!(shlvl(&ctx).as_deref(), Some("↕ 3 "));
    }

    /// 前缀跟内容一起出现：没检测到语言时不该在提示符上留下孤立的 "via"。
    #[test]
    fn language_prefix_never_dangles() {
        crate::style::set_color_enabled(false);
        let config = crate::config::Config::default();
        let empty = sandbox("nolang");
        let env = EnvMap::new();
        let ctx = test_ctx(&config, &empty, &env);
        assert_eq!(languages(&ctx), None);
        std::fs::remove_dir_all(&empty).ok();
    }

    #[test]
    fn never_visibility_wins_over_root() {
        crate::style::set_color_enabled(false);
        let mut config = crate::config::Config::default();
        config.identity.show_user = Visibility::Never;
        let cwd = PathBuf::from(".");
        let env = env_of(&[("USER", "root")]);
        let ctx = test_ctx(&config, &cwd, &env);
        assert_eq!(render_username(&ctx), None);
    }
}

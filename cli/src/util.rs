//! 跨平台小工具：环境变量、路径、可执行文件查找、时间格式化。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub type EnvMap = BTreeMap<String, String>;

/// 读取环境变量（Windows 下大小写不敏感）。
pub fn env_get<'a>(env: &'a EnvMap, key: &str) -> Option<&'a String> {
    if let Some(v) = env.get(key) {
        return Some(v);
    }
    env.iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(key))
        .map(|(_, v)| v)
}

/// 写入环境变量，复用已存在的键名大小写。
pub fn env_set(env: &mut EnvMap, key: &str, value: impl Into<String>) {
    let key = env
        .keys()
        .find(|k| k.eq_ignore_ascii_case(key))
        .cloned()
        .unwrap_or_else(|| key.to_string());
    env.insert(key, value.into());
}

pub fn env_remove(env: &mut EnvMap, key: &str) -> Option<String> {
    let key = env.keys().find(|k| k.eq_ignore_ascii_case(key)).cloned()?;
    env.remove(&key)
}

pub fn snapshot_env() -> EnvMap {
    std::env::vars().collect()
}

pub fn home_dir() -> Option<PathBuf> {
    for key in ["HOME", "USERPROFILE"] {
        if let Some(v) = std::env::var_os(key) {
            if !v.is_empty() {
                return Some(PathBuf::from(v));
            }
        }
    }
    #[cfg(windows)]
    if let (Some(drive), Some(path)) = (std::env::var_os("HOMEDRIVE"), std::env::var_os("HOMEPATH"))
    {
        let mut joined = drive;
        joined.push(path);
        return Some(PathBuf::from(joined));
    }
    None
}

/// 配置目录：`$CMDS_CONFIG_DIR` > `$XDG_CONFIG_HOME/cmds` > `%APPDATA%\cmds` > `~/.config/cmds`。
pub fn config_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("CMDS_CONFIG_DIR") {
        return PathBuf::from(dir);
    }
    if let Some(dir) = std::env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(dir).join("cmds");
    }
    #[cfg(windows)]
    if let Some(dir) = std::env::var_os("APPDATA") {
        return PathBuf::from(dir).join("cmds");
    }
    home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".config")
        .join("cmds")
}

/// 数据目录（历史等）：`$CMDS_DATA_DIR` > `$XDG_DATA_HOME/cmds` > `%LOCALAPPDATA%\cmds` > `~/.local/share/cmds`。
pub fn data_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("CMDS_DATA_DIR") {
        return PathBuf::from(dir);
    }
    if let Some(dir) = std::env::var_os("XDG_DATA_HOME") {
        return PathBuf::from(dir).join("cmds");
    }
    #[cfg(windows)]
    if let Some(dir) = std::env::var_os("LOCALAPPDATA") {
        return PathBuf::from(dir).join("cmds");
    }
    home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".local")
        .join("share")
        .join("cmds")
}

/// 展开开头的 `~` / `~/...`。
pub fn expand_tilde(input: &str) -> String {
    if input == "~" {
        return home_dir()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| input.to_string());
    }
    if let Some(rest) = input
        .strip_prefix("~/")
        .or_else(|| input.strip_prefix("~\\"))
    {
        if let Some(home) = home_dir() {
            return home.join(rest).to_string_lossy().into_owned();
        }
    }
    input.to_string()
}

/// 把 home 前缀折叠成符号（提示符里用）。
pub fn contract_home(path: &Path, symbol: &str) -> String {
    let text = path.to_string_lossy().replace('\\', "/");
    if let Some(home) = home_dir() {
        let home = home.to_string_lossy().replace('\\', "/");
        if !home.is_empty() {
            if text == home {
                return symbol.to_string();
            }
            let prefixed = format!("{home}/");
            if let Some(rest) = text.strip_prefix(&prefixed) {
                return format!("{symbol}/{rest}");
            }
        }
    }
    text
}

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// starship 风格的耗时格式：`1h2m3s`、`4.2s`、`320ms`。
pub fn format_duration(ms: u128) -> String {
    if ms < 1000 {
        return format!("{ms}ms");
    }
    let total_secs = ms / 1000;
    let hours = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    let secs = total_secs % 60;
    let mut out = String::new();
    if hours > 0 {
        out.push_str(&format!("{hours}h"));
    }
    if hours > 0 || mins > 0 {
        out.push_str(&format!("{mins}m"));
    }
    if hours > 0 || mins > 0 {
        out.push_str(&format!("{secs}s"));
    } else {
        let frac = (ms % 1000) / 100;
        if frac > 0 {
            out.push_str(&format!("{secs}.{frac}s"));
        } else {
            out.push_str(&format!("{secs}s"));
        }
    }
    out
}

/// 人类可读的文件大小。
pub fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "K", "M", "G", "T"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit + 1 < UNITS.len() {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes}{}", UNITS[0])
    } else {
        format!("{size:.1}{}", UNITS[unit])
    }
}

/// 过长文本中间省略。
pub fn truncate_middle(text: &str, max: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max || max < 5 {
        return text.to_string();
    }
    let head: String = chars[..max / 2].iter().collect();
    let tail: String = chars[chars.len() - (max / 2 - 1)..].iter().collect();
    format!("{head}…{tail}")
}

/// 编辑距离（命令拼写建议用）。
pub fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.is_empty() {
        return b.len();
    }
    if b.is_empty() {
        return a.len();
    }
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    let mut current = vec![0usize; b.len() + 1];
    for (i, ca) in a.iter().enumerate() {
        current[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            current[j + 1] = (previous[j] + cost)
                .min(previous[j + 1] + 1)
                .min(current[j] + 1);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[b.len()]
}

#[cfg(unix)]
pub fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(windows)]
pub fn is_executable(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => windows_exts().iter().any(|e| e.eq_ignore_ascii_case(ext)),
        None => false,
    }
}

#[cfg(windows)]
pub fn windows_exts() -> Vec<String> {
    std::env::var("PATHEXT")
        .unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_string())
        .split(';')
        .filter_map(|e| e.trim().strip_prefix('.').map(|s| s.to_string()))
        .filter(|e| !e.is_empty())
        .collect()
}

pub fn path_dirs(env: &EnvMap) -> Vec<PathBuf> {
    let Some(raw) = env_get(env, "PATH") else {
        return Vec::new();
    };
    let sep = if cfg!(windows) { ';' } else { ':' };
    raw.split(sep)
        .filter(|p| !p.is_empty())
        .map(PathBuf::from)
        .collect()
}

fn has_path_separator(name: &str) -> bool {
    name.contains('/') || (cfg!(windows) && name.contains('\\'))
}

/// 在 PATH 中定位命令（Windows 下自动尝试 PATHEXT 扩展）。
pub fn lookup_command(name: &str, env: &EnvMap, cwd: &Path) -> Option<PathBuf> {
    if name.is_empty() {
        return None;
    }
    if has_path_separator(name) {
        let candidate = if Path::new(name).is_absolute() {
            PathBuf::from(name)
        } else {
            cwd.join(name)
        };
        return executable_with_ext(&candidate);
    }
    for dir in path_dirs(env) {
        if let Some(found) = executable_with_ext(&dir.join(name)) {
            return Some(found);
        }
    }
    None
}

fn executable_with_ext(candidate: &Path) -> Option<PathBuf> {
    if is_executable(candidate) {
        return Some(candidate.to_path_buf());
    }
    #[cfg(windows)]
    for ext in windows_exts() {
        let mut with_ext = candidate.as_os_str().to_os_string();
        with_ext.push(".");
        with_ext.push(&ext);
        let path = PathBuf::from(with_ext);
        if is_executable(&path) {
            return Some(path);
        }
    }
    None
}

/// 扫描 PATH 下所有可执行文件名（补全用，调用方负责缓存）。
pub fn list_path_executables(env: &EnvMap) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for dir in path_dirs(env) {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !is_executable(&path) {
                continue;
            }
            let Some(file) = path.file_name().and_then(|f| f.to_str()) else {
                continue;
            };
            // Windows 下补全时不带 .exe / .cmd 后缀
            #[cfg(windows)]
            let file = path.file_stem().and_then(|f| f.to_str()).unwrap_or(file);
            names.push(file.to_string());
        }
    }
    names.sort_unstable();
    names.dedup();
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_durations() {
        assert_eq!(format_duration(250), "250ms");
        assert_eq!(format_duration(1500), "1.5s");
        assert_eq!(format_duration(65_000), "1m5s");
        assert_eq!(format_duration(3_725_000), "1h2m5s");
    }

    #[test]
    fn env_lookup_is_case_insensitive() {
        let mut env = EnvMap::new();
        env.insert("Path".into(), "/bin".into());
        assert_eq!(env_get(&env, "PATH").map(String::as_str), Some("/bin"));
        env_set(&mut env, "PATH", "/usr/bin");
        assert_eq!(env.len(), 1);
        assert_eq!(env.get("Path").map(String::as_str), Some("/usr/bin"));
    }

    #[test]
    fn contracts_home_symbol() {
        let home = home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        assert_eq!(contract_home(&home, "~"), "~");
        assert_eq!(contract_home(&home.join("code"), "~"), "~/code");
    }
}

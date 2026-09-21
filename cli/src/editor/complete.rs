//! 候选生成：历史整条命令、内建/别名/缩写/PATH 命令、路径、环境变量。

use crate::builtins;
use crate::config::Config;
use crate::shell::Shell;
use crate::util;
use std::ops::Range;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateKind {
    History,
    Alias,
    Abbr,
    Builtin,
    Command,
    Directory,
    File,
    Variable,
}

impl CandidateKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::History => "历史",
            Self::Alias => "别名",
            Self::Abbr => "缩写",
            Self::Builtin => "内建",
            Self::Command => "命令",
            Self::Directory => "目录",
            Self::File => "文件",
            Self::Variable => "变量",
        }
    }

    fn rank(self) -> u8 {
        match self {
            Self::History => 0,
            Self::Alias | Self::Abbr => 1,
            Self::Builtin | Self::Variable => 2,
            Self::Command => 3,
            Self::Directory => 4,
            Self::File => 5,
        }
    }

    /// 采纳候选后自动补的后缀。
    pub fn suffix(self) -> &'static str {
        match self {
            Self::History => "",
            Self::Directory => "",
            _ => " ",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    /// 实际插入的文本
    pub value: String,
    /// 菜单里展示的文本
    pub display: String,
    pub description: String,
    pub kind: CandidateKind,
    /// 需要被替换掉的缓冲区字节范围
    pub replace: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenContext {
    pub range: Range<usize>,
    pub text: String,
    /// 是否处于命令名位置（行首或操作符之后）
    pub command_position: bool,
    /// 同一条命令里光标前的上一个词
    pub previous: Option<String>,
}

fn is_boundary(c: char) -> bool {
    c.is_whitespace() || matches!(c, '|' | '&' | ';' | '<' | '>')
}

/// 定位光标所在 token（只考虑光标左侧内容，符合补全直觉）。
pub fn token_at(line: &str, cursor: usize) -> TokenContext {
    let cursor = cursor.min(line.len());
    let head = &line[..cursor];
    let start = head
        .char_indices()
        .rev()
        .find(|(_, c)| is_boundary(*c))
        .map(|(index, c)| index + c.len_utf8())
        .unwrap_or(0);
    let raw = &line[start..cursor];
    let text = raw
        .strip_prefix('"')
        .or_else(|| raw.strip_prefix('\''))
        .unwrap_or(raw)
        .to_string();

    let before = line[..start].trim_end();
    let command_position = before.is_empty()
        || before.ends_with('|')
        || before.ends_with("&&")
        || before.ends_with("||")
        || before.ends_with(';')
        || before.ends_with('&');
    let previous = before
        .rsplit(|c: char| is_boundary(c))
        .find(|w| !w.is_empty())
        .map(|w| w.to_string());

    TokenContext {
        range: start..cursor,
        text,
        command_position,
        previous,
    }
}

/// 生成候选列表：历史优先，随后是命令/路径/变量补全。
pub fn complete(shell: &mut Shell, line: &str, cursor: usize, config: &Config) -> Vec<Candidate> {
    let token = token_at(line, cursor);
    let mut candidates: Vec<Candidate> = Vec::new();

    // ① 历史整行匹配（用户输入前缀即可召回）
    if !line.trim().is_empty() {
        let needle = line.trim_start().to_string();
        let matches: Vec<(String, u32)> = shell
            .history
            .ranked_matches(&needle, 20)
            .into_iter()
            .map(|command| (command.to_string(), shell.history.use_count(command)))
            .collect();
        for (command, count) in matches {
            candidates.push(Candidate {
                value: command.clone(),
                display: command,
                description: if count > 1 {
                    format!("历史 · 用过 {count} 次")
                } else {
                    "历史".to_string()
                },
                kind: CandidateKind::History,
                replace: 0..line.len(),
            });
        }
    }

    // ② 环境变量
    if let Some(prefix) = token.text.strip_prefix('$') {
        let keys: Vec<String> = shell
            .env
            .keys()
            .filter(|key| key.starts_with(prefix))
            .cloned()
            .collect();
        for key in keys {
            let value = shell.lookup_var(&key).unwrap_or_default();
            candidates.push(Candidate {
                value: format!("${key}"),
                display: format!("${key}"),
                description: util::truncate_middle(&value, 40),
                kind: CandidateKind::Variable,
                replace: token.range.clone(),
            });
        }
        return finish(candidates, config);
    }

    // ③ 命令位置：内建 / 别名 / 缩写 / 历史命令 / PATH
    let prefix = token.text.clone();
    if token.command_position && !prefix.contains('/') && !prefix.contains('\\') {
        for name in builtins::names() {
            if name.starts_with(&prefix) {
                candidates.push(Candidate {
                    value: name.to_string(),
                    display: name.to_string(),
                    description: builtins::describe(name).unwrap_or("内建命令").to_string(),
                    kind: CandidateKind::Builtin,
                    replace: token.range.clone(),
                });
            }
        }
        let aliases: Vec<(String, String)> = shell
            .aliases
            .iter()
            .filter(|(name, _)| name.starts_with(&prefix))
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect();
        for (name, value) in aliases {
            candidates.push(Candidate {
                value: name.clone(),
                display: name,
                description: format!("别名 → {value}"),
                kind: CandidateKind::Alias,
                replace: token.range.clone(),
            });
        }
        let abbrs: Vec<(String, String)> = shell
            .abbreviations
            .iter()
            .filter(|(name, _)| name.starts_with(&prefix))
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect();
        for (name, value) in abbrs {
            candidates.push(Candidate {
                value: name.clone(),
                display: name,
                description: format!("缩写 → {value}"),
                kind: CandidateKind::Abbr,
                replace: token.range.clone(),
            });
        }
        let history_words: Vec<(String, u32)> = shell
            .history
            .command_words()
            .into_iter()
            .filter(|(word, _)| word.starts_with(&prefix))
            .map(|(word, count)| (word.to_string(), count))
            .take(10)
            .collect();
        for (word, count) in history_words {
            candidates.push(Candidate {
                value: word.clone(),
                display: word,
                description: format!("常用 · {count} 次"),
                kind: CandidateKind::Command,
                replace: token.range.clone(),
            });
        }
        let path_commands: Vec<String> = shell
            .path_commands()
            .iter()
            .filter(|name| starts_with_ci(name, &prefix))
            .take(120)
            .cloned()
            .collect();
        for name in path_commands {
            candidates.push(Candidate {
                value: name.clone(),
                display: name,
                description: "PATH 中的可执行文件".to_string(),
                kind: CandidateKind::Command,
                replace: token.range.clone(),
            });
        }
    }

    // ④ 路径补全
    let only_dirs = matches!(
        token.previous.as_deref(),
        Some("cd") | Some("pushd") | Some("rmdir")
    );
    candidates.extend(path_candidates(shell, &token, only_dirs));

    finish(candidates, config)
}

fn finish(mut candidates: Vec<Candidate>, config: &Config) -> Vec<Candidate> {
    candidates.dedup_by(|a, b| a.value == b.value && a.kind == b.kind);
    let mut seen: Vec<(String, CandidateKind)> = Vec::new();
    candidates.retain(|candidate| {
        let key = (candidate.value.clone(), candidate.kind);
        if seen.contains(&key) {
            false
        } else {
            seen.push(key);
            true
        }
    });
    candidates.sort_by_key(|candidate| candidate.kind.rank());
    candidates.truncate(config.menu.max_candidates.max(1));
    candidates
}

fn starts_with_ci(text: &str, prefix: &str) -> bool {
    if cfg!(windows) {
        text.to_lowercase().starts_with(&prefix.to_lowercase())
    } else {
        text.starts_with(prefix)
    }
}

fn path_candidates(shell: &Shell, token: &TokenContext, only_dirs: bool) -> Vec<Candidate> {
    let raw = token.text.clone();
    let expanded = util::expand_tilde(&raw);
    let separator = raw.rfind(['/', '\\']);
    let input_prefix = match separator {
        Some(index) => raw[..=index].to_string(),
        None => String::new(),
    };
    let (dir_text, file_prefix) = match expanded.rfind(['/', '\\']) {
        Some(index) => (
            expanded[..=index].to_string(),
            expanded[index + 1..].to_string(),
        ),
        None => (String::new(), expanded.clone()),
    };

    let dir = if dir_text.is_empty() {
        shell.cwd.clone()
    } else if Path::new(&dir_text).is_absolute() {
        PathBuf::from(&dir_text)
    } else {
        shell.cwd.join(&dir_text)
    };

    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };

    let mut candidates = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !starts_with_ci(&name, &file_prefix) {
            continue;
        }
        if name.starts_with('.') && !file_prefix.starts_with('.') {
            continue;
        }
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
        if only_dirs && !is_dir {
            continue;
        }
        let mut value = format!("{input_prefix}{name}");
        if is_dir {
            value.push('/');
        }
        let description = if is_dir {
            "目录".to_string()
        } else {
            match entry.metadata().map(|m| m.len()) {
                Ok(size) => format!("文件 · {}", util::format_size(size)),
                Err(_) => "文件".to_string(),
            }
        };
        candidates.push(Candidate {
            value: quote_if_needed(&value),
            display: if is_dir {
                format!("{name}/")
            } else {
                name.clone()
            },
            description,
            kind: if is_dir {
                CandidateKind::Directory
            } else {
                CandidateKind::File
            },
            replace: token.range.clone(),
        });
    }
    candidates.sort_by(|a, b| a.display.cmp(&b.display));
    candidates
}

fn quote_if_needed(value: &str) -> String {
    if value.contains(' ') && !value.starts_with('"') {
        format!("\"{value}\"")
    } else {
        value.to_string()
    }
}

/// 多个候选共享的前缀（Tab 先补齐公共部分，行为与 fish 一致）。
pub fn common_prefix(candidates: &[Candidate]) -> Option<(Range<usize>, String)> {
    let first = candidates.first()?;
    if first.kind == CandidateKind::History {
        return None;
    }
    let range = first.replace.clone();
    if candidates
        .iter()
        .any(|c| c.replace != range || c.kind == CandidateKind::History)
    {
        return None;
    }
    let mut prefix = first.value.clone();
    for candidate in candidates.iter().skip(1) {
        let common: String = prefix
            .chars()
            .zip(candidate.value.chars())
            .take_while(|(a, b)| a == b)
            .map(|(a, _)| a)
            .collect();
        prefix = common;
        if prefix.is_empty() {
            return None;
        }
    }
    Some((range, prefix))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn locates_command_position() {
        let token = token_at("ls -l", 2);
        assert_eq!(token.text, "ls");
        assert!(token.command_position);

        let token = token_at("ls -l", 5);
        assert_eq!(token.text, "-l");
        assert!(!token.command_position);
        assert_eq!(token.previous.as_deref(), Some("ls"));

        let token = token_at("cat a.txt | gr", 14);
        assert_eq!(token.text, "gr");
        assert!(token.command_position);
    }

    #[test]
    fn completes_builtin_commands() {
        let mut shell = Shell::new(Config::default(), false);
        let config = Config::default();
        let candidates = complete(&mut shell, "hel", 3, &config);
        assert!(
            candidates
                .iter()
                .any(|c| c.value == "help" && c.kind == CandidateKind::Builtin)
        );
    }

    #[test]
    fn completes_history_entries() {
        let mut shell = Shell::new(Config::default(), false);
        shell.history.record("cargo build --release");
        let config = Config::default();
        let candidates = complete(&mut shell, "cargo", 5, &config);
        let history: Vec<&Candidate> = candidates
            .iter()
            .filter(|c| c.kind == CandidateKind::History)
            .collect();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].value, "cargo build --release");
        assert_eq!(history[0].replace, 0..5);
        // 历史候选排在最前面
        assert_eq!(candidates[0].kind, CandidateKind::History);
    }

    #[test]
    fn completes_paths() {
        // 不能依赖进程当前目录：`cd` 的内建测试会调用 std::env::set_current_dir，
        // 并行跑测试时会把这里的相对路径补全打乱。用独立临时目录自带待补全文件。
        let mut shell = Shell::new(Config::default(), false);
        let sandbox = std::env::temp_dir().join(format!(
            "cmds-complete-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&sandbox).unwrap();
        std::fs::write(sandbox.join("Cargofile.txt"), b"fixture").unwrap();
        shell.cwd = sandbox.clone();

        let config = Config::default();
        let candidates = complete(&mut shell, "cat Car", 7, &config);
        let matched = candidates.iter().any(|c| c.value.starts_with("Cargofile"));

        std::fs::remove_dir_all(&sandbox).ok();
        assert!(matched, "相对路径补全应命中临时目录里的 Cargofile.txt");
    }

    #[test]
    fn completes_variables() {
        let mut shell = Shell::new(Config::default(), false);
        let config = Config::default();
        let candidates = complete(&mut shell, "echo $CMDS_VER", 13, &config);
        assert!(
            candidates
                .iter()
                .any(|c| c.value == "$CMDS_VERSION" && c.kind == CandidateKind::Variable)
        );
    }

    #[test]
    fn computes_common_prefix() {
        let make = |value: &str| Candidate {
            value: value.to_string(),
            display: value.to_string(),
            description: String::new(),
            kind: CandidateKind::Command,
            replace: 0..2,
        };
        let candidates = vec![make("cargo"), make("carbon")];
        let (range, prefix) = common_prefix(&candidates).unwrap();
        assert_eq!(range, 0..2);
        assert_eq!(prefix, "car");
    }
}

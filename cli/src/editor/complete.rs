//! 候选生成：历史整条命令、内建/别名/缩写/PATH 命令、路径、环境变量。

use super::scripts;
use super::subcommands;
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
    /// 内置/配置的子命令表，如 `git status`。
    Subcommand,
    /// 项目自带的脚本（package.json 的 scripts、Makefile 的 target）。
    Script,
    Command,
    Directory,
    File,
    Variable,
}

impl CandidateKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::History => t!("history", "历史"),
            Self::Alias => t!("alias", "别名"),
            Self::Abbr => t!("abbr", "缩写"),
            Self::Builtin => t!("builtin", "内建"),
            Self::Subcommand => t!("subcommand", "子命令"),
            Self::Script => t!("script", "脚本"),
            Self::Command => t!("command", "命令"),
            Self::Directory => t!("dir", "目录"),
            Self::File => t!("file", "文件"),
            Self::Variable => t!("var", "变量"),
        }
    }

    fn rank(self) -> u8 {
        match self {
            Self::History => 0,
            Self::Alias | Self::Abbr => 1,
            // 子命令和项目脚本排在路径前面：输入 `git s` 想要的是 status，
            // 不是当前目录里恰好以 s 开头的文件。
            Self::Builtin | Self::Variable | Self::Subcommand | Self::Script => 2,
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
                    tf!("history · used {count} times", "历史 · 用过 {count} 次")
                } else {
                    t!("history", "历史").to_string()
                },
                kind: CandidateKind::History,
                replace: 0..line.len(),
            });
        }
    }

    // ①.5 预置脚本：按整行前缀召回，和历史一个路子。
    //
    // 刻意不做成「必须记住缩写」：名字和命令本身都参与匹配，所以输入 `gst`
    // 或者 `git st` 都能找到 `git status --short --branch`。只认缩写的话，
    // 用户得先记住缩写才用得上——那就失去意义了。
    let typed = line.trim_start();
    for (name, command) in &config.scripts {
        if !name.starts_with(typed) && !command.starts_with(typed) {
            continue;
        }
        candidates.push(Candidate {
            value: command.clone(),
            display: command.clone(),
            description: tf!("script · {name}", "脚本 · {name}"),
            kind: CandidateKind::Script,
            replace: 0..line.len(),
        });
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
                    description: builtins::describe(name)
                        .unwrap_or(t!("builtin command", "内建命令"))
                        .to_string(),
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
                description: tf!("alias → {value}", "别名 → {value}"),
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
                description: tf!("abbr → {value}", "缩写 → {value}"),
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
                description: tf!("frequent · {count} times", "常用 · {count} 次"),
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
                description: t!("executable on PATH", "PATH 中的可执行文件").to_string(),
                kind: CandidateKind::Command,
                replace: token.range.clone(),
            });
        }
    }

    // ④ 子命令与项目脚本（只在参数位置，命令位置已经在 ③ 处理过了）
    if !token.command_position {
        candidates.extend(subcommand_candidates(&token, line, config));
        candidates.extend(script_candidates(shell, &token, line));
    }

    // ⑤ 路径补全
    let only_dirs = matches!(
        token.previous.as_deref(),
        Some("cd") | Some("pushd") | Some("rmdir")
    );
    candidates.extend(path_candidates(shell, &token, only_dirs));

    finish(candidates, config)
}

/// 光标所在命令已经输入完的词，跳过选项。用来查子命令表：
/// `git ` → `["git"]`，`git remote ` → `["git", "remote"]`。
fn command_words(line: &str, token_start: usize) -> Vec<String> {
    let head = &line[..token_start.min(line.len())];
    // 从上一个操作符之后算起，`ls | git ` 里的命令是 git 而不是 ls
    let start = head
        .rfind(['|', '&', ';'])
        .map(|index| index + 1)
        .unwrap_or(0);
    head[start..]
        .split_whitespace()
        .filter(|word| !word.starts_with('-'))
        .map(str::to_string)
        .collect()
}

fn subcommand_candidates(token: &TokenContext, line: &str, config: &Config) -> Vec<Candidate> {
    let chain = command_words(line, token.range.start).join(" ");
    if chain.is_empty() {
        return Vec::new();
    }
    let prefix = token.text.as_str();
    let mut candidates = Vec::new();

    // 用户配置先加：finish() 按「先到先留」去重，这样同名项用户的说明会胜出。
    for (key, entries) in &config.completions {
        if key != &chain {
            continue;
        }
        for (name, description) in entries {
            if name.starts_with(prefix) {
                candidates.push(Candidate {
                    value: name.clone(),
                    display: name.clone(),
                    description: description.clone(),
                    kind: CandidateKind::Subcommand,
                    replace: token.range.clone(),
                });
            }
        }
    }

    if let Some(list) = subcommands::lookup(&chain) {
        for entry in list {
            if entry.name.starts_with(prefix) {
                candidates.push(Candidate {
                    value: entry.name.to_string(),
                    display: entry.name.to_string(),
                    description: entry.description().to_string(),
                    kind: CandidateKind::Subcommand,
                    replace: token.range.clone(),
                });
            }
        }
    }
    candidates
}

/// 项目自带的脚本：`npm run <Tab>` 给 package.json 里的 scripts，
/// `make <Tab>` 给 Makefile 的 target。这些名字只有项目自己知道，
/// 记不住又常用，是补全最该帮忙的地方。
fn script_candidates(shell: &Shell, token: &TokenContext, line: &str) -> Vec<Candidate> {
    let words = command_words(line, token.range.start);
    let prefix = token.text.as_str();
    let entries = match words
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        // 包管理器的 run 子命令
        ["npm" | "pnpm" | "yarn" | "bun", "run"] => scripts::package_scripts(&shell.cwd),
        // pnpm / yarn 可以省掉 run 直接写脚本名
        ["pnpm" | "yarn" | "bun"] => scripts::package_scripts(&shell.cwd),
        ["make" | "gmake"] => scripts::make_targets(&shell.cwd),
        _ => Vec::new(),
    };

    entries
        .into_iter()
        .filter(|(name, _)| name.starts_with(prefix))
        .map(|(name, detail)| Candidate {
            value: name.clone(),
            display: name,
            description: detail,
            kind: CandidateKind::Script,
            replace: token.range.clone(),
        })
        .collect()
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
            t!("dir", "目录").to_string()
        } else {
            match entry.metadata().map(|m| m.len()) {
                Ok(size) => tf!("file · {}", "文件 · {}", util::format_size(size)),
                Err(_) => t!("file", "文件").to_string(),
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

    // ── 子命令 / 项目脚本 / 预置脚本 ─────────────────────────────────

    fn kinds_of(candidates: &[Candidate], kind: CandidateKind) -> Vec<&str> {
        candidates
            .iter()
            .filter(|c| c.kind == kind)
            .map(|c| c.value.as_str())
            .collect()
    }

    #[test]
    fn extracts_the_command_chain() {
        assert_eq!(command_words("git ", 4), vec!["git"]);
        assert_eq!(command_words("git remote ", 11), vec!["git", "remote"]);
        // 选项要跳过，否则 `git -C x status` 查不到表
        assert_eq!(command_words("git -v ", 7), vec!["git"]);
        // 管道后面是一条新命令
        assert_eq!(command_words("ls | git ", 9), vec!["git"]);
        assert!(command_words("", 0).is_empty());
    }

    #[test]
    fn suggests_git_subcommands() {
        let mut shell = Shell::new(Config::default(), false);
        let config = Config::default();

        // `git ` 之后直接给子命令
        let candidates = complete(&mut shell, "git ", 4, &config);
        let subs = kinds_of(&candidates, CandidateKind::Subcommand);
        for expected in ["status", "log", "commit"] {
            assert!(subs.contains(&expected), "缺少 git {expected}：{subs:?}");
        }

        // 带前缀时只给匹配的
        let candidates = complete(&mut shell, "git sta", 7, &config);
        let subs = kinds_of(&candidates, CandidateKind::Subcommand);
        assert!(subs.contains(&"status"));
        assert!(!subs.contains(&"log"), "log 不该匹配前缀 sta");

        // 子命令要排在路径候选前面：输入 `git s` 想要的是 status，
        // 不是当前目录里恰好以 s 开头的文件
        let candidates = complete(&mut shell, "git s", 5, &config);
        let first_sub = candidates
            .iter()
            .position(|c| c.kind == CandidateKind::Subcommand);
        let first_path = candidates
            .iter()
            .position(|c| matches!(c.kind, CandidateKind::File | CandidateKind::Directory));
        if let (Some(sub), Some(path)) = (first_sub, first_path) {
            assert!(sub < path, "子命令应排在路径候选之前");
        }
    }

    #[test]
    fn suggests_second_level_subcommands() {
        let mut shell = Shell::new(Config::default(), false);
        let config = Config::default();

        let candidates = complete(&mut shell, "git remote ", 11, &config);
        let subs = kinds_of(&candidates, CandidateKind::Subcommand);
        assert!(subs.contains(&"add"), "git remote 应给出 add：{subs:?}");
        assert!(subs.contains(&"prune"));

        // 二级命令后面不该再把一级子命令列一遍
        let candidates = complete(&mut shell, "git status ", 11, &config);
        let subs = kinds_of(&candidates, CandidateKind::Subcommand);
        assert!(subs.is_empty(), "git status 之后不该有子命令：{subs:?}");
    }

    /// 配置里的同名项要盖掉内置说明，否则用户改不动内置文案。
    #[test]
    fn config_completions_override_builtin_description() {
        let mut shell = Shell::new(Config::default(), false);
        let config = Config::parse_str(
            r#"
[completions.git]
status = "my own wording"
sync = "custom helper"

[completions."git remote"]
mine = "team script"
"#,
        )
        .expect("解析失败");

        let candidates = complete(&mut shell, "git ", 4, &config);
        let status = candidates
            .iter()
            .find(|c| c.value == "status" && c.kind == CandidateKind::Subcommand)
            .expect("status 候选存在");
        assert_eq!(status.description, "my own wording");

        // 配置还能新增内置表里没有的项
        let subs = kinds_of(&candidates, CandidateKind::Subcommand);
        assert!(subs.contains(&"sync"));

        // 带空格的命令链要能用引号键配置
        let candidates = complete(&mut shell, "git remote ", 11, &config);
        let subs = kinds_of(&candidates, CandidateKind::Subcommand);
        assert!(subs.contains(&"mine"), "引号键配置未生效：{subs:?}");
    }

    #[test]
    fn completes_package_scripts_and_make_targets() {
        let sandbox = std::env::temp_dir().join(format!(
            "cmds-projscripts-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::remove_dir_all(&sandbox).ok();
        std::fs::create_dir_all(&sandbox).unwrap();
        std::fs::write(
            sandbox.join("package.json"),
            br#"{"scripts": {"dev": "vite", "build": "vite build"}}"#,
        )
        .unwrap();
        std::fs::write(sandbox.join("Makefile"), b"release: ## ship it\n\ttrue\n").unwrap();

        let mut shell = Shell::new(Config::default(), false);
        shell.cwd = sandbox.clone();
        let config = Config::default();

        let candidates = complete(&mut shell, "npm run ", 8, &config);
        let scripts = kinds_of(&candidates, CandidateKind::Script);
        assert!(scripts.contains(&"dev"), "npm run 应补出 dev：{scripts:?}");
        assert!(scripts.contains(&"build"));

        // pnpm 可以省掉 run
        let candidates = complete(&mut shell, "pnpm ", 5, &config);
        let scripts = kinds_of(&candidates, CandidateKind::Script);
        assert!(scripts.contains(&"dev"));

        let candidates = complete(&mut shell, "make ", 5, &config);
        let targets = candidates
            .iter()
            .find(|c| c.value == "release" && c.kind == CandidateKind::Script)
            .expect("make target 候选存在");
        assert_eq!(targets.description, "ship it", "应该用 ## 后的自文档注释");

        std::fs::remove_dir_all(&sandbox).ok();
    }

    /// 预置脚本按整行召回，所以不必记住缩写——敲命令本身也能找到。
    #[test]
    fn recalls_preset_scripts_by_name_or_command() {
        let mut shell = Shell::new(Config::default(), false);
        let config = Config::parse_str(
            r#"
[scripts]
gst = "git status --short --branch"
"#,
        )
        .expect("解析失败");

        // 按缩写找
        let candidates = complete(&mut shell, "gst", 3, &config);
        let script = candidates
            .iter()
            .find(|c| c.kind == CandidateKind::Script)
            .expect("应召回预置脚本");
        assert_eq!(script.value, "git status --short --branch");
        assert_eq!(script.replace, 0..3, "预置脚本要替换整行");

        // 直接敲命令前缀也能找到
        let candidates = complete(&mut shell, "git stat", 8, &config);
        assert!(
            candidates.iter().any(
                |c| c.kind == CandidateKind::Script && c.value == "git status --short --branch"
            ),
            "按命令前缀也该召回预置脚本"
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

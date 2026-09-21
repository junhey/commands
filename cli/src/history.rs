//! 历史记录：持久化、去重、前缀/模糊检索与频率排序。
//!
//! 文件格式：每行 `<unix 秒>\t<命令>`，命令中的换行会转义为 `\n`。

use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use crate::config::HistoryConfig;
use crate::util;

#[derive(Debug, Clone)]
pub struct Entry {
    pub command: String,
    pub timestamp: u64,
}

#[derive(Debug)]
pub struct History {
    entries: Vec<Entry>,
    counts: HashMap<String, u32>,
    first_words: HashMap<String, u32>,
    path: Option<PathBuf>,
    max_entries: usize,
    dedup: bool,
    ignore_space: bool,
    ignore_commands: Vec<String>,
}

impl History {
    fn empty(config: &HistoryConfig, path: Option<PathBuf>) -> Self {
        Self {
            entries: Vec::new(),
            counts: HashMap::new(),
            first_words: HashMap::new(),
            path,
            max_entries: config.max_entries.max(1),
            dedup: config.dedup,
            ignore_space: config.ignore_space,
            ignore_commands: config.ignore_commands.clone(),
        }
    }

    /// 从文件加载历史。
    pub fn load(config: &HistoryConfig, path: PathBuf) -> Self {
        let mut history = Self::empty(config, Some(path.clone()));
        if let Ok(file) = File::open(&path) {
            for line in BufReader::new(file).lines().map_while(Result::ok) {
                if let Some(entry) = decode_line(&line) {
                    history.push(entry);
                }
            }
        }
        history
    }

    /// 不落盘的历史（`cmds -c` 等一次性场景）。
    pub fn ephemeral(config: &HistoryConfig) -> Self {
        Self::empty(config, None)
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    fn push(&mut self, entry: Entry) {
        if self.dedup {
            if let Some(index) = self.entries.iter().position(|e| e.command == entry.command) {
                self.entries.remove(index);
            }
        }
        *self.counts.entry(entry.command.clone()).or_insert(0) += 1;
        if let Some(word) = first_word(&entry.command) {
            *self.first_words.entry(word).or_insert(0) += 1;
        }
        self.entries.push(entry);
        while self.entries.len() > self.max_entries {
            self.entries.remove(0);
        }
    }

    /// 记录一条命令（含过滤规则），必要时追加写入文件。
    pub fn record(&mut self, raw: &str) {
        if self.ignore_space && raw.starts_with(' ') {
            return;
        }
        let command = raw.trim();
        if command.is_empty() {
            return;
        }
        if self
            .ignore_commands
            .iter()
            .any(|ignored| ignored == command)
        {
            return;
        }
        if self
            .entries
            .last()
            .is_some_and(|last| last.command == command)
        {
            return;
        }
        let entry = Entry {
            command: command.to_string(),
            timestamp: util::now_secs(),
        };
        let line = encode_line(&entry);
        self.push(entry);
        self.append_line(&line);
    }

    fn append_line(&self, line: &str) {
        let Some(path) = &self.path else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(file, "{line}");
        }
    }

    /// 重写历史文件（裁剪到 max_entries）。
    pub fn rewrite(&self) -> std::io::Result<()> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = File::create(path)?;
        for entry in &self.entries {
            writeln!(file, "{}", encode_line(entry))?;
        }
        Ok(())
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.counts.clear();
        self.first_words.clear();
        let _ = self.rewrite();
    }

    /// 自动建议用：最近一条以 `prefix` 开头且更长的命令。
    pub fn latest_with_prefix(&self, prefix: &str) -> Option<&str> {
        if prefix.is_empty() {
            return None;
        }
        self.entries
            .iter()
            .rev()
            .find(|e| e.command.len() > prefix.len() && e.command.starts_with(prefix))
            .map(|e| e.command.as_str())
    }

    /// 上下键历史浏览：按前缀过滤，新 -> 旧，去重。
    pub fn prefix_matches(&self, prefix: &str, limit: usize) -> Vec<&str> {
        let mut seen: Vec<&str> = Vec::new();
        for entry in self.entries.iter().rev() {
            if !entry.command.starts_with(prefix) {
                continue;
            }
            if seen.contains(&entry.command.as_str()) {
                continue;
            }
            seen.push(entry.command.as_str());
            if seen.len() >= limit {
                break;
            }
        }
        seen
    }

    /// 候选菜单用：前缀优先、其次子串，结合使用频率与新鲜度排序。
    pub fn ranked_matches(&self, needle: &str, limit: usize) -> Vec<&str> {
        let total = self.entries.len().max(1) as f64;
        let mut scored: Vec<(&str, f64)> = Vec::new();
        for (index, entry) in self.entries.iter().enumerate() {
            let command = entry.command.as_str();
            if command == needle {
                continue;
            }
            let base = if needle.is_empty() {
                10.0
            } else if command.starts_with(needle) {
                1000.0
            } else if contains_ignore_case(command, needle) {
                200.0
            } else {
                continue;
            };
            let count = *self.counts.get(command).unwrap_or(&1) as f64;
            let freshness = index as f64 / total * 100.0;
            let score = base + count * 5.0 + freshness;
            match scored.iter_mut().find(|(c, _)| *c == command) {
                Some(slot) => slot.1 = slot.1.max(score),
                None => scored.push((command, score)),
            }
        }
        scored.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(b.0)));
        scored.truncate(limit);
        scored.into_iter().map(|(c, _)| c).collect()
    }

    /// 命令位置补全用：历史里出现过的首个单词 + 次数。
    pub fn command_words(&self) -> Vec<(&str, u32)> {
        let mut words: Vec<(&str, u32)> = self
            .first_words
            .iter()
            .map(|(k, v)| (k.as_str(), *v))
            .collect();
        words.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
        words
    }

    pub fn use_count(&self, command: &str) -> u32 {
        *self.counts.get(command).unwrap_or(&0)
    }
}

fn contains_ignore_case(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    haystack.to_lowercase().contains(&needle.to_lowercase())
}

fn first_word(command: &str) -> Option<String> {
    command
        .split_whitespace()
        .next()
        .filter(|w| !w.is_empty())
        .map(|w| w.to_string())
}

fn encode_line(entry: &Entry) -> String {
    let escaped = entry
        .command
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\t', "\\t");
    format!("{}\t{}", entry.timestamp, escaped)
}

fn decode_line(line: &str) -> Option<Entry> {
    let line = line.trim_end_matches(['\r', '\n']);
    if line.trim().is_empty() {
        return None;
    }
    let (timestamp, raw) = match line.split_once('\t') {
        Some((ts, rest)) => (ts.trim().parse::<u64>().unwrap_or(0), rest),
        None => (0, line),
    };
    let command = unescape(raw);
    if command.trim().is_empty() {
        return None;
    }
    Some(Entry { command, timestamp })
}

fn unescape(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('\\') => out.push('\\'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn history() -> History {
        History::ephemeral(&Config::default().history)
    }

    #[test]
    fn records_and_dedups() {
        let mut h = history();
        h.record("git status");
        h.record("cargo build");
        h.record("git status");
        assert_eq!(h.len(), 2);
        assert_eq!(h.entries().last().unwrap().command, "git status");
        assert_eq!(h.use_count("git status"), 2);
    }

    #[test]
    fn filters_ignored_entries() {
        let mut h = history();
        h.record("  leading space");
        h.record("exit");
        h.record("");
        assert_eq!(h.len(), 0);
    }

    #[test]
    fn suggests_latest_prefix_match() {
        let mut h = history();
        h.record("cargo test");
        h.record("cargo build --release");
        assert_eq!(
            h.latest_with_prefix("cargo "),
            Some("cargo build --release")
        );
        assert_eq!(h.latest_with_prefix("zzz"), None);
    }

    #[test]
    fn ranks_prefix_above_substring() {
        let mut h = history();
        h.record("grep -R foo .");
        h.record("cargo run -- grep");
        let ranked = h.ranked_matches("grep", 5);
        assert_eq!(ranked.first(), Some(&"grep -R foo ."));
        assert!(ranked.contains(&"cargo run -- grep"));
    }

    #[test]
    fn collects_command_words() {
        let mut h = history();
        h.record("git status");
        h.record("git diff");
        h.record("ls -la");
        let words = h.command_words();
        assert_eq!(words.first().map(|(w, c)| (*w, *c)), Some(("git", 2)));
    }

    #[test]
    fn round_trips_multiline_commands() {
        let entry = Entry {
            command: "echo a\nb\tc\\d".to_string(),
            timestamp: 42,
        };
        let decoded = decode_line(&encode_line(&entry)).unwrap();
        assert_eq!(decoded.command, entry.command);
        assert_eq!(decoded.timestamp, 42);
    }
}

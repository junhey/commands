//! fish 风格自动建议：输入时给出灰色的历史/补全提示。

use super::complete::{self, CandidateKind};
use crate::config::Config;
use crate::shell::Shell;

/// 返回完整建议行（一定以 `line` 为前缀），没有建议时返回 None。
pub fn autosuggestion(shell: &mut Shell, line: &str, config: &Config) -> Option<String> {
    if !config.autosuggest.enabled || line.is_empty() || line.trim().is_empty() {
        return None;
    }
    for source in &config.autosuggest.sources {
        let found = match source.as_str() {
            "history" => shell
                .history
                .latest_with_prefix(line)
                .map(|command| command.to_string()),
            "completion" => from_completion(shell, line, config),
            _ => None,
        };
        if let Some(suggestion) = found {
            if suggestion.len() > line.len() && suggestion.starts_with(line) {
                return Some(suggestion);
            }
        }
    }
    None
}

/// 当前词只有一个补全候选时，把它当作建议。
fn from_completion(shell: &mut Shell, line: &str, config: &Config) -> Option<String> {
    let cursor = line.len();
    let candidates = complete::complete(shell, line, cursor, config);
    let usable: Vec<&complete::Candidate> = candidates
        .iter()
        .filter(|candidate| {
            candidate.kind != CandidateKind::History && candidate.replace.end == cursor
        })
        .collect();
    let first = usable.first()?;
    if usable.len() > 1 {
        return None;
    }
    let mut suggestion = line.to_string();
    suggestion.replace_range(first.replace.clone(), &first.value);
    Some(suggestion)
}

/// 采纳建议中的下一个词（Alt-→ 行为）。
pub fn next_word_boundary(tail: &str) -> usize {
    let mut index = 0usize;
    let mut chars = tail.char_indices().peekable();
    // 先吃掉前导空白
    while let Some((offset, c)) = chars.peek().copied() {
        if c.is_whitespace() {
            index = offset + c.len_utf8();
            chars.next();
        } else {
            break;
        }
    }
    for (offset, c) in chars {
        if c.is_whitespace() {
            return offset;
        }
        index = offset + c.len_utf8();
    }
    index
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn suggests_from_history() {
        let mut shell = Shell::new(Config::default(), false);
        shell.history.record("cargo build --release");
        let config = Config::default();
        assert_eq!(
            autosuggestion(&mut shell, "cargo bu", &config).as_deref(),
            Some("cargo build --release")
        );
    }

    #[test]
    fn falls_back_to_unique_completion() {
        let mut shell = Shell::new(Config::default(), false);
        let config = Config::default();
        // `histo` 只能补成内建 history
        let suggestion = autosuggestion(&mut shell, "histo", &config);
        assert_eq!(suggestion.as_deref(), Some("history"));
    }

    #[test]
    fn returns_none_without_match() {
        let mut shell = Shell::new(Config::default(), false);
        let config = Config::default();
        assert!(autosuggestion(&mut shell, "zzzz-no-match", &config).is_none());
        assert!(autosuggestion(&mut shell, "", &config).is_none());
    }

    #[test]
    fn respects_disabled_flag() {
        let mut shell = Shell::new(Config::default(), false);
        shell.history.record("ls -la");
        let mut config = Config::default();
        config.autosuggest.enabled = false;
        assert!(autosuggestion(&mut shell, "ls", &config).is_none());
    }

    #[test]
    fn finds_next_word_in_tail() {
        assert_eq!(next_word_boundary(" build --release"), 6);
        assert_eq!(&" build --release"[..6], " build");
        assert_eq!(next_word_boundary("abc"), 3);
    }
}

//! 轻量 glob：支持 `*`、`?`、`[abc]`、`[a-z]`、`[!abc]` 与跨目录 `**`。

use std::path::{Path, PathBuf};

pub fn is_pattern(text: &str) -> bool {
    text.contains('*') || text.contains('?') || text.contains('[')
}

/// 展开通配符；无匹配时返回空 Vec（调用方负责保留原字符串）。
pub fn expand(pattern: &str, cwd: &Path) -> Vec<String> {
    let normalized = if cfg!(windows) {
        pattern.replace('\\', "/")
    } else {
        pattern.to_string()
    };
    let absolute = Path::new(&normalized).is_absolute();
    let mut components: Vec<&str> = normalized.split('/').filter(|c| !c.is_empty()).collect();

    let base = if absolute {
        if normalized.starts_with('/') {
            PathBuf::from("/")
        } else {
            // Windows 盘符前缀，如 C:/...
            let root = components.remove(0);
            PathBuf::from(format!("{root}/"))
        }
    } else {
        cwd.to_path_buf()
    };

    let mut found: Vec<PathBuf> = Vec::new();
    walk(&base, &components, &mut found);
    found.sort();
    found.dedup();

    found
        .iter()
        .map(|path| {
            let text = if absolute {
                path.to_string_lossy().into_owned()
            } else {
                path.strip_prefix(cwd)
                    .unwrap_or(path)
                    .to_string_lossy()
                    .into_owned()
            };
            if cfg!(windows) {
                text.replace('\\', "/")
            } else {
                text
            }
        })
        .filter(|text| !text.is_empty())
        .collect()
}

fn walk(base: &Path, components: &[&str], out: &mut Vec<PathBuf>) {
    let Some((head, rest)) = components.split_first() else {
        if base.symlink_metadata().is_ok() {
            out.push(base.to_path_buf());
        }
        return;
    };

    match *head {
        "." => walk(base, rest, out),
        ".." => walk(&base.join(".."), rest, out),
        "**" => {
            walk(base, rest, out);
            let Ok(entries) = std::fs::read_dir(base) else {
                return;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                let hidden = entry.file_name().to_string_lossy().starts_with('.');
                if hidden {
                    continue;
                }
                if path.is_dir() {
                    walk(&path, components, out);
                }
            }
        }
        head if is_pattern(head) => {
            let Ok(entries) = std::fs::read_dir(base) else {
                return;
            };
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.starts_with('.') && !head.starts_with('.') {
                    continue;
                }
                if !match_name(head, &name) {
                    continue;
                }
                let path = entry.path();
                if rest.is_empty() {
                    out.push(path);
                } else if path.is_dir() {
                    walk(&path, rest, out);
                }
            }
        }
        head => walk(&base.join(head), rest, out),
    }
}

/// 单个文件名匹配。
pub fn match_name(pattern: &str, name: &str) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let name: Vec<char> = name.chars().collect();
    let (mut pi, mut ni) = (0usize, 0usize);
    let mut star: Option<(usize, usize)> = None;

    while ni < name.len() {
        let mut matched = false;
        if pi < pattern.len() {
            match pattern[pi] {
                '*' => {
                    star = Some((pi, ni));
                    pi += 1;
                    continue;
                }
                '?' => {
                    pi += 1;
                    ni += 1;
                    continue;
                }
                '[' => match match_class(&pattern, pi, name[ni]) {
                    Some((hit, next)) => {
                        if hit {
                            pi = next;
                            ni += 1;
                            matched = true;
                        }
                    }
                    None => {
                        if pattern[pi] == name[ni] {
                            pi += 1;
                            ni += 1;
                            matched = true;
                        }
                    }
                },
                c => {
                    if c == name[ni] {
                        pi += 1;
                        ni += 1;
                        matched = true;
                    }
                }
            }
        }
        if matched {
            continue;
        }
        match star {
            Some((sp, sn)) => {
                pi = sp + 1;
                ni = sn + 1;
                star = Some((sp, sn + 1));
            }
            None => return false,
        }
    }

    while pi < pattern.len() && pattern[pi] == '*' {
        pi += 1;
    }
    pi == pattern.len()
}

/// 解析 `[...]` 字符类；返回 (是否命中, 类结束后的下标)，无闭合时返回 None。
fn match_class(pattern: &[char], start: usize, target: char) -> Option<(bool, usize)> {
    let mut index = start + 1;
    let negated = matches!(pattern.get(index), Some('!') | Some('^'));
    if negated {
        index += 1;
    }
    let mut hit = false;
    let mut first = true;
    while index < pattern.len() {
        let c = pattern[index];
        if c == ']' && !first {
            return Some((hit != negated, index + 1));
        }
        first = false;
        if pattern.get(index + 1) == Some(&'-')
            && pattern.get(index + 2).is_some_and(|end| *end != ']')
        {
            let end = pattern[index + 2];
            if c <= target && target <= end {
                hit = true;
            }
            index += 3;
            continue;
        }
        if c == target {
            hit = true;
        }
        index += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_wildcards() {
        assert!(match_name("*.rs", "main.rs"));
        assert!(!match_name("*.rs", "main.py"));
        assert!(match_name("mai?.rs", "main.rs"));
        assert!(match_name("*", "anything"));
        assert!(match_name("a*b*c", "azzbzzc"));
        assert!(!match_name("a*b*c", "azzbzz"));
    }

    #[test]
    fn matches_classes() {
        assert!(match_name("[abc].txt", "b.txt"));
        assert!(!match_name("[abc].txt", "d.txt"));
        assert!(match_name("[a-z]*.rs", "main.rs"));
        assert!(match_name("[!x]y", "ay"));
        assert!(!match_name("[!x]y", "xy"));
        assert!(match_name("[unclosed", "[unclosed"));
    }

    #[test]
    fn detects_patterns() {
        assert!(is_pattern("*.rs"));
        assert!(!is_pattern("Cargo.toml"));
    }

    #[test]
    fn expands_in_directory() {
        let dir = std::env::temp_dir().join(format!("cmds-glob-{}", std::process::id()));
        let nested = dir.join("sub");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(dir.join("a.txt"), "a").unwrap();
        std::fs::write(dir.join("b.md"), "b").unwrap();
        std::fs::write(nested.join("c.txt"), "c").unwrap();

        let mut txt = expand("*.txt", &dir);
        txt.sort();
        assert_eq!(txt, vec!["a.txt".to_string()]);

        let deep = expand("**/*.txt", &dir);
        assert!(deep.iter().any(|p| p.ends_with("sub/c.txt")));

        let _ = std::fs::remove_dir_all(&dir);
    }
}

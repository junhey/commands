//! 项目自带的脚本：package.json 的 `scripts`、Makefile 的 target。
//!
//! 这些名字只有当前项目自己知道，又最常用——`npm run <Tab>` 能直接列出来，
//! 省掉每次去翻 package.json。
//!
//! 有缓存：补全是按键触发的，autosuggest 每敲一个字符就会走一遍，
//! 没缓存的话每次按键都要读一遍文件。

use crate::json;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// 缓存 2 秒。改完 package.json 立刻按 Tab 也能看到新脚本，
/// 同时挡掉连续按键造成的重复读盘。
const CACHE_TTL_SECS: u64 = 2;

type Entries = Vec<(String, String)>;

fn cache() -> &'static Mutex<HashMap<PathBuf, (u64, Entries)>> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, (u64, Entries)>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn cached(key: PathBuf, compute: impl FnOnce() -> Entries) -> Entries {
    let now = crate::util::now_secs();
    if let Ok(map) = cache().lock() {
        if let Some((stamp, entries)) = map.get(&key) {
            if now.saturating_sub(*stamp) < CACHE_TTL_SECS {
                return entries.clone();
            }
        }
    }
    let entries = compute();
    if let Ok(mut map) = cache().lock() {
        map.insert(key, (now, entries.clone()));
    }
    entries
}

/// 从 cwd 向上找最近的文件。monorepo 里 `npm run` 用的是最近那个
/// package.json，补全也得跟着走，不能只看当前目录。
fn find_upwards(start: &Path, names: &[&str]) -> Option<PathBuf> {
    let mut current = Some(start);
    while let Some(dir) = current {
        for name in names {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
        // 到仓库根就停：再往上就是别人的项目了
        if dir.join(".git").exists() {
            break;
        }
        current = dir.parent();
    }
    None
}

/// package.json 里的 scripts，返回 (脚本名, 脚本内容)。
pub fn package_scripts(cwd: &Path) -> Entries {
    let Some(path) = find_upwards(cwd, &["package.json"]) else {
        return Vec::new();
    };
    cached(path.clone(), || {
        let Ok(text) = std::fs::read_to_string(&path) else {
            return Vec::new();
        };
        json::top_level_string_map(&text, "scripts")
            .into_iter()
            .map(|(name, command)| (name, crate::util::truncate_middle(&command, 48)))
            .collect()
    })
}

/// Makefile 的 target。描述优先取 `target: ## 说明` 里的自文档注释——
/// 这是 Makefile 里很常见的写法，有就用上。
pub fn make_targets(cwd: &Path) -> Entries {
    let Some(path) = find_upwards(cwd, &["Makefile", "makefile", "GNUmakefile"]) else {
        return Vec::new();
    };
    cached(path.clone(), || {
        let Ok(text) = std::fs::read_to_string(&path) else {
            return Vec::new();
        };
        parse_make_targets(&text)
    })
}

fn parse_make_targets(text: &str) -> Entries {
    let mut out: Entries = Vec::new();
    for line in text.lines() {
        // target 必须顶格写，缩进的是配方（recipe）
        if line.starts_with([' ', '\t']) || line.trim().is_empty() {
            continue;
        }
        let Some(colon) = line.find(':') else {
            continue;
        };
        // 变量赋值也含冒号：`CC := gcc`、`X ::= y`。
        // 注意别把 `target:: deps`（GNU make 的双冒号规则）误判成赋值——
        // 区别在于再跳过一个冒号之后是不是 `=`。
        let after = line[colon + 1..].trim_start();
        let after = after
            .strip_prefix(':')
            .map(str::trim_start)
            .unwrap_or(after);
        if after.starts_with('=') {
            continue;
        }
        let name = line[..colon].trim();
        if name.is_empty() || name.contains([' ', '\t']) {
            // `a b: dep` 这种多目标规则不好归类，跳过
            continue;
        }
        // 特殊目标（.PHONY / .DEFAULT_GOAL）和模式规则不是能直接跑的
        if name.starts_with('.') || name.contains('%') || name.contains('$') {
            continue;
        }
        let description = line[colon + 1..]
            .split_once("##")
            .map(|(_, comment)| comment.trim().to_string())
            .unwrap_or_else(|| t!("Makefile target", "Makefile 目标").to_string());
        if !out.iter().any(|(existing, _)| existing == name) {
            out.push((name.to_string(), description));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_targets_and_self_documenting_comments() {
        let makefile = "\
.PHONY: build test
CC := gcc
LDFLAGS ::= -lm

build: ## compile everything
\tcargo build

test: build
\tcargo test

%.o: %.c
\t$(CC) -c $<

$(BIN): build
\ttrue

deploy:## no space before comment
\ttrue
";
        let targets = parse_make_targets(makefile);
        let names: Vec<&str> = targets.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, vec!["build", "test", "deploy"]);

        // `##` 注释被当成描述
        assert_eq!(targets[0].1, "compile everything");
        assert_eq!(targets[2].1, "no space before comment");
        // 没有注释就给个通用说明，而不是空字符串（菜单里会是空白一行）
        assert!(!targets[1].1.is_empty());
    }

    #[test]
    fn ignores_variable_assignments() {
        // 这些都含冒号，但都不是 target
        let makefile = "CC := gcc\nX ::= y\nZ = 1\nPREFIX ?= /usr/local\n";
        assert!(parse_make_targets(makefile).is_empty());
    }

    /// `target:: deps` 是 GNU make 的双冒号规则，是真 target，
    /// 不能和 `VAR ::= value` 混为一谈。
    #[test]
    fn keeps_double_colon_rules() {
        let targets = parse_make_targets("clean:: \n\trm -rf out\n");
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].0, "clean");
    }

    #[test]
    fn skips_indented_recipe_lines() {
        // 配方里出现的冒号不该被当成 target
        let makefile = "build:\n\techo 'a: b'\n\tcurl http://x\n";
        let targets = parse_make_targets(makefile);
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].0, "build");
    }

    #[test]
    fn reads_package_scripts_from_disk() {
        let dir = std::env::temp_dir().join(format!(
            "cmds-scripts-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("package.json"),
            br#"{"scripts": {"dev": "vite", "build": "vite build"}}"#,
        )
        .unwrap();

        let scripts = package_scripts(&dir);
        let names: Vec<&str> = scripts.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, vec!["dev", "build"]);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn finds_package_json_in_parent_directory() {
        // monorepo：在子目录里也该找到上层的 package.json
        let root = std::env::temp_dir().join(format!(
            "cmds-monorepo-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::remove_dir_all(&root).ok();
        let nested = root.join("packages/app/src");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(
            root.join("package.json"),
            br#"{"scripts":{"ci":"turbo run"}}"#,
        )
        .unwrap();

        let scripts = package_scripts(&nested);
        assert_eq!(scripts.len(), 1, "应该找到上层的 package.json");
        assert_eq!(scripts[0].0, "ci");

        std::fs::remove_dir_all(&root).ok();
    }

    /// 向上查找要在仓库根停下，不然会跑到别人的项目里去。
    #[test]
    fn stops_at_repository_root() {
        let outer = std::env::temp_dir().join(format!(
            "cmds-stop-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::remove_dir_all(&outer).ok();
        let repo = outer.join("repo");
        let inner = repo.join("src");
        std::fs::create_dir_all(&inner).unwrap();
        std::fs::create_dir_all(repo.join(".git")).unwrap();
        // package.json 在仓库**外面**，不该被找到
        std::fs::write(
            outer.join("package.json"),
            br#"{"scripts":{"outside":"x"}}"#,
        )
        .unwrap();

        assert!(package_scripts(&inner).is_empty());
        std::fs::remove_dir_all(&outer).ok();
    }
}

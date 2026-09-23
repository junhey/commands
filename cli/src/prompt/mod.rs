//! 模块化提示符（借鉴 starship 的 `format` 占位符设计）。

pub mod modules;

use crate::config::Config;
use crate::style;
use crate::util::EnvMap;
use std::path::Path;

pub struct Context<'a> {
    pub config: &'a Config,
    pub cwd: &'a Path,
    pub status: i32,
    pub duration_ms: u128,
    pub jobs: usize,
    pub env: &'a EnvMap,
}

#[derive(Debug, Clone, Default)]
pub struct Prompt {
    /// 最后一行之前的内容（含换行），交互式下一次性打印。
    pub leading: String,
    /// 最后一行，行编辑器每次重绘都会用到。
    pub last_line: String,
    /// 右侧提示符。
    pub right: String,
}

impl Prompt {
    pub fn full(&self) -> String {
        format!("{}{}", self.leading, self.last_line)
    }
}

/// 渲染提示符。
pub fn render(ctx: &Context<'_>) -> Prompt {
    let mut text = render_format(&ctx.config.format, ctx);
    if ctx.config.add_newline {
        text.insert(0, '\n');
    }
    let (leading, last_line) = match text.rfind('\n') {
        Some(index) => (text[..=index].to_string(), text[index + 1..].to_string()),
        None => (String::new(), text),
    };
    Prompt {
        leading,
        last_line,
        right: render_format(&ctx.config.right_format, ctx),
    }
}

/// `$all` 展开成的模块链——提示符第一行的「信息区」。
///
/// 存在的理由：配置文件里如果写死了完整的模块列表，升级后新增的模块永远不会
/// 出现（旧配置会盖掉新默认值）。把 `format = "$all$line_break$character"`
/// 写进模板，用户就能自动跟上后续版本新加的模块。
///
/// 默认关闭的模块（os / package / shlvl / git_commit / time）也列在这里：
/// 它们靠自己的 `enabled` 控制显隐，摆进来才能做到「改一个 enabled 就看得见」，
/// 而不是改完配置发现还得先搞懂占位符叫什么、该插在哪。
///
/// `character` / `container` / `shlvl` 不在这里：它们属于第二行，贴着光标。
/// `identity` 也不在——它是 `$username$hostname` 的紧凑替代，同时出现会重复。
pub const ALL_MODULES: &str = concat!(
    "$os$username$hostname$dir",
    "$git_branch$git_commit$git_state$git_status",
    "$venv$languages$package",
    "$jobs$cmd_duration$time$status",
);

/// 解析 `$module` / `${module}` 占位符。
pub fn render_format(format: &str, ctx: &Context<'_>) -> String {
    let chars: Vec<char> = format.chars().collect();
    let mut out = String::new();
    let mut index = 0usize;

    while index < chars.len() {
        match chars[index] {
            '$' => {
                index += 1;
                let braced = chars.get(index) == Some(&'{');
                if braced {
                    index += 1;
                }
                let mut name = String::new();
                while let Some(c) = chars.get(index) {
                    if braced && *c == '}' {
                        index += 1;
                        break;
                    }
                    if c.is_alphanumeric() || *c == '_' {
                        name.push(*c);
                        index += 1;
                    } else {
                        break;
                    }
                }
                if name.is_empty() {
                    out.push('$');
                    continue;
                }
                if name == "all" {
                    out.push_str(&render_format(ALL_MODULES, ctx));
                    continue;
                }
                if let Some(rendered) = modules::render_module(&name, ctx) {
                    out.push_str(&rendered);
                }
            }
            '\\' => {
                match chars.get(index + 1) {
                    Some('n') => out.push('\n'),
                    Some('t') => out.push('\t'),
                    Some('e') => out.push('\x1b'),
                    Some(other) => out.push(*other),
                    None => out.push('\\'),
                }
                index += 2;
            }
            c => {
                out.push(c);
                index += 1;
            }
        }
    }
    out
}

/// 把右侧提示符贴到前导行的右端（交互式下使用）。
pub fn attach_right(prompt: &mut Prompt, width: usize) {
    if prompt.right.trim().is_empty() {
        prompt.right.clear();
        return;
    }
    let right = std::mem::take(&mut prompt.right);
    let right_width = style::visible_width(&right);

    if prompt.leading.is_empty() {
        let pad = width.saturating_sub(right_width);
        prompt.leading = format!("{}{right}\n", " ".repeat(pad));
        return;
    }

    let trimmed = prompt.leading.trim_end_matches('\n');
    let (head, last) = match trimmed.rfind('\n') {
        Some(index) => (&trimmed[..=index], &trimmed[index + 1..]),
        None => ("", trimmed),
    };
    let used = style::visible_width(last);
    let pad = width.saturating_sub(used + right_width).max(1);
    prompt.leading = format!("{head}{last}{}{right}\n", " ".repeat(pad));
}

/// 供 bash/zsh 使用时，用 readline 的忽略标记包裹 ANSI，保证行宽计算正确。
pub fn wrap_for_readline(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 16);
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\x1b' {
            out.push(c);
            continue;
        }
        let mut escape = String::from(c);
        for e in chars.by_ref() {
            escape.push(e);
            if e.is_ascii_alphabetic() {
                break;
            }
        }
        out.push('\x01');
        out.push_str(&escape);
        out.push('\x02');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util;
    use std::path::PathBuf;

    fn ctx_config(format: &str) -> Config {
        let mut config = Config {
            format: format.to_string(),
            add_newline: false,
            ..Config::default()
        };
        config.git.enabled = false;
        config.languages.enabled = false;
        config
    }

    #[test]
    fn renders_literal_and_modules() {
        style::set_color_enabled(false);
        let config = ctx_config("[cmds] $status$character");
        let env = util::EnvMap::new();
        let cwd = PathBuf::from(".");
        let prompt = render(&Context {
            config: &config,
            cwd: &cwd,
            status: 1,
            duration_ms: 0,
            jobs: 0,
            env: &env,
        });
        assert!(prompt.last_line.starts_with("[cmds] "));
        assert!(prompt.last_line.contains('1'));
        assert!(prompt.last_line.contains('❯'));
    }

    #[test]
    fn splits_leading_lines() {
        style::set_color_enabled(false);
        let config = ctx_config("first$line_break$character");
        let env = util::EnvMap::new();
        let cwd = PathBuf::from(".");
        let prompt = render(&Context {
            config: &config,
            cwd: &cwd,
            status: 0,
            duration_ms: 0,
            jobs: 0,
            env: &env,
        });
        assert_eq!(prompt.leading, "first\n");
        assert!(prompt.last_line.contains('❯'));
    }

    /// `$all` 必须真的展开成模块，而不是被当作未知占位符吃掉。
    #[test]
    fn expands_all_placeholder() {
        style::set_color_enabled(false);
        let mut config = ctx_config("$all$character");
        config.identity.show_user = crate::config::Visibility::Always;
        let env: util::EnvMap = [("USER".to_string(), "root".to_string())]
            .into_iter()
            .collect();
        let cwd = PathBuf::from(".");
        let prompt = render(&Context {
            config: &config,
            cwd: &cwd,
            status: 0,
            duration_ms: 0,
            jobs: 0,
            env: &env,
        });
        assert!(
            prompt.last_line.contains("root in "),
            "$all 应该展开出 username 模块，实际：{:?}",
            prompt.last_line
        );
    }

    /// 默认 format 必须用 `$all` 而不是写死模块列表——写死的话，
    /// 以后新增的模块对着老配置永远不会出现。
    #[test]
    fn default_format_uses_all_placeholder() {
        let config = Config::default();
        assert!(
            config.format.contains("$all"),
            "默认 format 应该用 $all：{}",
            config.format
        );
    }

    /// 实现了模块却忘了接进提示符，是很容易犯又很难发现的漏洞（功能在，但看不见）。
    #[test]
    fn every_module_is_reachable() {
        let config = Config::default();
        let reachable = format!("{ALL_MODULES}{}", config.format);
        // identity 是 $username$hostname 的紧凑替代，两者同时出现会重复显示，
        // 所以它是唯一刻意不进默认 format 的模块。
        const OPT_IN_ONLY: &[&str] = &["identity"];
        for name in modules::MODULE_NAMES {
            if OPT_IN_ONLY.contains(name) {
                continue;
            }
            assert!(
                reachable.contains(&format!("${name}")),
                "模块 {name} 没有接进默认 format 或 $all"
            );
        }
    }

    #[test]
    fn wraps_ansi_for_readline() {
        let wrapped = wrap_for_readline("\x1b[32m❯\x1b[0m ");
        assert!(wrapped.starts_with('\x01'));
        assert!(wrapped.contains("\x02❯\x01"));
    }
}

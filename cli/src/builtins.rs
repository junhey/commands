//! 内建命令。内建在管道中也可用（输出会被捕获后送往下游）。

use crate::config::{Config, DEFAULT_TEMPLATE};
use crate::exec;
use crate::shell::Shell;
use crate::util;
use std::io::Write;

pub struct BuiltinIo<'a> {
    pub stdout: &'a mut dyn Write,
    pub stderr: &'a mut dyn Write,
}

impl BuiltinIo<'_> {
    pub fn out(&mut self, text: &str) {
        let _ = writeln!(self.stdout, "{text}");
    }

    pub fn err(&mut self, text: &str) {
        let _ = writeln!(self.stderr, "cmds: {text}");
    }
}

pub type BuiltinFn = fn(&mut Shell, &[String], &mut BuiltinIo<'_>) -> i32;

/// 内建命令清单（名称 + 一句话说明），用于 `help`、补全与 `type`。
pub const BUILTINS: &[(&str, &str)] = &[
    ("abbr", "查看/设置缩写，输入后按空格展开"),
    ("alias", "查看/设置别名"),
    ("cd", "切换目录，cd - 回到上一个目录"),
    ("clear", "清屏"),
    ("config", "配置管理：path / init / reload / show"),
    ("echo", "输出文本，支持 -n / -e"),
    ("exit", "退出 shell，可带退出码"),
    ("export", "设置环境变量，也可写作 set"),
    ("false", "什么都不做，返回 1"),
    ("help", "查看使用说明与快捷键"),
    ("history", "查看/搜索/清空历史"),
    ("jobs", "查看后台任务"),
    ("pwd", "打印当前目录"),
    ("set", "设置环境变量（export 的别名）"),
    ("source", "在当前 shell 执行脚本，别名 ."),
    ("true", "什么都不做，返回 0"),
    ("type", "查看命令类型，别名 which"),
    ("unabbr", "删除缩写"),
    ("unalias", "删除别名"),
    ("unset", "删除环境变量"),
    ("which", "查看命令类型"),
];

pub fn lookup(name: &str) -> Option<BuiltinFn> {
    Some(match name {
        "abbr" => abbr as BuiltinFn,
        "alias" => alias,
        "cd" => cd,
        "clear" => clear,
        "config" => config,
        "echo" => echo,
        "exit" => exit,
        "export" | "set" => export,
        "false" => bool_false,
        "help" => help,
        "history" => history,
        "jobs" => jobs,
        "pwd" => pwd,
        "source" | "." => source,
        "true" => bool_true,
        "type" | "which" => type_of,
        "unabbr" => unabbr,
        "unalias" => unalias,
        "unset" => unset,
        _ => return None,
    })
}

pub fn names() -> impl Iterator<Item = &'static str> {
    BUILTINS.iter().map(|(name, _)| *name)
}

pub fn describe(name: &str) -> Option<&'static str> {
    BUILTINS
        .iter()
        .find(|(candidate, _)| *candidate == name)
        .map(|(_, description)| *description)
}

fn cd(shell: &mut Shell, args: &[String], io: &mut BuiltinIo<'_>) -> i32 {
    let target = match args.first().map(String::as_str) {
        None => util::home_dir().unwrap_or_else(|| shell.cwd.clone()),
        Some("-") => match shell.previous_dir.clone() {
            Some(dir) => dir,
            None => {
                io.err("cd: 没有上一个目录");
                return 1;
            }
        },
        Some(path) => shell.resolve_path(path),
    };
    match shell.set_cwd(target.clone()) {
        Ok(()) => {
            if args.first().map(String::as_str) == Some("-") {
                io.out(&shell.cwd.display().to_string());
            }
            0
        }
        Err(error) => {
            io.err(&format!("cd: {}: {error}", target.display()));
            1
        }
    }
}

fn pwd(shell: &mut Shell, _args: &[String], io: &mut BuiltinIo<'_>) -> i32 {
    io.out(&shell.cwd.display().to_string());
    0
}

fn echo(_shell: &mut Shell, args: &[String], io: &mut BuiltinIo<'_>) -> i32 {
    let mut newline = true;
    let mut escapes = false;
    let mut rest = args;
    while let Some(first) = rest.first() {
        match first.as_str() {
            "-n" => {
                newline = false;
                rest = &rest[1..];
            }
            "-e" => {
                escapes = true;
                rest = &rest[1..];
            }
            _ => break,
        }
    }
    let mut text = rest.join(" ");
    if escapes {
        text = text
            .replace("\\n", "\n")
            .replace("\\t", "\t")
            .replace("\\e", "\x1b")
            .replace("\\\\", "\\");
    }
    if newline {
        let _ = writeln!(io.stdout, "{text}");
    } else {
        let _ = write!(io.stdout, "{text}");
    }
    0
}

fn exit(shell: &mut Shell, args: &[String], io: &mut BuiltinIo<'_>) -> i32 {
    let code = match args.first() {
        Some(raw) => match raw.parse::<i32>() {
            Ok(code) => code,
            Err(_) => {
                io.err(&format!("exit: {raw} 不是有效的退出码"));
                return 2;
            }
        },
        None => shell.last_status,
    };
    shell.exit_request = Some(code);
    code
}

fn bool_true(_shell: &mut Shell, _args: &[String], _io: &mut BuiltinIo<'_>) -> i32 {
    0
}

fn bool_false(_shell: &mut Shell, _args: &[String], _io: &mut BuiltinIo<'_>) -> i32 {
    1
}

fn clear(_shell: &mut Shell, _args: &[String], io: &mut BuiltinIo<'_>) -> i32 {
    let _ = write!(io.stdout, "\x1b[2J\x1b[3J\x1b[H");
    0
}

fn export(shell: &mut Shell, args: &[String], io: &mut BuiltinIo<'_>) -> i32 {
    if args.is_empty() {
        let entries: Vec<String> = shell
            .env
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect();
        for entry in entries {
            io.out(&entry);
        }
        return 0;
    }
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if let Some((name, value)) = arg.split_once('=') {
            util::env_set(&mut shell.env, name, value.to_string());
            index += 1;
        } else if index + 1 < args.len() {
            util::env_set(&mut shell.env, arg, args[index + 1].clone());
            index += 2;
        } else {
            if util::env_get(&shell.env, arg).is_none() {
                util::env_set(&mut shell.env, arg, String::new());
            }
            index += 1;
        }
    }
    0
}

fn unset(shell: &mut Shell, args: &[String], io: &mut BuiltinIo<'_>) -> i32 {
    if args.is_empty() {
        io.err("unset: 需要变量名");
        return 2;
    }
    for name in args {
        util::env_remove(&mut shell.env, name);
    }
    0
}

fn alias(shell: &mut Shell, args: &[String], io: &mut BuiltinIo<'_>) -> i32 {
    define_mapping(args, io, "alias", &mut shell.aliases)
}

fn unalias(shell: &mut Shell, args: &[String], io: &mut BuiltinIo<'_>) -> i32 {
    remove_mapping(args, io, "unalias", &mut shell.aliases)
}

fn abbr(shell: &mut Shell, args: &[String], io: &mut BuiltinIo<'_>) -> i32 {
    define_mapping(args, io, "abbr", &mut shell.abbreviations)
}

fn unabbr(shell: &mut Shell, args: &[String], io: &mut BuiltinIo<'_>) -> i32 {
    remove_mapping(args, io, "unabbr", &mut shell.abbreviations)
}

fn define_mapping(
    args: &[String],
    io: &mut BuiltinIo<'_>,
    keyword: &str,
    table: &mut std::collections::BTreeMap<String, String>,
) -> i32 {
    if args.is_empty() {
        let entries: Vec<String> = table
            .iter()
            .map(|(key, value)| format!("{keyword} {key}='{value}'"))
            .collect();
        for entry in entries {
            io.out(&entry);
        }
        return 0;
    }
    if args.len() == 1 {
        if let Some((name, value)) = args[0].split_once('=') {
            table.insert(name.trim().to_string(), value.trim().to_string());
            return 0;
        }
        return match table.get(&args[0]) {
            Some(value) => {
                io.out(&format!("{keyword} {}='{value}'", args[0]));
                0
            }
            None => {
                io.err(&format!("{keyword}: {} 未定义", args[0]));
                1
            }
        };
    }
    let name = args[0].trim_end_matches('=').to_string();
    let value = args[1..]
        .join(" ")
        .trim_start_matches('=')
        .trim()
        .to_string();
    if name.is_empty() || value.is_empty() {
        io.err(&format!("{keyword}: 用法 {keyword} 名称=\"命令\""));
        return 2;
    }
    table.insert(name, value);
    0
}

fn remove_mapping(
    args: &[String],
    io: &mut BuiltinIo<'_>,
    keyword: &str,
    table: &mut std::collections::BTreeMap<String, String>,
) -> i32 {
    if args.is_empty() {
        io.err(&format!("{keyword}: 需要名称"));
        return 2;
    }
    let mut status = 0;
    for name in args {
        if table.remove(name).is_none() {
            io.err(&format!("{keyword}: {name} 未定义"));
            status = 1;
        }
    }
    status
}

fn history(shell: &mut Shell, args: &[String], io: &mut BuiltinIo<'_>) -> i32 {
    match args.first().map(String::as_str) {
        Some("-c" | "--clear") => {
            shell.history.clear();
            io.out("历史已清空");
            0
        }
        Some("-s" | "search" | "--search") => {
            let term = args[1..].join(" ");
            let matches: Vec<String> = shell
                .history
                .ranked_matches(&term, 30)
                .into_iter()
                .map(str::to_string)
                .collect();
            if matches.is_empty() {
                io.err(&format!("history: 没有匹配 `{term}` 的记录"));
                return 1;
            }
            for command in matches {
                io.out(&command);
            }
            0
        }
        other => {
            let limit = other.and_then(|raw| raw.parse::<usize>().ok());
            if other.is_some() && limit.is_none() {
                io.err("history: 用法 history [数量] | history search <关键字> | history -c");
                return 2;
            }
            let entries = shell.history.entries();
            let total = entries.len();
            let start = match limit {
                Some(count) => total.saturating_sub(count),
                None => 0,
            };
            let lines: Vec<String> = entries[start..]
                .iter()
                .enumerate()
                .map(|(offset, entry)| format!("{:>5}  {}", start + offset + 1, entry.command))
                .collect();
            for line in lines {
                io.out(&line);
            }
            0
        }
    }
}

fn jobs(shell: &mut Shell, _args: &[String], io: &mut BuiltinIo<'_>) -> i32 {
    shell.reap_jobs();
    if shell.jobs.is_empty() {
        io.out("没有后台任务");
        return 0;
    }
    let lines: Vec<String> = shell
        .jobs
        .iter()
        .map(|job| format!("[{}] {} {}", job.id, job.pid, job.command))
        .collect();
    for line in lines {
        io.out(&line);
    }
    0
}

fn source(shell: &mut Shell, args: &[String], io: &mut BuiltinIo<'_>) -> i32 {
    let Some(path) = args.first() else {
        io.err("source: 需要文件路径");
        return 2;
    };
    let resolved = shell.resolve_path(path);
    match std::fs::read_to_string(&resolved) {
        Ok(text) => exec::run_script(shell, &text),
        Err(error) => {
            io.err(&format!("source: {}: {error}", resolved.display()));
            1
        }
    }
}

fn type_of(shell: &mut Shell, args: &[String], io: &mut BuiltinIo<'_>) -> i32 {
    if args.is_empty() {
        io.err("type: 需要命令名");
        return 2;
    }
    let mut status = 0;
    for name in args {
        if let Some(description) = describe(name) {
            io.out(&format!("{name} 是内建命令：{description}"));
        } else if let Some(value) = shell.aliases.get(name) {
            io.out(&format!("{name} 是别名：{value}"));
        } else if let Some(value) = shell.abbreviations.get(name) {
            io.out(&format!("{name} 是缩写：{value}"));
        } else if let Some(path) = util::lookup_command(name, &shell.env, &shell.cwd) {
            io.out(&format!("{name} 位于 {}", path.display()));
        } else if shell.resolve_path(name).is_file() {
            io.out(&format!(
                "{name} 是文件，不是命令（{}）",
                shell.resolve_path(name).display()
            ));
        } else {
            io.err(&format!("type: {name}: 未找到"));
            status = 1;
        }
    }
    status
}

fn config(shell: &mut Shell, args: &[String], io: &mut BuiltinIo<'_>) -> i32 {
    match args.first().map(String::as_str) {
        None | Some("path") => {
            let path = Config::path();
            io.out(&format!("配置文件：{}", path.display()));
            match &shell.config.loaded_from {
                Some(loaded) => io.out(&format!("已加载：{}", loaded.display())),
                None => io.out("尚未创建配置文件（使用默认值，可运行 `config init` 生成模板）"),
            }
            io.out(&format!(
                "历史文件：{}",
                shell.config.history_path().display()
            ));
            0
        }
        Some("init") => {
            let force = args.iter().any(|a| a == "--force" || a == "-f");
            let path = Config::path();
            if path.exists() && !force {
                io.err(&format!(
                    "config: {} 已存在，加 --force 覆盖",
                    path.display()
                ));
                return 1;
            }
            if let Some(parent) = path.parent() {
                if let Err(error) = std::fs::create_dir_all(parent) {
                    io.err(&format!("config: 无法创建 {}: {error}", parent.display()));
                    return 1;
                }
            }
            match std::fs::write(&path, DEFAULT_TEMPLATE) {
                Ok(()) => {
                    io.out(&format!("已写入 {}", path.display()));
                    0
                }
                Err(error) => {
                    io.err(&format!("config: 写入失败：{error}"));
                    1
                }
            }
        }
        Some("reload") => {
            let warnings = shell.reload_config();
            for warning in &warnings {
                io.err(warning);
            }
            io.out("配置已重新加载");
            i32::from(!warnings.is_empty())
        }
        Some("show") => {
            let config = shell.config.clone();
            io.out(&format!("format          = {:?}", config.format));
            io.out(&format!("right_format    = {:?}", config.right_format));
            io.out(&format!("add_newline     = {}", config.add_newline));
            io.out(&format!(
                "autosuggest     = {} (来源 {})",
                config.autosuggest.enabled,
                config.autosuggest.sources.join(", ")
            ));
            io.out(&format!(
                "menu            = {} (最多 {} 行)",
                config.menu.enabled, config.menu.max_rows
            ));
            io.out(&format!(
                "history         = {} 条（上限 {}，去重 {}）",
                shell.history.len(),
                config.history.max_entries,
                config.history.dedup
            ));
            io.out(&format!("aliases         = {}", shell.aliases.len()));
            io.out(&format!("abbreviations   = {}", shell.abbreviations.len()));
            0
        }
        Some(other) => {
            io.err(&format!(
                "config: 未知子命令 `{other}`（可用：path / init / reload / show）"
            ));
            2
        }
    }
}

fn help(shell: &mut Shell, _args: &[String], io: &mut BuiltinIo<'_>) -> i32 {
    let version = env!("CARGO_PKG_VERSION");
    io.out(&format!(
        "Commands (cmds) v{version} — 轻量高效的交互式终端\n"
    ));
    io.out("输入体验");
    io.out("  打字时右侧灰色文本      历史自动建议：→ / End / Ctrl-F 采纳整条，Alt-→ 采纳一个词");
    io.out("  Tab                     打开候选菜单（历史 / 命令 / 路径 / 变量），再按 Tab 下一项");
    io.out("  ↑ ↓                     菜单内上下选择；菜单关闭时按当前前缀浏览历史");
    io.out("  Enter                   菜单打开时采纳候选，否则执行命令");
    io.out("  Esc                     关闭菜单 / 放弃建议");
    io.out("  Ctrl-R                  历史模糊搜索（结果同样用 ↑↓ 选择）");
    io.out("  Ctrl-A / Ctrl-E         行首 / 行尾        Alt-← / Alt-→  按词移动");
    io.out("  Ctrl-W / Ctrl-U / Ctrl-K 删词 / 删到行首 / 删到行尾");
    io.out("  Ctrl-L                  清屏              Ctrl-C 放弃当前输入");
    io.out("  Ctrl-D                  空行时退出        空格 展开缩写（abbr）");
    io.out("");
    io.out("语法");
    io.out("  管道 |   逻辑 && ||   顺序 ;   后台 &");
    io.out("  重定向 > >> < 2> 2>> &>");
    io.out("  变量 $VAR ${VAR} $?   通配 * ? [abc] **   引号 '...' \"...\"");
    io.out("");
    io.out("内建命令");
    for (name, description) in BUILTINS {
        io.out(&format!("  {name:<10}{description}"));
    }
    io.out("");
    io.out(&format!(
        "配置：{}（`config init` 生成模板，`config reload` 重载）",
        Config::path().display()
    ));
    io.out(&format!("历史：{}", shell.config.history_path().display()));
    io.out("文档：https://junhey.github.io/commands/#guide");
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn run(shell: &mut Shell, name: &str, args: &[&str]) -> (i32, String, String) {
        let builtin = lookup(name).expect("内建不存在");
        let owned: Vec<String> = args.iter().map(|a| a.to_string()).collect();
        let mut out: Vec<u8> = Vec::new();
        let mut err: Vec<u8> = Vec::new();
        let status = {
            let mut io = BuiltinIo {
                stdout: &mut out,
                stderr: &mut err,
            };
            builtin(shell, &owned, &mut io)
        };
        (
            status,
            String::from_utf8_lossy(&out).into_owned(),
            String::from_utf8_lossy(&err).into_owned(),
        )
    }

    #[test]
    fn echo_supports_no_newline() {
        let mut shell = Shell::new(Config::default(), false);
        let (status, out, _) = run(&mut shell, "echo", &["-n", "hi"]);
        assert_eq!(status, 0);
        assert_eq!(out, "hi");
    }

    #[test]
    fn alias_defines_and_lists() {
        let mut shell = Shell::new(Config::default(), false);
        let (status, _, _) = run(&mut shell, "alias", &["ll=ls -la"]);
        assert_eq!(status, 0);
        assert_eq!(shell.aliases.get("ll").map(String::as_str), Some("ls -la"));
        let (_, out, _) = run(&mut shell, "alias", &[]);
        assert!(out.contains("alias ll='ls -la'"));
    }

    #[test]
    fn export_sets_and_unset_removes() {
        let mut shell = Shell::new(Config::default(), false);
        run(&mut shell, "export", &["CMDS_TEST=1"]);
        assert_eq!(shell.lookup_var("CMDS_TEST").as_deref(), Some("1"));
        run(&mut shell, "unset", &["CMDS_TEST"]);
        assert_eq!(shell.lookup_var("CMDS_TEST"), None);
    }

    #[test]
    fn cd_tracks_previous_directory() {
        let mut shell = Shell::new(Config::default(), false);
        let start = shell.cwd.clone();
        let temp = std::env::temp_dir();
        let (status, _, err) = run(&mut shell, "cd", &[&temp.to_string_lossy()]);
        assert_eq!(status, 0, "{err}");
        let (status, _, _) = run(&mut shell, "cd", &["-"]);
        assert_eq!(status, 0);
        assert_eq!(shell.cwd, start);
    }

    #[test]
    fn type_reports_builtin() {
        let mut shell = Shell::new(Config::default(), false);
        let (status, out, _) = run(&mut shell, "type", &["cd"]);
        assert_eq!(status, 0);
        assert!(out.contains("内建命令"));
    }

    #[test]
    fn exit_requests_shutdown() {
        let mut shell = Shell::new(Config::default(), false);
        let (status, _, _) = run(&mut shell, "exit", &["3"]);
        assert_eq!(status, 3);
        assert_eq!(shell.exit_request, Some(3));
    }

    #[test]
    fn help_lists_keybindings() {
        let mut shell = Shell::new(Config::default(), false);
        let (status, out, _) = run(&mut shell, "help", &[]);
        assert_eq!(status, 0);
        assert!(out.contains("Tab"));
        assert!(out.contains("Ctrl-R"));
    }
}

//! cmds —— Commands：轻量高效的交互式终端。

// i18n 必须第一个声明：`#[macro_use]` 导出的 t! / tf! 只对**之后**声明的模块可见，
// 放在中间的话前面的模块会报 "cannot find macro"。
#[macro_use]
mod i18n;

mod builtins;
mod config;
mod editor;
mod exec;
mod glob;
mod history;
mod integration;
mod json;
mod parser;
mod prompt;
mod shell;
mod style;
mod util;

use config::Config;
use shell::Shell;
use std::io::{IsTerminal, Write};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(dispatch(&args));
}

fn dispatch(args: &[String]) -> i32 {
    match args.first().map(String::as_str) {
        None | Some("-i") => interactive(),
        Some("-c") => {
            if args.len() < 2 {
                eprintln!(
                    "{}",
                    t!("cmds: -c needs a command", "cmds: -c 需要一个命令")
                );
                return 2;
            }
            command_mode(&args[1..].join(" "))
        }
        Some("prompt") => prompt_mode(&args[1..]),
        Some("init") => init_mode(&args[1..]),
        Some("config") => config_mode(&args[1..]),
        Some("--version" | "-V") => {
            println!("cmds {VERSION}");
            0
        }
        Some("--help" | "-h") => {
            print_usage();
            0
        }
        Some(path) if !path.starts_with('-') => script_mode(path, &args[1..]),
        Some(other) => {
            eprintln!(
                "{}",
                tf!(
                    "cmds: unrecognized argument `{other}`",
                    "cmds: 无法识别的参数 `{other}`"
                )
            );
            print_usage();
            2
        }
    }
}

fn enable_color_for_stdout() {
    let forced = std::env::var_os("CLICOLOR_FORCE").is_some();
    let disabled = std::env::var_os("NO_COLOR").is_some();
    style::set_color_enabled(!disabled && (forced || std::io::stdout().is_terminal()));
}

fn load_config() -> Config {
    let (config, warnings) = Config::load();
    for warning in &warnings {
        eprintln!("cmds: {warning}");
    }
    config
}

fn interactive() -> i32 {
    enable_color_for_stdout();
    let mut shell = Shell::new(load_config(), true);
    if !std::io::stdin().is_terminal() {
        // 脚本通过管道喂进来：按脚本执行
        return shell::run_stdin(&mut shell);
    }
    shell::run_interactive(&mut shell)
}

fn command_mode(line: &str) -> i32 {
    enable_color_for_stdout();
    let mut shell = Shell::new(load_config(), false);
    shell::run_command(&mut shell, line)
}

fn script_mode(path: &str, args: &[String]) -> i32 {
    enable_color_for_stdout();
    let mut shell = Shell::new(load_config(), false);
    util::env_set(&mut shell.env, "0", path.to_string());
    for (index, arg) in args.iter().enumerate() {
        util::env_set(&mut shell.env, &(index + 1).to_string(), arg.clone());
    }
    match std::fs::read_to_string(path) {
        Ok(text) => {
            let status = exec::run_script(&mut shell, &text);
            shell.exit_request.unwrap_or(status)
        }
        Err(error) => {
            eprintln!("cmds: {path}: {error}");
            127
        }
    }
}

fn prompt_mode(args: &[String]) -> i32 {
    let mut status = 0i32;
    let mut duration_ms = 0u128;
    let mut jobs = 0usize;
    let mut target = String::from("plain");
    let mut index = 0usize;

    while index < args.len() {
        let key = args[index].as_str();
        let value = args.get(index + 1).map(String::as_str);
        match key {
            "--status" | "-s" => {
                status = value.and_then(|v| v.parse().ok()).unwrap_or(0);
                index += 2;
            }
            "--duration" | "-d" => {
                // fish 的 CMD_DURATION 是毫秒，bash 侧可传秒
                duration_ms = value
                    .and_then(|v| v.parse::<f64>().ok())
                    .map(|v| v as u128)
                    .unwrap_or(0);
                index += 2;
            }
            "--jobs" | "-j" => {
                jobs = value.and_then(|v| v.parse().ok()).unwrap_or(0);
                index += 2;
            }
            "--shell" => {
                target = value.unwrap_or("plain").to_string();
                index += 2;
            }
            other => {
                eprintln!(
                    "{}",
                    tf!(
                        "cmds prompt: unrecognized argument `{other}`",
                        "cmds prompt: 无法识别的参数 `{other}`"
                    )
                );
                return 2;
            }
        }
    }

    // 提示符通常通过命令替换取用，默认保留颜色
    style::set_color_enabled(std::env::var_os("NO_COLOR").is_none());
    let config = load_config();
    let env = util::snapshot_env();
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let rendered = prompt::render(&prompt::Context {
        config: &config,
        cwd: &cwd,
        status,
        duration_ms,
        jobs,
        env: &env,
    });
    let text = match target.as_str() {
        "bash" | "zsh" => prompt::wrap_for_readline(&rendered.full()),
        _ => rendered.full(),
    };
    print!("{text}");
    let _ = std::io::stdout().flush();
    0
}

fn init_mode(args: &[String]) -> i32 {
    let Some(target) = args.first() else {
        eprintln!(
            "{}",
            tf!(
                "cmds init: needs a shell ({})",
                "cmds init: 需要指定 shell（{}）",
                integration::SUPPORTED.join(" / ")
            )
        );
        return 2;
    };
    match integration::script(target) {
        Some(script) => {
            print!("{script}");
            0
        }
        None => {
            eprintln!(
                "{}",
                tf!(
                    "cmds init: `{target}` is not supported yet; available: {}",
                    "cmds init: 暂不支持 `{target}`，可用：{}",
                    integration::SUPPORTED.join(" / ")
                )
            );
            2
        }
    }
}

fn config_mode(args: &[String]) -> i32 {
    enable_color_for_stdout();
    let mut shell = Shell::new(load_config(), false);
    let builtin = builtins::lookup("config").expect("config 内建存在");
    let mut stdout = std::io::stdout();
    let mut stderr = std::io::stderr();
    let mut io = builtins::BuiltinIo {
        stdout: &mut stdout,
        stderr: &mut stderr,
    };
    builtin(&mut shell, args, &mut io)
}

fn print_usage() {
    println!(
        "{}",
        tf!(
            r#"cmds {} — a small, fast interactive shell

Usage:
  cmds                          start the interactive shell
  cmds -c "<command>"           run one command and exit
  cmds <script> [args...]       run a script ($1, $2 available inside)
  cmds prompt [options]         print the prompt, for use by other shells
      --status <N>    exit code of the previous command
      --duration <MS> how long the previous command took
      --jobs <N>      number of background jobs
      --shell <bash|zsh|fish|plain>
  cmds init <bash|zsh|fish|powershell>
                                print the integration snippet, e.g. eval "$(cmds init bash)"
  cmds config [path|init|reload|show]
                                show or create the config file
  cmds --version                print the version
  cmds --help                   print this help

Type `help` inside the shell for every keybinding.
Docs: https://junhey.github.io/commands/#guide"#,
            r#"cmds {} — 轻量高效的交互式终端

用法：
  cmds                          启动交互式 shell
  cmds -c "<命令>"               执行一条命令后退出
  cmds <脚本> [参数...]          执行脚本（脚本内可用 $1、$2）
  cmds prompt [选项]            输出提示符，供其它 shell 使用
      --status <N>    上一条命令退出码
      --duration <MS> 上一条命令耗时（毫秒）
      --jobs <N>      后台任务数量
      --shell <bash|zsh|fish|plain>
  cmds init <bash|zsh|fish|powershell>
                                输出集成脚本，例如：eval "$(cmds init bash)"
  cmds config [path|init|reload|show]
                                查看/生成配置文件
  cmds --version                查看版本
  cmds --help                   查看本帮助

交互式下输入 `help` 可查看全部快捷键。
文档：https://junhey.github.io/commands/#guide"#,
            VERSION
        )
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_and_help_succeed() {
        assert_eq!(dispatch(&["--version".to_string()]), 0);
        assert_eq!(dispatch(&["--help".to_string()]), 0);
    }

    #[test]
    fn rejects_unknown_flags() {
        assert_eq!(dispatch(&["--nope".to_string()]), 2);
        assert_eq!(dispatch(&["-c".to_string()]), 2);
    }

    #[test]
    fn runs_command_mode() {
        assert_eq!(dispatch(&["-c".to_string(), "true".to_string()]), 0);
        assert_eq!(dispatch(&["-c".to_string(), "false".to_string()]), 1);
    }

    #[test]
    fn prints_prompt_without_panicking() {
        assert_eq!(
            dispatch(&[
                "prompt".to_string(),
                "--status".to_string(),
                "1".to_string()
            ]),
            0
        );
    }

    #[test]
    fn init_requires_known_shell() {
        assert_eq!(dispatch(&["init".to_string(), "bash".to_string()]), 0);
        assert_eq!(dispatch(&["init".to_string(), "tcsh".to_string()]), 2);
        assert_eq!(dispatch(&["init".to_string()]), 2);
    }
}

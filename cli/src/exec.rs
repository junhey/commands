//! 执行引擎：别名/通配展开、管道、重定向、逻辑连接、后台任务。

use crate::builtins::{self, BuiltinIo};
use crate::glob;
use crate::parser::{self, ChainOp, Command as CommandNode, Fd, Node, Pipeline, RedirectKind};
use crate::shell::Shell;
use crate::util;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::process::{Child, Command, Stdio};

#[derive(Debug, Default, Clone)]
struct Redirs {
    stdin: Option<String>,
    stdout: Option<(String, bool)>,
    stderr: Option<(String, bool)>,
    merge_stderr: bool,
}

#[derive(Debug, Clone)]
struct Resolved {
    argv: Vec<String>,
    redirs: Redirs,
}

enum Input {
    Inherit,
    Bytes(Vec<u8>),
    Pipe(std::process::ChildStdout),
    File(File),
}

enum Stage {
    Child(Child),
    Done(i32),
    Bytes(Vec<u8>, i32),
}

/// 执行一行命令，返回退出码。
pub fn run_line(shell: &mut Shell, line: &str) -> i32 {
    let parsed = {
        let vars = |name: &str| shell.lookup_var(name);
        parser::parse_line(line, &vars)
    };
    let status = match parsed {
        Ok(Some(node)) => run_node(shell, &node),
        Ok(None) => shell.last_status,
        Err(error) => {
            eprintln!(
                "{}",
                tf!("cmds: syntax error: {error}", "cmds: 语法错误：{error}")
            );
            2
        }
    };
    shell.last_status = status;
    status
}

/// 执行脚本内容（支持续行），用于 `source` 与 `~/.cmdsrc`。
pub fn run_script(shell: &mut Shell, text: &str) -> i32 {
    let mut status = 0;
    let mut buffer = String::new();
    // 某些编辑器/管道会带 UTF-8 BOM
    let text = text.trim_start_matches('\u{feff}');
    for raw in text.lines() {
        if buffer.is_empty() && raw.trim().is_empty() {
            continue;
        }
        if !buffer.is_empty() {
            buffer.push('\n');
        }
        buffer.push_str(raw);
        if parser::needs_continuation(&buffer) {
            continue;
        }
        status = run_line(shell, &buffer);
        buffer.clear();
        if shell.exit_request.is_some() {
            break;
        }
    }
    if !buffer.trim().is_empty() && shell.exit_request.is_none() {
        status = run_line(shell, &buffer);
    }
    status
}

pub fn run_node(shell: &mut Shell, node: &Node) -> i32 {
    match node {
        Node::Pipeline(pipeline) => run_pipeline(shell, pipeline),
        Node::Chain { left, op, right } => {
            let status = run_node(shell, left);
            if shell.exit_request.is_some() {
                return status;
            }
            match op {
                ChainOp::And if status != 0 => status,
                ChainOp::Or if status == 0 => status,
                _ => run_node(shell, right),
            }
        }
    }
}

fn run_pipeline(shell: &mut Shell, pipeline: &Pipeline) -> i32 {
    let mut stages: Vec<Resolved> = Vec::new();
    for command in &pipeline.commands {
        match resolve(shell, command) {
            Ok(Some(resolved)) => stages.push(resolved),
            Ok(None) => {}
            Err(message) => {
                eprintln!("cmds: {message}");
                return 1;
            }
        }
    }
    if stages.is_empty() {
        return 0;
    }

    let total = stages.len();
    let mut input = Input::Inherit;
    let mut children: Vec<Child> = Vec::new();
    let mut status = 0;

    for (index, stage) in stages.iter().enumerate() {
        let is_last = index + 1 == total;
        let stdin = match &stage.redirs.stdin {
            Some(path) => {
                let resolved = shell.resolve_path(path);
                match File::open(&resolved) {
                    Ok(file) => Input::File(file),
                    Err(error) => {
                        eprintln!("cmds: {}: {error}", resolved.display());
                        return 1;
                    }
                }
            }
            None => std::mem::replace(&mut input, Input::Inherit),
        };
        match exec_stage(shell, stage, stdin, !is_last, pipeline.background) {
            Ok(Stage::Child(mut child)) => {
                if !is_last {
                    if let Some(stdout) = child.stdout.take() {
                        input = Input::Pipe(stdout);
                    }
                }
                children.push(child);
            }
            Ok(Stage::Done(code)) => {
                status = code;
                input = Input::Inherit;
            }
            Ok(Stage::Bytes(bytes, code)) => {
                status = code;
                input = Input::Bytes(bytes);
            }
            Err(message) => {
                eprintln!("cmds: {message}");
                return 127;
            }
        }
    }

    let label = stages
        .iter()
        .map(|s| s.argv.join(" "))
        .collect::<Vec<_>>()
        .join(" | ");

    if pipeline.background {
        for child in children {
            shell.register_job(child, &label);
        }
        return 0;
    }

    for mut child in children {
        match child.wait() {
            Ok(exit) => status = exit_code(&exit),
            Err(error) => {
                eprintln!(
                    "{}",
                    tf!(
                        "cmds: failed to wait for the child process: {error}",
                        "cmds: 等待子进程失败：{error}"
                    )
                );
                status = 1;
            }
        }
    }
    status
}

fn exit_code(status: &std::process::ExitStatus) -> i32 {
    if let Some(code) = status.code() {
        return code;
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            return 128 + signal;
        }
    }
    1
}

/// 别名展开 + 通配展开 + 重定向收集。
fn resolve(shell: &mut Shell, node: &CommandNode) -> Result<Option<Resolved>, String> {
    let mut argv: Vec<String> = Vec::new();
    for word in &node.words {
        if word.globbable && glob::is_pattern(&word.text) {
            let matches = glob::expand(&word.text, &shell.cwd);
            if matches.is_empty() {
                argv.push(word.text.clone());
            } else {
                argv.extend(matches);
            }
        } else {
            argv.push(word.text.clone());
        }
    }

    let mut expanded: Vec<String> = Vec::new();
    while let Some(first) = argv.first().cloned() {
        if expanded.contains(&first) {
            break;
        }
        let Some(definition) = shell.aliases.get(&first).cloned() else {
            break;
        };
        expanded.push(first);
        let tokens = {
            let vars = |name: &str| shell.lookup_var(name);
            parser::tokenize(&definition, &vars).map_err(|error| error.message)?
        };
        let mut replacement: Vec<String> = tokens
            .iter()
            .filter_map(|token| match token {
                parser::Token::Word(word) => Some(word.text.clone()),
                _ => None,
            })
            .collect();
        if replacement.is_empty() {
            break;
        }
        replacement.extend(argv.into_iter().skip(1));
        argv = replacement;
    }

    let mut redirs = Redirs::default();
    for redirect in &node.redirects {
        match redirect.kind {
            RedirectKind::In => redirs.stdin = Some(redirect.target.clone()),
            RedirectKind::Out { fd, append } => match fd {
                Fd::Out => redirs.stdout = Some((redirect.target.clone(), append)),
                Fd::Err => redirs.stderr = Some((redirect.target.clone(), append)),
                Fd::Both => {
                    redirs.stdout = Some((redirect.target.clone(), append));
                    redirs.merge_stderr = true;
                }
            },
        }
    }

    if argv.is_empty() {
        return Ok(None);
    }
    Ok(Some(Resolved { argv, redirs }))
}

fn exec_stage(
    shell: &mut Shell,
    stage: &Resolved,
    stdin: Input,
    pipe_out: bool,
    background: bool,
) -> Result<Stage, String> {
    if let Some(builtin) = builtins::lookup(&stage.argv[0]) {
        drop(stdin);
        let args: Vec<String> = stage.argv[1..].to_vec();
        let mut buffer: Vec<u8> = Vec::new();
        let status;
        {
            let mut out: Box<dyn Write> = match &stage.redirs.stdout {
                Some((path, append)) => Box::new(open_out(shell, path, *append)?),
                None if pipe_out => Box::new(&mut buffer),
                None => Box::new(std::io::stdout()),
            };
            let mut err: Box<dyn Write> = match &stage.redirs.stderr {
                Some((path, append)) => Box::new(open_out(shell, path, *append)?),
                None => Box::new(std::io::stderr()),
            };
            let mut io = BuiltinIo {
                stdout: out.as_mut(),
                stderr: err.as_mut(),
            };
            status = builtin(shell, &args, &mut io);
            let _ = io.stdout.flush();
            let _ = io.stderr.flush();
        }
        return Ok(if pipe_out {
            Stage::Bytes(buffer, status)
        } else {
            Stage::Done(status)
        });
    }

    let program = util::lookup_command(&stage.argv[0], &shell.env, &shell.cwd);
    let mut command = match program {
        Some(path) => {
            let mut command = Command::new(path);
            command.args(&stage.argv[1..]);
            command
        }
        None if cfg!(windows) && is_cmd_builtin(&stage.argv[0]) => {
            // Windows 上回落到 cmd 内部命令（dir、copy、where 等）
            let mut command = Command::new("cmd");
            command.arg("/C");
            command.args(&stage.argv);
            command
        }
        None => return Err(command_not_found(shell, &stage.argv[0])),
    };

    command.current_dir(&shell.cwd).env_clear().envs(&shell.env);

    let pending = match stdin {
        Input::Inherit => {
            command.stdin(Stdio::inherit());
            None
        }
        Input::File(file) => {
            command.stdin(Stdio::from(file));
            None
        }
        Input::Pipe(stdout) => {
            command.stdin(Stdio::from(stdout));
            None
        }
        Input::Bytes(bytes) => {
            command.stdin(Stdio::piped());
            Some(bytes)
        }
    };

    match &stage.redirs.stdout {
        Some((path, append)) => {
            let file = open_out(shell, path, *append)?;
            if stage.redirs.merge_stderr {
                let clone = file
                    .try_clone()
                    .map_err(|error| format!("{path}: {error}"))?;
                command.stderr(Stdio::from(clone));
            }
            command.stdout(Stdio::from(file));
        }
        None if pipe_out => {
            command.stdout(Stdio::piped());
        }
        None => {
            command.stdout(Stdio::inherit());
        }
    }

    if let Some((path, append)) = &stage.redirs.stderr {
        let file = open_out(shell, path, *append)?;
        command.stderr(Stdio::from(file));
    }

    if background && pending.is_none() {
        command.stdin(Stdio::null());
    }

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        unsafe {
            command.pre_exec(|| {
                // 子进程恢复默认信号处理，Ctrl-C 只影响前台任务
                libc::signal(libc::SIGINT, libc::SIG_DFL);
                libc::signal(libc::SIGQUIT, libc::SIG_DFL);
                libc::signal(libc::SIGTSTP, libc::SIG_DFL);
                Ok(())
            });
        }
    }

    let mut child = command
        .spawn()
        .map_err(|error| format!("{}: {error}", stage.argv[0]))?;

    if let Some(bytes) = pending {
        if let Some(mut sink) = child.stdin.take() {
            std::thread::spawn(move || {
                let _ = sink.write_all(&bytes);
            });
        }
    }
    Ok(Stage::Child(child))
}

fn open_out(shell: &Shell, path: &str, append: bool) -> Result<File, String> {
    let resolved = shell.resolve_path(path);
    let mut options = OpenOptions::new();
    options.write(true).create(true);
    if append {
        options.append(true);
    } else {
        options.truncate(true);
    }
    options
        .open(&resolved)
        .map_err(|error| format!("{}: {error}", resolved.display()))
}

/// cmd.exe 的内部命令（Windows 上没有对应可执行文件，需要经 `cmd /C` 执行）。
const CMD_BUILTINS: &[&str] = &[
    "assoc", "cls", "copy", "date", "del", "dir", "erase", "ftype", "md", "mkdir", "mklink",
    "move", "path", "rd", "ren", "rename", "rmdir", "start", "time", "title", "tree", "type",
    "ver", "vol",
];

fn is_cmd_builtin(name: &str) -> bool {
    CMD_BUILTINS
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(name))
}

fn command_not_found(shell: &mut Shell, name: &str) -> String {
    let mut message = tf!("{name}: command not found", "{name}：未找到命令");
    let suggestions = shell.similar_commands(name, 3);
    if !suggestions.is_empty() {
        // 分隔符也跟着语言走：英文用逗号，中文用顿号。
        message.push_str(&tf!(
            ". Did you mean: {}",
            "，也许你想输入：{}",
            suggestions.join(t!(", ", "、"))
        ));
    }
    message
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn shell() -> Shell {
        Shell::new(Config::default(), false)
    }

    #[test]
    fn runs_builtin_and_sets_status() {
        let mut sh = shell();
        assert_eq!(run_line(&mut sh, "true"), 0);
        assert_eq!(run_line(&mut sh, "false"), 1);
    }

    #[test]
    fn honours_logical_operators() {
        let mut sh = shell();
        assert_eq!(run_line(&mut sh, "false && true"), 1);
        assert_eq!(run_line(&mut sh, "false || true"), 0);
        assert_eq!(run_line(&mut sh, "true; false"), 1);
    }

    #[test]
    fn tracks_last_status_for_next_line() {
        let mut sh = shell();
        run_line(&mut sh, "false");
        assert_eq!(sh.lookup_var("?").as_deref(), Some("1"));
        run_line(&mut sh, "echo $?");
        run_line(&mut sh, "true");
        assert_eq!(sh.lookup_var("?").as_deref(), Some("0"));
    }

    #[test]
    fn ignores_utf8_bom() {
        let mut sh = shell();
        assert_eq!(run_script(&mut sh, "\u{feff}true"), 0);
        assert_eq!(run_line(&mut sh, "\u{feff}false"), 1);
    }

    #[test]
    fn expands_aliases() {
        let mut sh = shell();
        sh.aliases.insert("t".to_string(), "true".to_string());
        assert_eq!(run_line(&mut sh, "t"), 0);
    }

    #[test]
    fn reports_missing_command() {
        let mut sh = shell();
        assert_eq!(run_line(&mut sh, "definitely-not-a-real-command-xyz"), 127);
    }

    #[test]
    fn redirects_builtin_output_to_file() {
        let mut sh = shell();
        let file = std::env::temp_dir().join(format!("cmds-exec-{}.txt", std::process::id()));
        let line = format!("echo hello > {}", file.display());
        assert_eq!(run_line(&mut sh, &line), 0);
        let content = std::fs::read_to_string(&file).unwrap();
        assert_eq!(content.trim(), "hello");
        let _ = std::fs::remove_file(&file);
    }

    #[test]
    fn runs_script_with_continuation() {
        let mut sh = shell();
        let file = std::env::temp_dir().join(format!("cmds-script-{}.txt", std::process::id()));
        let script = format!("echo one >> {0}\necho \\\n  two >> {0}\n", file.display());
        let _ = std::fs::remove_file(&file);
        run_script(&mut sh, &script);
        let content = std::fs::read_to_string(&file).unwrap();
        assert!(content.contains("one"));
        assert!(content.contains("two"));
        let _ = std::fs::remove_file(&file);
    }
}

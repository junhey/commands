//! Shell 运行时状态与交互式主循环。

use crate::config::Config;
use crate::editor::{self, ReadOutcome};
use crate::exec;
use crate::history::History;
use crate::parser;
use crate::prompt::{self, Prompt};
use crate::style;
use crate::util::{self, EnvMap};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Child;
use std::rc::Rc;
use std::time::Instant;

pub struct Job {
    pub id: u32,
    pub pid: u32,
    pub command: String,
    pub child: Child,
}

pub struct Shell {
    pub config: Rc<Config>,
    pub env: EnvMap,
    pub aliases: BTreeMap<String, String>,
    pub abbreviations: BTreeMap<String, String>,
    pub cwd: PathBuf,
    pub previous_dir: Option<PathBuf>,
    pub last_status: i32,
    pub last_duration_ms: u128,
    pub history: History,
    pub jobs: Vec<Job>,
    pub exit_request: Option<i32>,
    pub interactive: bool,
    next_job_id: u32,
    command_cache: Option<(u64, Vec<String>)>,
}

impl Shell {
    pub fn new(config: Config, interactive: bool) -> Self {
        let aliases: BTreeMap<String, String> = config.aliases.iter().cloned().collect();
        let abbreviations: BTreeMap<String, String> =
            config.abbreviations.iter().cloned().collect();
        let mut env = util::snapshot_env();
        for (key, value) in &config.env {
            util::env_set(&mut env, key, value.clone());
        }
        let cwd = normalize(
            std::env::current_dir().unwrap_or_else(|_| util::home_dir().unwrap_or_default()),
        );
        util::env_set(&mut env, "PWD", cwd.to_string_lossy().to_string());
        util::env_set(&mut env, "CMDS_VERSION", env!("CARGO_PKG_VERSION"));
        if let Ok(exe) = std::env::current_exe() {
            util::env_set(&mut env, "SHELL", exe.to_string_lossy().to_string());
        }
        let history = if interactive {
            History::load(&config.history, config.history_path())
        } else {
            History::ephemeral(&config.history)
        };
        Self {
            config: Rc::new(config),
            env,
            aliases,
            abbreviations,
            cwd,
            previous_dir: None,
            last_status: 0,
            last_duration_ms: 0,
            history,
            jobs: Vec::new(),
            exit_request: None,
            interactive,
            next_job_id: 1,
            command_cache: None,
        }
    }

    /// 变量查询（供词法分析展开 `$VAR`）。
    pub fn lookup_var(&self, name: &str) -> Option<String> {
        match name {
            "?" => Some(self.last_status.to_string()),
            "$" => Some(std::process::id().to_string()),
            "PWD" => Some(self.cwd.to_string_lossy().into_owned()),
            _ => util::env_get(&self.env, name).cloned(),
        }
    }

    /// 相对当前目录解析路径，并展开 `~`。
    pub fn resolve_path(&self, raw: &str) -> PathBuf {
        let expanded = util::expand_tilde(raw);
        let path = PathBuf::from(expanded);
        if path.is_absolute() {
            path
        } else {
            self.cwd.join(path)
        }
    }

    pub fn set_cwd(&mut self, path: PathBuf) -> std::io::Result<()> {
        let canonical = normalize(std::fs::canonicalize(&path)?);
        if !canonical.is_dir() {
            return Err(std::io::Error::other(t!("not a directory", "不是目录")));
        }
        std::env::set_current_dir(&canonical)?;
        let previous = std::mem::replace(&mut self.cwd, canonical.clone());
        util::env_set(
            &mut self.env,
            "OLDPWD",
            previous.to_string_lossy().to_string(),
        );
        util::env_set(
            &mut self.env,
            "PWD",
            canonical.to_string_lossy().to_string(),
        );
        self.previous_dir = Some(previous);
        Ok(())
    }

    /// 重新读取配置文件，返回警告列表。
    pub fn reload_config(&mut self) -> Vec<String> {
        let (config, warnings) = Config::load();
        for (key, value) in &config.aliases {
            self.aliases.insert(key.clone(), value.clone());
        }
        for (key, value) in &config.abbreviations {
            self.abbreviations.insert(key.clone(), value.clone());
        }
        for (key, value) in &config.env {
            util::env_set(&mut self.env, key, value.clone());
        }
        self.config = Rc::new(config);
        warnings
    }

    pub fn register_job(&mut self, child: Child, command: &str) {
        let id = self.next_job_id;
        self.next_job_id += 1;
        let pid = child.id();
        if self.interactive {
            println!("[{id}] {pid}");
        }
        self.jobs.push(Job {
            id,
            pid,
            command: command.to_string(),
            child,
        });
    }

    /// 回收已结束的后台任务。
    pub fn reap_jobs(&mut self) {
        let mut finished: Vec<(u32, String, i32)> = Vec::new();
        for job in &mut self.jobs {
            if let Ok(Some(status)) = job.child.try_wait() {
                finished.push((job.id, job.command.clone(), status.code().unwrap_or(0)));
            }
        }
        if finished.is_empty() {
            return;
        }
        self.jobs
            .retain(|job| !finished.iter().any(|(id, _, _)| *id == job.id));
        if self.interactive {
            for (id, command, code) in finished {
                println!(
                    "{}",
                    tf!(
                        "[{id}] done (exit {code}) {command}",
                        "[{id}] 结束（退出码 {code}）{command}"
                    )
                );
            }
        }
    }

    /// PATH 中的可执行文件名，30 秒缓存。
    pub fn path_commands(&mut self) -> &[String] {
        let now = util::now_secs();
        let stale = match &self.command_cache {
            Some((stamp, _)) => now.saturating_sub(*stamp) > 30,
            None => true,
        };
        if stale {
            self.command_cache = Some((now, util::list_path_executables(&self.env)));
        }
        self.command_cache
            .as_ref()
            .map(|(_, list)| list.as_slice())
            .unwrap_or(&[])
    }

    /// 命令是否存在（高亮与补全用）。
    pub fn is_known_command(&mut self, name: &str) -> bool {
        if name.is_empty() {
            return false;
        }
        if crate::builtins::lookup(name).is_some()
            || self.aliases.contains_key(name)
            || self.abbreviations.contains_key(name)
        {
            return true;
        }
        if name.contains('/') || (cfg!(windows) && name.contains('\\')) {
            return util::lookup_command(name, &self.env, &self.cwd).is_some();
        }
        let found = {
            let commands = self.path_commands();
            if cfg!(windows) {
                commands.iter().any(|c| c.eq_ignore_ascii_case(name))
            } else {
                commands.iter().any(|c| c == name)
            }
        };
        found || util::lookup_command(name, &self.env, &self.cwd).is_some()
    }

    /// 拼错命令时给出相近建议。
    ///
    /// 排序依次看：编辑距离 → 候选类型（内建 / 别名缩写 / PATH）→ 首字母是否相同 → 长度差 →
    /// 字典序。只按「距离 + 字典序」排会让 `hepl` 被 PATH 里的 `h2ph`、`head`、`heap`
    /// 顶掉真正想要的内建 `help`，所以内建与别名必须优先，且优先保留首字母一致的候选。
    pub fn similar_commands(&mut self, name: &str, limit: usize) -> Vec<String> {
        const KIND_BUILTIN: u8 = 0;
        const KIND_USER: u8 = 1;
        const KIND_PATH: u8 = 2;

        let lowered = name.to_lowercase();
        let typed_len = lowered.chars().count();
        // 输入太短时放宽一格容易全是噪声
        let threshold = if typed_len <= 3 { 1 } else { 2 };
        let first = lowered.chars().next();

        let mut pool: Vec<(String, u8)> = crate::builtins::names()
            .map(|builtin| (builtin.to_string(), KIND_BUILTIN))
            .collect();
        pool.extend(self.aliases.keys().map(|key| (key.clone(), KIND_USER)));
        pool.extend(
            self.abbreviations
                .keys()
                .map(|key| (key.clone(), KIND_USER)),
        );
        pool.extend(
            self.path_commands()
                .iter()
                .map(|command| (command.clone(), KIND_PATH)),
        );

        let mut scored: Vec<(usize, u8, bool, usize, String)> = pool
            .into_iter()
            .filter_map(|(candidate, kind)| {
                let folded = candidate.to_lowercase();
                if folded == lowered {
                    return None;
                }
                let distance = util::levenshtein(&lowered, &folded);
                if distance > threshold {
                    return None;
                }
                let differing_first = first.is_some() && folded.chars().next() != first;
                let length_gap = typed_len.abs_diff(folded.chars().count());
                Some((distance, kind, differing_first, length_gap, candidate))
            })
            .collect();
        scored.sort();
        scored.dedup_by(|a, b| a.4 == b.4);
        scored.truncate(limit);
        scored
            .into_iter()
            .map(|(.., candidate)| candidate)
            .collect()
    }

    pub fn render_prompt(&self) -> Prompt {
        let config = Rc::clone(&self.config);
        let mut rendered = prompt::render(&prompt::Context {
            config: config.as_ref(),
            cwd: &self.cwd,
            status: self.last_status,
            duration_ms: self.last_duration_ms,
            jobs: self.jobs.len(),
            env: &self.env,
        });
        let width = crossterm::terminal::size()
            .map(|(columns, _)| columns as usize)
            .unwrap_or(80);
        prompt::attach_right(&mut rendered, width);
        rendered
    }

    /// 多行输入时的续行提示符。
    pub fn continuation_prompt(&self) -> Prompt {
        Prompt {
            leading: String::new(),
            last_line: style::paint(
                &self.config.continuation_style,
                &self.config.continuation_symbol,
            ),
            right: String::new(),
        }
    }
}

/// 去掉 Windows 的 `\\?\` 前缀，保证展示与拼接正常。
fn normalize(path: PathBuf) -> PathBuf {
    let text = path.to_string_lossy();
    match text.strip_prefix(r"\\?\") {
        Some(rest) => PathBuf::from(rest),
        None => path,
    }
}

fn rc_candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Ok(custom) = std::env::var("CMDS_RC") {
        paths.push(PathBuf::from(util::expand_tilde(&custom)));
    }
    if let Some(home) = util::home_dir() {
        paths.push(home.join(".cmdsrc"));
    }
    paths.push(util::config_dir().join("init.cmds"));
    paths
}

/// 执行启动脚本。
pub fn load_rc(shell: &mut Shell) {
    for path in rc_candidates() {
        if !path.is_file() {
            continue;
        }
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                exec::run_script(shell, &text);
            }
            Err(error) => eprintln!(
                "{}",
                tf!(
                    "cmds: cannot read {}: {error}",
                    "cmds: 无法读取 {}: {error}",
                    path.display()
                )
            ),
        }
        break;
    }
}

/// 交互式主循环。
pub fn run_interactive(shell: &mut Shell) -> i32 {
    #[cfg(unix)]
    unsafe {
        // shell 自身忽略这些信号，子进程会在 pre_exec 中恢复默认处理
        libc::signal(libc::SIGINT, libc::SIG_IGN);
        libc::signal(libc::SIGQUIT, libc::SIG_IGN);
        libc::signal(libc::SIGTSTP, libc::SIG_IGN);
    }

    load_rc(shell);
    greet(shell);

    loop {
        shell.reap_jobs();
        let mut buffer = String::new();
        let line = loop {
            let prompt = if buffer.is_empty() {
                shell.render_prompt()
            } else {
                shell.continuation_prompt()
            };
            match editor::read_line(shell, &prompt) {
                Ok(ReadOutcome::Line(line)) => {
                    if !buffer.is_empty() {
                        buffer.push('\n');
                    }
                    buffer.push_str(&line);
                    if parser::needs_continuation(&buffer) {
                        continue;
                    }
                    break Some(std::mem::take(&mut buffer));
                }
                Ok(ReadOutcome::Interrupted) => break None,
                Ok(ReadOutcome::Eof) => {
                    println!("exit");
                    return shell.last_status;
                }
                Err(error) => {
                    eprintln!(
                        "{}",
                        tf!(
                            "cmds: failed to read input: {error}",
                            "cmds: 读取输入失败：{error}"
                        )
                    );
                    return 1;
                }
            }
        };

        let Some(line) = line else {
            shell.last_status = 130;
            shell.last_duration_ms = 0;
            continue;
        };
        if line.trim().is_empty() {
            continue;
        }

        shell.history.record(&line);
        let started = Instant::now();
        let status = exec::run_line(shell, &line);
        shell.last_duration_ms = started.elapsed().as_millis();
        shell.last_status = status;

        if let Some(code) = shell.exit_request {
            let _ = shell.history.rewrite();
            return code;
        }
    }
}

fn greet(shell: &Shell) {
    if shell.config.loaded_from.is_some() {
        return;
    }
    let version = env!("CARGO_PKG_VERSION");
    println!(
        "{}",
        style::paint("bold cyan", &format!("Commands v{version}"))
    );
    println!(
        "{}",
        style::paint(
            "dimmed",
            t!(
                "Type help for keybindings · Tab opens candidates · config init writes a template",
                "输入 help 查看快捷键 · Tab 弹出候选 · config init 生成配置模板"
            ),
        )
    );
}

/// 供 `-c` / 脚本模式使用。
pub fn run_command(shell: &mut Shell, line: &str) -> i32 {
    let started = Instant::now();
    let status = exec::run_line(shell, line);
    shell.last_duration_ms = started.elapsed().as_millis();
    shell.last_status = status;
    shell.exit_request.unwrap_or(status)
}

/// 从 stdin 读取脚本执行（`cmds < script`、`cat x | cmds`）。
pub fn run_stdin(shell: &mut Shell) -> i32 {
    use std::io::Read;
    let mut text = String::new();
    if let Err(error) = std::io::stdin().read_to_string(&mut text) {
        eprintln!(
            "{}",
            tf!(
                "cmds: failed to read stdin: {error}",
                "cmds: 读取 stdin 失败：{error}"
            )
        );
        return 1;
    }
    let status = exec::run_script(shell, &text);
    shell.exit_request.unwrap_or(status)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialises_environment() {
        let shell = Shell::new(Config::default(), false);
        assert_eq!(
            shell.lookup_var("CMDS_VERSION").as_deref(),
            Some(env!("CARGO_PKG_VERSION"))
        );
        assert!(shell.lookup_var("PWD").is_some());
        assert_eq!(shell.lookup_var("?").as_deref(), Some("0"));
    }

    #[test]
    fn resolves_relative_paths() {
        let shell = Shell::new(Config::default(), false);
        assert_eq!(
            shell.resolve_path("Cargo.toml"),
            shell.cwd.join("Cargo.toml")
        );
        let home = util::home_dir().unwrap();
        assert_eq!(shell.resolve_path("~"), home);
    }

    #[test]
    fn detects_known_commands() {
        let mut shell = Shell::new(Config::default(), false);
        assert!(shell.is_known_command("cd"));
        assert!(!shell.is_known_command("definitely-not-a-command-xyz"));
    }

    #[test]
    fn suggests_similar_commands() {
        let mut shell = Shell::new(Config::default(), false);
        let suggestions = shell.similar_commands("hepl", 3);
        assert!(suggestions.iter().any(|s| s == "help"));
    }

    #[test]
    fn ranks_builtins_before_path_noise() {
        let mut shell = Shell::new(Config::default(), false);
        // PATH 上通常还有 head / heap / h2ph 这类同距离候选，内建必须排在最前
        assert_eq!(
            shell
                .similar_commands("hepl", 5)
                .first()
                .map(String::as_str),
            Some("help")
        );
    }

    #[test]
    fn never_suggests_the_typed_name_itself() {
        let mut shell = Shell::new(Config::default(), false);
        assert!(
            !shell
                .similar_commands("help", 5)
                .iter()
                .any(|s| s == "help")
        );
    }

    #[test]
    fn prefers_user_aliases_over_path_commands() {
        let config = Config {
            aliases: vec![("gti".to_string(), "git".to_string())],
            ..Config::default()
        };
        let mut shell = Shell::new(config, false);
        assert_eq!(
            shell.similar_commands("gto", 5).first().map(String::as_str),
            Some("gti")
        );
    }
}

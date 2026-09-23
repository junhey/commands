//! 配置加载：`~/.config/cmds/config.toml`（Windows: `%APPDATA%\cmds\config.toml`）。

pub mod toml;

use self::toml::Table;
use crate::util;
use std::path::PathBuf;

/// 默认配置模板。两份只有注释不同，键值必须完全一致——
/// `config_template_matches_across_languages` 测试会把这条钉住。
pub const DEFAULT_TEMPLATE_EN: &str = include_str!("../../assets/config.default.toml");
pub const DEFAULT_TEMPLATE_ZH: &str = include_str!("../../assets/config.default.zh-CN.toml");

/// `cmds config init` 写出的模板，注释跟着界面语言走。
pub fn default_template() -> &'static str {
    crate::i18n::t(DEFAULT_TEMPLATE_EN, DEFAULT_TEMPLATE_ZH)
}

#[derive(Debug, Clone)]
pub struct Config {
    pub format: String,
    pub right_format: String,
    pub add_newline: bool,
    pub continuation_symbol: String,
    pub continuation_style: String,
    pub character: CharacterConfig,
    pub dir: DirConfig,
    pub git: GitConfig,
    pub languages: LanguagesConfig,
    pub cmd_duration: CmdDurationConfig,
    pub status: StatusConfig,
    pub time: TimeConfig,
    pub identity: IdentityConfig,
    pub container: ContainerConfig,
    pub venv: VenvConfig,
    pub package: PackageConfig,
    pub shlvl: ShlvlConfig,
    pub os: OsConfig,
    pub autosuggest: AutosuggestConfig,
    pub menu: MenuConfig,
    pub history: HistoryConfig,
    pub highlight: HighlightConfig,
    pub aliases: Vec<(String, String)>,
    pub abbreviations: Vec<(String, String)>,
    pub env: Vec<(String, String)>,
    /// 子命令补全的用户扩展：命令链 -> [(子命令, 说明)]。
    /// 命令链带空格时用引号键写，例如 `[completions."git remote"]`。
    /// 同名项会盖掉内置表里的说明。
    pub completions: Vec<(String, Vec<(String, String)>)>,
    /// 预置脚本：名字 -> 完整命令。在命令位置作为候选出现。
    pub scripts: Vec<(String, String)>,
    pub loaded_from: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct CharacterConfig {
    pub success_symbol: String,
    pub error_symbol: String,
    pub success_style: String,
    pub error_style: String,
}

#[derive(Debug, Clone)]
pub struct DirConfig {
    pub style: String,
    pub truncation_length: usize,
    pub truncation_symbol: String,
    pub truncate_to_repo: bool,
    pub home_symbol: String,
}

#[derive(Debug, Clone)]
pub struct GitConfig {
    pub enabled: bool,
    pub branch_symbol: String,
    pub branch_style: String,
    pub status_enabled: bool,
    pub status_style: String,
    pub detached_symbol: String,
    /// rebase / merge / cherry-pick 这类「操作进行中」的状态。
    pub state_enabled: bool,
    pub state_style: String,
    pub commit_enabled: bool,
    pub commit_style: String,
    pub commit_length: usize,
}

#[derive(Debug, Clone)]
pub struct LanguagesConfig {
    pub enabled: bool,
    pub show_version: bool,
    /// 聚合模块 `$languages` 的前缀，渲染成 `via  v20.20.2`。
    /// 单独用 `$rust` 这类模块时不加。
    pub prefix: String,
    pub entries: Vec<LangSpec>,
}

#[derive(Debug, Clone)]
pub struct VenvConfig {
    pub enabled: bool,
    pub symbol: String,
    pub style: String,
}

#[derive(Debug, Clone)]
pub struct PackageConfig {
    pub enabled: bool,
    pub symbol: String,
    pub style: String,
}

#[derive(Debug, Clone)]
pub struct ShlvlConfig {
    pub enabled: bool,
    pub symbol: String,
    pub style: String,
    /// 超过这个层数才显示，默认 2（顶层 shell 不必提示）。
    pub threshold: usize,
}

#[derive(Debug, Clone)]
pub struct OsConfig {
    pub enabled: bool,
    pub style: String,
}

#[derive(Debug, Clone)]
pub struct LangSpec {
    pub name: String,
    pub enabled: bool,
    pub symbol: String,
    pub style: String,
    pub files: Vec<String>,
    pub extensions: Vec<String>,
    pub version_cmd: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CmdDurationConfig {
    pub enabled: bool,
    pub min_time_ms: u128,
    pub style: String,
    pub prefix: String,
}

#[derive(Debug, Clone)]
pub struct StatusConfig {
    pub enabled: bool,
    pub style: String,
    pub symbol: String,
}

#[derive(Debug, Clone)]
pub struct TimeConfig {
    pub enabled: bool,
    pub style: String,
    /// 与 UTC 的小时偏移，例如东八区填 8。
    pub utc_offset: f64,
    pub show_seconds: bool,
}

/// 三态可见性。`true` / `false` 仍然可用（向后兼容），另外接受 `"auto"`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Always,
    /// 只在「值得说明身份」的场合显示：root、SSH 会话、容器里。
    /// 本地日常开发时提示符保持干净，进了容器或远程机器自动带上。
    Auto,
    Never,
}

impl Visibility {
    fn parse(text: &str) -> Option<Self> {
        match text {
            "auto" => Some(Self::Auto),
            "always" | "true" | "yes" | "on" => Some(Self::Always),
            "never" | "false" | "no" | "off" => Some(Self::Never),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct IdentityConfig {
    pub show_user: Visibility,
    pub show_host: Visibility,
    pub style: String,
    /// root 单独上色——它是个需要被看见的状态。
    pub root_style: String,
    pub host_symbol: String,
    /// 连接词。`$username$hostname$dir` 渲染成 `root in 🌐 box in repo`。
    /// 放在模块内部而不是 format 字面量里，是为了模块取不到值时连接词
    /// 不会孤零零地留在提示符上。
    pub suffix: String,
}

#[derive(Debug, Clone)]
pub struct ContainerConfig {
    pub enabled: bool,
    pub symbol: String,
    pub style: String,
    /// 是否显示容器类型名，如 `⬢ [Docker]`。
    pub show_name: bool,
}

#[derive(Debug, Clone)]
pub struct AutosuggestConfig {
    pub enabled: bool,
    pub style: String,
    /// 建议来源顺序：history / completion
    pub sources: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct MenuConfig {
    pub enabled: bool,
    pub max_rows: usize,
    pub max_candidates: usize,
    pub show_descriptions: bool,
    pub auto_insert_common_prefix: bool,
    pub selected_style: String,
    pub description_style: String,
    pub hint_style: String,
}

#[derive(Debug, Clone)]
pub struct HistoryConfig {
    pub file: Option<PathBuf>,
    pub max_entries: usize,
    pub dedup: bool,
    pub ignore_space: bool,
    pub ignore_commands: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct HighlightConfig {
    pub enabled: bool,
    pub valid_command: String,
    pub invalid_command: String,
    pub string: String,
    pub operator: String,
    pub variable: String,
    pub option: String,
    pub comment: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            // `$all` 是第一行的信息区（见 prompt::ALL_MODULES），用它而不是写死模块
            // 列表：配置里写死的话，以后版本新增的模块永远不会出现。
            // container / shlvl 放第二行，贴着光标，和 starship 的观感一致。
            format: "$all$line_break$container$shlvl$character".to_string(),
            right_format: String::new(),
            add_newline: true,
            continuation_symbol: "∙ ".to_string(),
            continuation_style: "dimmed yellow".to_string(),
            character: CharacterConfig {
                success_symbol: "❯".to_string(),
                error_symbol: "❯".to_string(),
                success_style: "bold green".to_string(),
                error_style: "bold red".to_string(),
            },
            dir: DirConfig {
                style: "bold cyan".to_string(),
                truncation_length: 3,
                truncation_symbol: "…/".to_string(),
                truncate_to_repo: true,
                home_symbol: "~".to_string(),
            },
            git: GitConfig {
                enabled: true,
                branch_symbol: "".to_string(),
                branch_style: "bold purple".to_string(),
                status_enabled: true,
                status_style: "bold red".to_string(),
                detached_symbol: "@".to_string(),
                state_enabled: true,
                state_style: "bold yellow".to_string(),
                commit_enabled: false,
                commit_style: "dimmed green".to_string(),
                commit_length: 7,
            },
            languages: LanguagesConfig {
                enabled: true,
                show_version: true,
                prefix: "via ".to_string(),
                entries: default_languages(),
            },
            cmd_duration: CmdDurationConfig {
                enabled: true,
                min_time_ms: 2000,
                style: "bold yellow".to_string(),
                prefix: "took ".to_string(),
            },
            status: StatusConfig {
                enabled: true,
                style: "bold red".to_string(),
                symbol: "✘ ".to_string(),
            },
            time: TimeConfig {
                enabled: false,
                style: "dimmed white".to_string(),
                utc_offset: 0.0,
                show_seconds: false,
            },
            identity: IdentityConfig {
                show_user: Visibility::Auto,
                show_host: Visibility::Auto,
                style: "bold green".to_string(),
                root_style: "bold red".to_string(),
                host_symbol: "🌐 ".to_string(),
                suffix: " in ".to_string(),
            },
            container: ContainerConfig {
                enabled: true,
                symbol: "⬢".to_string(),
                style: "bold dimmed red".to_string(),
                show_name: true,
            },
            venv: VenvConfig {
                enabled: true,
                symbol: String::new(),
                style: "dimmed cyan".to_string(),
            },
            package: PackageConfig {
                enabled: false,
                symbol: "📦 ".to_string(),
                style: "bold blue".to_string(),
            },
            shlvl: ShlvlConfig {
                enabled: false,
                symbol: "↕ ".to_string(),
                style: "bold yellow".to_string(),
                threshold: 2,
            },
            os: OsConfig {
                enabled: false,
                style: "bold white".to_string(),
            },
            autosuggest: AutosuggestConfig {
                enabled: true,
                style: "dimmed".to_string(),
                sources: vec!["history".to_string(), "completion".to_string()],
            },
            menu: MenuConfig {
                enabled: true,
                max_rows: 8,
                max_candidates: 200,
                show_descriptions: true,
                auto_insert_common_prefix: true,
                selected_style: "bold fg:black bg:cyan".to_string(),
                description_style: "dimmed".to_string(),
                hint_style: "dimmed italic".to_string(),
            },
            history: HistoryConfig {
                file: None,
                max_entries: 20_000,
                dedup: true,
                ignore_space: true,
                ignore_commands: vec!["exit".to_string(), "clear".to_string()],
            },
            highlight: HighlightConfig {
                enabled: true,
                valid_command: "green".to_string(),
                invalid_command: "red".to_string(),
                string: "yellow".to_string(),
                operator: "purple".to_string(),
                variable: "cyan".to_string(),
                option: "bold blue".to_string(),
                comment: "dimmed".to_string(),
            },
            aliases: Vec::new(),
            abbreviations: Vec::new(),
            env: Vec::new(),
            completions: Vec::new(),
            scripts: Vec::new(),
            loaded_from: None,
        }
    }
}

fn lang(
    name: &str,
    symbol: &str,
    style: &str,
    files: &[&str],
    extensions: &[&str],
    version_cmd: &[&str],
) -> LangSpec {
    LangSpec {
        name: name.to_string(),
        enabled: true,
        symbol: symbol.to_string(),
        style: style.to_string(),
        files: files.iter().map(|s| s.to_string()).collect(),
        extensions: extensions.iter().map(|s| s.to_string()).collect(),
        version_cmd: version_cmd.iter().map(|s| s.to_string()).collect(),
    }
}

fn default_languages() -> Vec<LangSpec> {
    vec![
        lang(
            "rust",
            "🦀 ",
            "bold red",
            &["Cargo.toml"],
            &["rs"],
            &["rustc", "--version"],
        ),
        lang(
            "nodejs",
            "⬢ ",
            "bold green",
            &["package.json", ".nvmrc"],
            &["js", "mjs", "cjs", "ts", "tsx"],
            &["node", "--version"],
        ),
        lang(
            "python",
            "🐍 ",
            "bold yellow",
            &[
                "requirements.txt",
                "pyproject.toml",
                "setup.py",
                ".python-version",
            ],
            &["py"],
            &["python3", "--version"],
        ),
        lang(
            "golang",
            "🐹 ",
            "bold cyan",
            &["go.mod", "go.sum"],
            &["go"],
            &["go", "version"],
        ),
        lang(
            "java",
            "☕ ",
            "bold blue",
            &["pom.xml", "build.gradle", "build.gradle.kts"],
            &["java"],
            &["java", "-version"],
        ),
    ]
}

impl Config {
    /// 配置文件路径：`$CMDS_CONFIG` 优先。
    pub fn path() -> PathBuf {
        if let Some(path) = std::env::var_os("CMDS_CONFIG") {
            return PathBuf::from(path);
        }
        util::config_dir().join("config.toml")
    }

    /// 加载配置，返回 (配置, 警告列表)。缺失文件视为使用默认值。
    pub fn load() -> (Self, Vec<String>) {
        let path = Self::path();
        let mut warnings = Vec::new();
        let mut config = Config::default();
        match std::fs::read_to_string(&path) {
            Ok(src) => match Self::parse_str(&src) {
                Ok(parsed) => {
                    config = parsed;
                    config.loaded_from = Some(path);
                }
                Err(err) => warnings.push(tf!(
                    "cannot parse config ({}): {err}",
                    "配置解析失败（{}）：{err}",
                    path.display()
                )),
            },
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => warnings.push(tf!(
                "cannot read config ({}): {err}",
                "配置读取失败（{}）：{err}",
                path.display()
            )),
        }
        (config, warnings)
    }

    pub fn parse_str(src: &str) -> Result<Self, String> {
        let table = toml::parse(src)?;
        let mut config = Config::default();
        config.apply(&table);
        Ok(config)
    }

    fn apply(&mut self, table: &Table) {
        if let Some(v) = table.string("format") {
            self.format = v;
        }
        if let Some(v) = table.string("right_format") {
            self.right_format = v;
        }
        if let Some(v) = table.bool("add_newline") {
            self.add_newline = v;
        }
        if let Some(v) = table.string("continuation_symbol") {
            self.continuation_symbol = v;
        }
        if let Some(v) = table.string("continuation_style") {
            self.continuation_style = v;
        }

        let c = &mut self.character;
        set_str(table, "character.success_symbol", &mut c.success_symbol);
        set_str(table, "character.error_symbol", &mut c.error_symbol);
        set_str(table, "character.success_style", &mut c.success_style);
        set_str(table, "character.error_style", &mut c.error_style);

        let d = &mut self.dir;
        set_str(table, "dir.style", &mut d.style);
        set_usize(table, "dir.truncation_length", &mut d.truncation_length);
        set_str(table, "dir.truncation_symbol", &mut d.truncation_symbol);
        set_bool(table, "dir.truncate_to_repo", &mut d.truncate_to_repo);
        set_str(table, "dir.home_symbol", &mut d.home_symbol);

        let g = &mut self.git;
        set_bool(table, "git.enabled", &mut g.enabled);
        set_str(table, "git.branch_symbol", &mut g.branch_symbol);
        set_str(table, "git.branch_style", &mut g.branch_style);
        set_bool(table, "git.status_enabled", &mut g.status_enabled);
        set_str(table, "git.status_style", &mut g.status_style);
        set_str(table, "git.detached_symbol", &mut g.detached_symbol);
        set_bool(table, "git.state_enabled", &mut g.state_enabled);
        set_str(table, "git.state_style", &mut g.state_style);
        set_bool(table, "git.commit_enabled", &mut g.commit_enabled);
        set_str(table, "git.commit_style", &mut g.commit_style);
        set_usize(table, "git.commit_length", &mut g.commit_length);

        set_bool(table, "languages.enabled", &mut self.languages.enabled);
        set_bool(
            table,
            "languages.show_version",
            &mut self.languages.show_version,
        );
        set_str(table, "languages.prefix", &mut self.languages.prefix);
        for spec in &mut self.languages.entries {
            let prefix = format!("languages.{}", spec.name);
            set_bool(table, &format!("{prefix}.enabled"), &mut spec.enabled);
            set_str(table, &format!("{prefix}.symbol"), &mut spec.symbol);
            set_str(table, &format!("{prefix}.style"), &mut spec.style);
            if let Some(files) = table.string_list(&format!("{prefix}.files")) {
                spec.files = files;
            }
            if let Some(exts) = table.string_list(&format!("{prefix}.extensions")) {
                spec.extensions = exts;
            }
            if let Some(cmd) = table.string_list(&format!("{prefix}.version_cmd")) {
                spec.version_cmd = cmd;
            }
        }

        let cd = &mut self.cmd_duration;
        set_bool(table, "cmd_duration.enabled", &mut cd.enabled);
        if let Some(v) = table.int("cmd_duration.min_time_ms") {
            cd.min_time_ms = v.max(0) as u128;
        }
        set_str(table, "cmd_duration.style", &mut cd.style);
        set_str(table, "cmd_duration.prefix", &mut cd.prefix);

        let s = &mut self.status;
        set_bool(table, "status.enabled", &mut s.enabled);
        set_str(table, "status.style", &mut s.style);
        set_str(table, "status.symbol", &mut s.symbol);

        let t = &mut self.time;
        set_bool(table, "time.enabled", &mut t.enabled);
        set_str(table, "time.style", &mut t.style);
        if let Some(v) = table.lookup("time.utc_offset") {
            t.utc_offset = match v {
                self::toml::Value::Float(f) => *f,
                self::toml::Value::Int(i) => *i as f64,
                _ => t.utc_offset,
            };
        }
        set_bool(table, "time.show_seconds", &mut t.show_seconds);

        let id = &mut self.identity;
        set_visibility(table, "identity.show_user", &mut id.show_user);
        set_visibility(table, "identity.show_host", &mut id.show_host);
        set_str(table, "identity.style", &mut id.style);
        set_str(table, "identity.root_style", &mut id.root_style);
        set_str(table, "identity.host_symbol", &mut id.host_symbol);
        set_str(table, "identity.suffix", &mut id.suffix);

        let ct = &mut self.container;
        set_bool(table, "container.enabled", &mut ct.enabled);
        set_str(table, "container.symbol", &mut ct.symbol);
        set_str(table, "container.style", &mut ct.style);
        set_bool(table, "container.show_name", &mut ct.show_name);

        let ve = &mut self.venv;
        set_bool(table, "venv.enabled", &mut ve.enabled);
        set_str(table, "venv.symbol", &mut ve.symbol);
        set_str(table, "venv.style", &mut ve.style);

        let pk = &mut self.package;
        set_bool(table, "package.enabled", &mut pk.enabled);
        set_str(table, "package.symbol", &mut pk.symbol);
        set_str(table, "package.style", &mut pk.style);

        let sl = &mut self.shlvl;
        set_bool(table, "shlvl.enabled", &mut sl.enabled);
        set_str(table, "shlvl.symbol", &mut sl.symbol);
        set_str(table, "shlvl.style", &mut sl.style);
        set_usize(table, "shlvl.threshold", &mut sl.threshold);

        let os = &mut self.os;
        set_bool(table, "os.enabled", &mut os.enabled);
        set_str(table, "os.style", &mut os.style);

        let a = &mut self.autosuggest;
        set_bool(table, "autosuggest.enabled", &mut a.enabled);
        set_str(table, "autosuggest.style", &mut a.style);
        if let Some(sources) = table.string_list("autosuggest.sources") {
            a.sources = sources;
        }

        let m = &mut self.menu;
        set_bool(table, "menu.enabled", &mut m.enabled);
        set_usize(table, "menu.max_rows", &mut m.max_rows);
        set_usize(table, "menu.max_candidates", &mut m.max_candidates);
        set_bool(table, "menu.show_descriptions", &mut m.show_descriptions);
        set_bool(
            table,
            "menu.auto_insert_common_prefix",
            &mut m.auto_insert_common_prefix,
        );
        set_str(table, "menu.selected_style", &mut m.selected_style);
        set_str(table, "menu.description_style", &mut m.description_style);
        set_str(table, "menu.hint_style", &mut m.hint_style);

        let h = &mut self.history;
        if let Some(file) = table.string("history.file") {
            h.file = Some(PathBuf::from(util::expand_tilde(&file)));
        }
        set_usize(table, "history.max_entries", &mut h.max_entries);
        set_bool(table, "history.dedup", &mut h.dedup);
        set_bool(table, "history.ignore_space", &mut h.ignore_space);
        if let Some(list) = table.string_list("history.ignore_commands") {
            h.ignore_commands = list;
        }

        let hl = &mut self.highlight;
        set_bool(table, "highlight.enabled", &mut hl.enabled);
        set_str(table, "highlight.valid_command", &mut hl.valid_command);
        set_str(table, "highlight.invalid_command", &mut hl.invalid_command);
        set_str(table, "highlight.string", &mut hl.string);
        set_str(table, "highlight.operator", &mut hl.operator);
        set_str(table, "highlight.variable", &mut hl.variable);
        set_str(table, "highlight.option", &mut hl.option);
        set_str(table, "highlight.comment", &mut hl.comment);

        self.aliases = table.str_pairs("aliases");
        self.abbreviations = table.str_pairs("abbreviations");
        self.env = table.str_pairs("env");
        self.scripts = table.str_pairs("scripts");

        // 逐层走 Value 而不是拼 "completions.<chain>" 路径字符串：
        // 命令链里可能带空格甚至点（`[completions."git remote"]`），
        // 拼路径再按 '.' 切会把键切坏。
        self.completions.clear();
        if let Some(completions) = table.table("completions") {
            for (chain, value) in completions.iter() {
                if let self::toml::Value::Table(inner) = value {
                    let entries: Vec<(String, String)> = inner
                        .iter()
                        .filter_map(|(name, v)| match v {
                            self::toml::Value::Str(s) => Some((name.to_string(), s.clone())),
                            _ => None,
                        })
                        .collect();
                    if !entries.is_empty() {
                        self.completions.push((chain.to_string(), entries));
                    }
                }
            }
        }
    }

    /// 历史文件路径。
    pub fn history_path(&self) -> PathBuf {
        self.history
            .file
            .clone()
            .unwrap_or_else(|| util::data_dir().join("history"))
    }

    pub fn lang(&self, name: &str) -> Option<&LangSpec> {
        self.languages.entries.iter().find(|l| l.name == name)
    }
}

fn set_str(table: &Table, path: &str, slot: &mut String) {
    if let Some(v) = table.string(path) {
        *slot = v;
    }
}

fn set_bool(table: &Table, path: &str, slot: &mut bool) {
    if let Some(v) = table.bool(path) {
        *slot = v;
    }
}

fn set_usize(table: &Table, path: &str, slot: &mut usize) {
    if let Some(v) = table.usize(path) {
        *slot = v;
    }
}

/// `Table::string` 会把 `Value::Bool` 也转成 "true"/"false"，
/// 所以一次 string 取值就能同时覆盖 bool 与 "auto" 两种写法。
fn set_visibility(table: &Table, path: &str, slot: &mut Visibility) {
    if let Some(text) = table.string(path) {
        if let Some(parsed) = Visibility::parse(&text) {
            *slot = parsed;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_template_parses() {
        for (name, template) in [("en", DEFAULT_TEMPLATE_EN), ("zh", DEFAULT_TEMPLATE_ZH)] {
            let config =
                Config::parse_str(template).unwrap_or_else(|e| panic!("{name} 模板应可解析：{e}"));
            assert!(
                config.format.contains("$character"),
                "{name} 模板缺 $character"
            );
        }
    }

    /// 两份模板只该在注释上不同。把「去掉注释和空行后逐字节相同」钉住，
    /// 否则改了一份的默认值、另一份没跟上，两种语言的用户拿到的默认配置就不一样了。
    #[test]
    fn config_template_matches_across_languages() {
        fn significant(template: &str) -> Vec<String> {
            template
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && !line.starts_with('#'))
                .map(str::to_string)
                .collect()
        }

        let en = significant(DEFAULT_TEMPLATE_EN);
        let zh = significant(DEFAULT_TEMPLATE_ZH);
        assert_eq!(
            en.len(),
            zh.len(),
            "两份模板的有效行数不同：en {} 行 / zh {} 行",
            en.len(),
            zh.len()
        );
        for (line_en, line_zh) in en.iter().zip(&zh) {
            assert_eq!(line_en, line_zh, "两份模板的键值不一致");
        }
    }

    /// 英文模板不该带中文——它是默认写出的那份。
    #[test]
    fn english_template_has_no_chinese() {
        let offender = DEFAULT_TEMPLATE_EN
            .lines()
            .find(|line| line.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c)));
        assert!(offender.is_none(), "英文模板里有中文：{offender:?}");
    }

    #[test]
    fn overrides_nested_values() {
        let config = Config::parse_str(
            r#"
add_newline = false

[character]
success_symbol = ">"

[menu]
max_rows = 12

[languages.rust]
enabled = false

[aliases]
ll = "ls -lah"
"#,
        )
        .expect("解析失败");
        assert!(!config.add_newline);
        assert_eq!(config.character.success_symbol, ">");
        assert_eq!(config.menu.max_rows, 12);
        assert!(!config.lang("rust").unwrap().enabled);
        assert_eq!(config.aliases.len(), 1);
    }

    /// `show_user` 从 bool 变成了三态。原来写 `true` / `false` 的配置必须照旧生效，
    /// 否则升级会静默改变别人的提示符。
    #[test]
    fn visibility_accepts_bool_and_auto() {
        let cases = [
            ("true", Visibility::Always),
            ("false", Visibility::Never),
            ("\"always\"", Visibility::Always),
            ("\"never\"", Visibility::Never),
            ("\"auto\"", Visibility::Auto),
        ];
        for (literal, expected) in cases {
            let src = format!("[identity]\nshow_user = {literal}\n");
            let config = Config::parse_str(&src).expect("解析失败");
            assert_eq!(
                config.identity.show_user, expected,
                "show_user = {literal} 应解析成 {expected:?}"
            );
        }
        // 不认识的值保持默认，不该把配置打成 Never（那会静默隐藏信息）
        let config = Config::parse_str("[identity]\nshow_user = \"sometimes\"\n").unwrap();
        assert_eq!(config.identity.show_user, Visibility::Auto);
    }

    /// 模板注释里写了内置子命令表的数量。这种数字必然漂移——加了新命令表却
    /// 忘了改注释，用户看到的就是错的。把它钉在真实值上。
    #[test]
    fn template_states_the_real_table_count() {
        let count = crate::editor::subcommands::chains().count();
        for (name, template, needle) in [
            ("en", DEFAULT_TEMPLATE_EN, format!("{count} command tables")),
            ("zh", DEFAULT_TEMPLATE_ZH, format!("{count} 张命令表")),
        ] {
            assert!(
                template.contains(&needle),
                "{name} 模板里的内置命令表数量不是 {count}（应包含「{needle}」）"
            );
        }
    }

    /// 模板给出的默认 format 必须和代码里的默认值一致，否则
    /// 「装完直接用」和「先 config init」两条路会得到不一样的提示符。
    #[test]
    fn template_format_matches_code_default() {
        let default = Config::default();
        for (name, template) in [("en", DEFAULT_TEMPLATE_EN), ("zh", DEFAULT_TEMPLATE_ZH)] {
            let parsed = Config::parse_str(template).expect("模板可解析");
            assert_eq!(
                parsed.format, default.format,
                "{name} 模板的 format 和代码默认值不一致"
            );
        }
    }

    /// 预置脚本不该带破坏性命令：它们会出现在候选菜单里，一个回车就执行了。
    #[test]
    fn preset_scripts_are_not_destructive() {
        let config = Config::parse_str(DEFAULT_TEMPLATE_EN).expect("模板可解析");
        assert!(!config.scripts.is_empty(), "模板应预置一些脚本");
        for (name, command) in &config.scripts {
            // `git clean` 必须是 dry-run；带 --hard / -f 的命令不该预置
            if command.contains("git clean") {
                assert!(
                    command.contains("-n"),
                    "预置脚本 {name} 会真的删文件：{command}"
                );
            }
            for danger in ["--hard", "push --force", "rm -rf"] {
                assert!(
                    !command.contains(danger),
                    "预置脚本 {name} 含危险操作 {danger}：{command}"
                );
            }
        }
    }

    #[test]
    fn container_module_is_configurable() {
        let config = Config::parse_str(
            r#"
[container]
symbol = "📦"
show_name = false
"#,
        )
        .expect("解析失败");
        assert_eq!(config.container.symbol, "📦");
        assert!(!config.container.show_name);
        assert!(config.container.enabled);
    }
}

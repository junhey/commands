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
    pub autosuggest: AutosuggestConfig,
    pub menu: MenuConfig,
    pub history: HistoryConfig,
    pub highlight: HighlightConfig,
    pub aliases: Vec<(String, String)>,
    pub abbreviations: Vec<(String, String)>,
    pub env: Vec<(String, String)>,
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
}

#[derive(Debug, Clone)]
pub struct LanguagesConfig {
    pub enabled: bool,
    pub show_version: bool,
    pub entries: Vec<LangSpec>,
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

#[derive(Debug, Clone)]
pub struct IdentityConfig {
    pub show_user: bool,
    pub show_host: bool,
    pub style: String,
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
            format: "$identity$dir$git_branch$git_status$languages$cmd_duration$status$line_break$character".to_string(),
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
            },
            languages: LanguagesConfig {
                enabled: true,
                show_version: true,
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
                show_user: false,
                show_host: false,
                style: "bold dimmed green".to_string(),
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

        set_bool(table, "languages.enabled", &mut self.languages.enabled);
        set_bool(
            table,
            "languages.show_version",
            &mut self.languages.show_version,
        );
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
        set_bool(table, "identity.show_user", &mut id.show_user);
        set_bool(table, "identity.show_host", &mut id.show_host);
        set_str(table, "identity.style", &mut id.style);

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
}

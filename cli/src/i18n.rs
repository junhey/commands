//! 界面语言。**默认英文**，只有检测到中文 locale 才输出中文。
//!
//! 判断只看环境变量：`CMDS_LANG` 最高，之后与 POSIX 一致
//! `LC_ALL` > `LC_MESSAGES` > `LANG`。`C` / `POSIX` / 未设置都算英文。
//!
//! `CMDS_LANG` 是给「系统是中文、但想要英文界面」的人留的逃生口（反之亦然），
//! 而且必须和 `install/install.sh` 认的变量保持一致——安装脚本支持一个名字、
//! 装好的程序认另一个名字，是最让人火大的那种不一致。
//!
//! 这里没有引入 gettext 那一套：整个产品的文案量很小，把中英两份写在同一行
//! 反而最难写错——改文案时两种语言就在眼前，不会出现只改一边的情况，也不需要
//! 额外的资源文件跟着二进制走。

use std::sync::OnceLock;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Lang {
    En,
    Zh,
}

static CURRENT: OnceLock<Lang> = OnceLock::new();

/// 语言相关环境变量，按优先级从高到低。
///
/// `CMDS_LANG` 在最前面：它是显式意图，应当盖过系统 locale。名字必须和
/// `install/install.sh` 里认的那个一致。
const LANG_KEYS: [&str; 4] = ["CMDS_LANG", "LC_ALL", "LC_MESSAGES", "LANG"];

/// 当前语言。只在首次调用时读环境变量，之后整个进程固定。
///
/// 固定下来有两个好处：省掉每条提示都查一遍环境；也避免像
/// `std::env::set_current_dir` 那样成为进程级可变状态，让并行测试互相干扰。
pub fn lang() -> Lang {
    *CURRENT.get_or_init(detect_from_env)
}

fn detect_from_env() -> Lang {
    for key in LANG_KEYS {
        match std::env::var(key) {
            // 空值等于没设置，继续看下一个：`LANG= cmds` 不该被当成中文。
            Ok(value) if !value.is_empty() => return from_locale(&value),
            _ => continue,
        }
    }
    Lang::En
}

/// 从 locale 字符串判断语言。独立成纯函数，好测。
///
/// `zh`、`zh_CN.UTF-8`、`zh-Hans`、`zh_TW` 都算中文；没有别的语言代码以 `zh`
/// 开头，所以前缀判断就够了。其余一律英文——包括 `C`、`POSIX` 和其它语种，
/// 我们只有这两套文案，不认识的语言给英文比给中文更合适。
pub fn from_locale(locale: &str) -> Lang {
    if locale.to_ascii_lowercase().starts_with("zh") {
        Lang::Zh
    } else {
        Lang::En
    }
}

/// 按当前语言选一条静态文案。
pub fn t(en: &'static str, zh: &'static str) -> &'static str {
    match lang() {
        Lang::En => en,
        Lang::Zh => zh,
    }
}

/// 选一条静态文案：`t!("Done", "完成")`。
#[macro_export]
macro_rules! t {
    ($en:expr, $zh:expr) => {
        $crate::i18n::t($en, $zh)
    };
}

/// 选一条格式化文案：`tf!("no such dir: {}", "目录不存在：{}", path)`。
///
/// 两个分支各自展开成独立的 `format!`，参数个数与类型仍由编译器检查；
/// 先 `t!` 拿到格式串再 `format!` 是不行的，`format!` 的第一个参数必须是字面量。
#[macro_export]
macro_rules! tf {
    ($en:literal, $zh:literal $(, $arg:expr)* $(,)?) => {
        match $crate::i18n::lang() {
            $crate::i18n::Lang::En => format!($en $(, $arg)*),
            $crate::i18n::Lang::Zh => format!($zh $(, $arg)*),
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chinese_locales_pick_chinese() {
        for locale in [
            "zh",
            "zh_CN",
            "zh_CN.UTF-8",
            "zh_TW.Big5",
            "zh-Hans",
            "zh-Hant-HK",
            "ZH_CN.UTF-8",
        ] {
            assert_eq!(from_locale(locale), Lang::Zh, "{locale} 应该是中文");
        }
    }

    #[test]
    fn everything_else_falls_back_to_english() {
        // C / POSIX 是「无本地化」的意思，给英文；不认识的语种也给英文，
        // 因为我们只有中英两套文案，英文是更安全的默认。
        for locale in [
            "C",
            "POSIX",
            "en_US.UTF-8",
            "en",
            "ja_JP.UTF-8",
            "de_DE",
            "",
            "zzz",
        ] {
            assert_eq!(from_locale(locale), Lang::En, "{locale} 应该是英文");
        }
    }

    #[test]
    fn cmds_lang_outranks_the_system_locale() {
        // 显式意图要盖过系统 locale：`CMDS_LANG=en` 在中文系统上也得给英文。
        // 这里钉的是真实的 LANG_KEYS 顺序，不是另抄一份。
        let position = |key| LANG_KEYS.iter().position(|k| *k == key);
        assert_eq!(position("CMDS_LANG"), Some(0));
        assert!(position("CMDS_LANG") < position("LC_ALL"));
        assert!(position("LC_ALL") < position("LC_MESSAGES"));
        assert!(position("LC_MESSAGES") < position("LANG"));
        // 端到端的「LANG=zh_CN 时 CMDS_LANG=en 仍是英文」在
        // scripts/check-cli-language.sh 里跑真实进程验证——改环境变量会污染并行测试。
    }

    #[test]
    fn language_is_stable_within_a_run() {
        // lang() 第一次求值后就固定，多次调用必须一致——
        // 否则同一次运行里的提示会中英混排。
        let first = lang();
        for _ in 0..8 {
            assert_eq!(lang(), first);
        }
    }

    #[test]
    fn picks_the_matching_side() {
        let picked = t("english", "中文");
        match lang() {
            Lang::En => assert_eq!(picked, "english"),
            Lang::Zh => assert_eq!(picked, "中文"),
        }
    }

    #[test]
    fn format_macro_keeps_arguments() {
        let rendered = tf!("count {} of {}", "第 {} 项，共 {} 项", 2, 5);
        assert!(rendered.contains('2') && rendered.contains('5'));
    }
}

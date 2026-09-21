//! 样式描述 -> ANSI 序列。
//!
//! 语法参考 starship：`"bold green"`、`"fg:#ff8800 bg:blue dimmed"`、`"fg:214"`、`"none"`。

use std::sync::atomic::{AtomicBool, Ordering};
use unicode_width::UnicodeWidthChar;

static NO_COLOR: AtomicBool = AtomicBool::new(false);

pub const RESET: &str = "\x1b[0m";

pub fn set_color_enabled(on: bool) {
    NO_COLOR.store(!on, Ordering::Relaxed);
}

pub fn color_enabled() -> bool {
    !NO_COLOR.load(Ordering::Relaxed)
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum Layer {
    Fg,
    Bg,
}

/// 把样式描述编译成 ANSI 前缀；无有效样式时返回空串。
pub fn ansi(spec: &str) -> String {
    if !color_enabled() {
        return String::new();
    }
    let spec = spec.trim();
    if spec.is_empty() || spec.eq_ignore_ascii_case("none") {
        return String::new();
    }
    let mut codes: Vec<String> = Vec::new();
    for token in spec.split_whitespace() {
        let (layer, value, explicit) = match token.split_once(':') {
            Some((k, v)) if k.eq_ignore_ascii_case("fg") => (Layer::Fg, v, true),
            Some((k, v)) if k.eq_ignore_ascii_case("bg") => (Layer::Bg, v, true),
            _ => (Layer::Fg, token, false),
        };
        if !explicit {
            if let Some(code) = attribute(value) {
                codes.push(code.to_string());
                continue;
            }
        }
        if let Some(code) = color(value, layer) {
            codes.push(code);
        }
    }
    if codes.is_empty() {
        String::new()
    } else {
        format!("\x1b[{}m", codes.join(";"))
    }
}

/// 给文本套上样式，空文本返回空串（方便模块拼接）。
pub fn paint(spec: &str, text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    let prefix = ansi(spec);
    if prefix.is_empty() {
        text.to_string()
    } else {
        format!("{prefix}{text}{RESET}")
    }
}

fn attribute(name: &str) -> Option<&'static str> {
    Some(match name.to_ascii_lowercase().as_str() {
        "bold" => "1",
        "dimmed" | "dim" => "2",
        "italic" => "3",
        "underline" => "4",
        "blink" => "5",
        "inverted" | "reverse" => "7",
        "hidden" => "8",
        "strikethrough" => "9",
        _ => return None,
    })
}

fn color(name: &str, layer: Layer) -> Option<String> {
    let lower = name.to_ascii_lowercase();
    let (name, bright) = match lower.strip_prefix("bright-") {
        Some(rest) => (rest.to_string(), true),
        None => (lower, false),
    };
    let named = match name.as_str() {
        "black" => Some(0),
        "red" => Some(1),
        "green" => Some(2),
        "yellow" => Some(3),
        "blue" => Some(4),
        "purple" | "magenta" => Some(5),
        "cyan" => Some(6),
        "white" => Some(7),
        _ => None,
    };
    if let Some(offset) = named {
        let start = match (layer, bright) {
            (Layer::Fg, false) => 30,
            (Layer::Fg, true) => 90,
            (Layer::Bg, false) => 40,
            (Layer::Bg, true) => 100,
        };
        return Some((start + offset).to_string());
    }
    let lead = if layer == Layer::Fg { 38 } else { 48 };
    if let Some(hex) = name.strip_prefix('#') {
        let (r, g, b) = parse_hex(hex)?;
        return Some(format!("{lead};2;{r};{g};{b}"));
    }
    if let Ok(n) = name.parse::<u8>() {
        return Some(format!("{lead};5;{n}"));
    }
    None
}

fn parse_hex(hex: &str) -> Option<(u8, u8, u8)> {
    let digits: Vec<u8> = hex
        .chars()
        .map(|c| c.to_digit(16).map(|d| d as u8))
        .collect::<Option<Vec<u8>>>()?;
    match digits.len() {
        3 => Some((digits[0] * 17, digits[1] * 17, digits[2] * 17)),
        6 => Some((
            digits[0] * 16 + digits[1],
            digits[2] * 16 + digits[3],
            digits[4] * 16 + digits[5],
        )),
        _ => None,
    }
}

/// 单字符显示宽度（控制字符按 0 计）。
pub fn char_width(c: char) -> usize {
    c.width().unwrap_or(0)
}

/// 忽略 ANSI 转义后的可见宽度。
pub fn visible_width(s: &str) -> usize {
    let mut width = 0;
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            for e in chars.by_ref() {
                if e.is_ascii_alphabetic() || e == '\u{7}' {
                    break;
                }
            }
            continue;
        }
        width += char_width(c);
    }
    width
}

/// 按可见宽度截断（超出时补 `…`）。
pub fn truncate_visible(s: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    if visible_width(s) <= max {
        return s.to_string();
    }
    let mut out = String::new();
    let mut width = 0;
    for c in s.chars() {
        let w = char_width(c);
        if width + w > max.saturating_sub(1) {
            break;
        }
        width += w;
        out.push(c);
    }
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiles_basic_styles() {
        set_color_enabled(true);
        assert_eq!(ansi("bold green"), "\x1b[1;32m");
        assert_eq!(ansi("fg:#ff0000"), "\x1b[38;2;255;0;0m");
        assert_eq!(ansi("bg:4"), "\x1b[48;5;4m");
        assert_eq!(ansi("none"), "");
    }

    #[test]
    fn measures_visible_width() {
        assert_eq!(visible_width("\x1b[1;32mab\x1b[0m"), 2);
        assert_eq!(visible_width("中文"), 4);
    }

    #[test]
    fn truncates_by_width() {
        assert_eq!(truncate_visible("abcdef", 4), "abc…");
        assert_eq!(truncate_visible("abc", 4), "abc");
    }
}

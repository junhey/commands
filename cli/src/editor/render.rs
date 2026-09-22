//! 渲染：按显示宽度折行、绘制高亮/灰色建议、绘制候选菜单。

use super::Menu;
use crate::config::MenuConfig;
use crate::style;
use crossterm::QueueableCommand;
use crossterm::cursor::{MoveDown, MoveToColumn, MoveUp};
use crossterm::terminal::{Clear, ClearType};
use std::io::Write;
use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    pub lines: Vec<String>,
    pub cursor_row: usize,
    pub cursor_col: usize,
}

struct Painter {
    width: usize,
    lines: Vec<String>,
    col: usize,
    current_style: String,
}

impl Painter {
    fn new(width: usize) -> Self {
        Self {
            width: width.max(1),
            lines: vec![String::new()],
            col: 0,
            current_style: String::new(),
        }
    }

    fn line(&mut self) -> &mut String {
        self.lines.last_mut().expect("至少有一行")
    }

    fn close_style(&mut self) {
        if self.current_style.is_empty() {
            return;
        }
        self.current_style.clear();
        self.line().push_str(style::RESET);
    }

    fn newline(&mut self) {
        self.close_style();
        self.lines.push(String::new());
        self.col = 0;
    }

    fn set_style(&mut self, spec: &str) {
        if self.current_style == spec {
            return;
        }
        self.close_style();
        if spec.is_empty() {
            return;
        }
        let ansi = style::ansi(spec);
        if !ansi.is_empty() {
            self.line().push_str(&ansi);
            self.current_style = spec.to_string();
        }
    }

    fn push_char(&mut self, c: char) {
        let width = style::char_width(c);
        if self.col + width > self.width {
            let saved = self.current_style.clone();
            self.newline();
            self.set_style(&saved);
        }
        self.line().push(c);
        self.col += width;
    }

    /// 追加已经带 ANSI 的文本（提示符）。
    fn push_preformatted(&mut self, text: &str) {
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\x1b' {
                let mut escape = String::from(c);
                for e in chars.by_ref() {
                    escape.push(e);
                    if e.is_ascii_alphabetic() {
                        break;
                    }
                }
                self.line().push_str(&escape);
                continue;
            }
            if c == '\n' {
                self.newline();
                continue;
            }
            if c == '\r' {
                continue;
            }
            self.push_char(c);
        }
    }

    fn position(&self) -> (usize, usize) {
        (self.lines.len() - 1, self.col)
    }

    fn finish(mut self) -> Vec<String> {
        self.close_style();
        self.lines
    }
}

/// 计算一屏内容的布局与光标位置。
pub fn layout(
    width: usize,
    prompt: &str,
    buffer: &str,
    spans: &[(Range<usize>, String)],
    ghost: &str,
    ghost_style: &str,
    cursor: usize,
) -> Layout {
    let mut painter = Painter::new(width);
    painter.push_preformatted(prompt);
    let mut position = painter.position();

    for (index, ch) in buffer.char_indices() {
        if index == cursor {
            position = painter.position();
        }
        if ch == '\n' {
            painter.newline();
            continue;
        }
        let spec = super::highlight::style_at(spans, index).to_string();
        painter.set_style(&spec);
        painter.push_char(ch);
    }
    if cursor >= buffer.len() {
        position = painter.position();
    }

    if !ghost.is_empty() {
        painter.set_style(ghost_style);
        for ch in ghost.chars() {
            if ch == '\n' {
                painter.newline();
            } else {
                painter.push_char(ch);
            }
        }
    }

    Layout {
        lines: painter.finish(),
        cursor_row: position.0,
        cursor_col: position.1,
    }
}

/// 候选菜单的每一行（含底部操作提示）。
pub fn menu_lines(menu: &Menu, config: &MenuConfig, width: usize) -> Vec<String> {
    let max_rows = config.max_rows.max(1);
    let total = menu.items.len();
    let mut lines = Vec::new();
    let start = menu.offset.min(total.saturating_sub(1));
    let end = (start + max_rows).min(total);

    for (offset, item) in menu.items[start..end].iter().enumerate() {
        let index = start + offset;
        let selected = index == menu.selected;
        let mut text = format!("{}{}", if selected { "▸ " } else { "  " }, item.display);
        if config.show_descriptions && !item.description.is_empty() {
            let used = style::visible_width(&text);
            let budget = width.saturating_sub(used + 2);
            if budget > 4 {
                let detail = format!("  {} · {}", item.kind.label(), item.description);
                text.push_str(&style::truncate_visible(&detail, budget));
            }
        }
        let text = style::truncate_visible(&text, width);
        lines.push(if selected {
            style::paint(&config.selected_style, &text)
        } else {
            text
        });
    }

    let mut hint = tf!(
        "{}/{total} · Tab next · ↑↓ select · Enter accept · Esc close",
        "{}/{total} · Tab 下一项 · ↑↓ 选择 · Enter 采纳 · Esc 关闭",
        menu.selected + 1
    );
    if !menu.title.is_empty() {
        hint = format!("{} · {hint}", menu.title);
    }
    lines.push(style::paint(
        &config.hint_style,
        &style::truncate_visible(&hint, width),
    ));
    lines
}

/// 负责把布局写到终端，并记录上一次占用的行数。
#[derive(Debug, Default)]
pub struct Renderer {
    rows: usize,
    cursor_row: usize,
}

impl Renderer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.rows = 0;
        self.cursor_row = 0;
    }

    pub fn draw<W: Write>(
        &mut self,
        out: &mut W,
        layout: &Layout,
        extra: &[String],
    ) -> std::io::Result<()> {
        if self.cursor_row > 0 {
            out.queue(MoveUp(self.cursor_row as u16))?;
        }
        out.queue(MoveToColumn(0))?;
        out.queue(Clear(ClearType::FromCursorDown))?;

        let total = layout.lines.len() + extra.len();
        for (index, line) in layout.lines.iter().chain(extra.iter()).enumerate() {
            if index > 0 {
                out.write_all(b"\r\n")?;
            }
            out.write_all(line.as_bytes())?;
        }

        let last_row = total.saturating_sub(1);
        let up = last_row.saturating_sub(layout.cursor_row);
        if up > 0 {
            out.queue(MoveUp(up as u16))?;
        }
        out.queue(MoveToColumn(layout.cursor_col as u16))?;
        out.flush()?;

        self.rows = total;
        self.cursor_row = layout.cursor_row;
        Ok(())
    }

    /// 输入结束：光标移到内容末尾并换行。
    pub fn finish<W: Write>(&mut self, out: &mut W, layout: &Layout) -> std::io::Result<()> {
        let down = layout
            .lines
            .len()
            .saturating_sub(1)
            .saturating_sub(layout.cursor_row);
        if down > 0 {
            out.queue(MoveDown(down as u16))?;
        }
        out.queue(MoveToColumn(0))?;
        out.write_all(b"\r\n")?;
        out.flush()?;
        self.reset();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::complete::{Candidate, CandidateKind};

    #[test]
    fn places_cursor_after_prompt() {
        let layout = layout(80, "❯ ", "ls -la", &[], "", "", 6);
        assert_eq!(layout.lines.len(), 1);
        assert_eq!(layout.cursor_row, 0);
        assert_eq!(layout.cursor_col, 2 + 6);
        assert!(layout.lines[0].ends_with("ls -la"));
    }

    #[test]
    fn wraps_long_input() {
        let buffer = "x".repeat(25);
        let layout = layout(10, "> ", &buffer, &[], "", "", buffer.len());
        assert_eq!(layout.lines.len(), 3);
        assert_eq!(layout.cursor_row, 2);
        assert_eq!(layout.cursor_col, 7);
    }

    #[test]
    fn handles_wide_characters() {
        let layout = layout(80, "", "中文ab", &[], "", "", "中文ab".len());
        assert_eq!(layout.cursor_col, 6);
    }

    #[test]
    fn ghost_does_not_move_cursor() {
        style::set_color_enabled(false);
        let layout = layout(80, "", "ls", &[], " -la", "dimmed", 2);
        assert_eq!(layout.cursor_col, 2);
        assert_eq!(layout.lines[0], "ls -la");
    }

    #[test]
    fn multiline_buffer_uses_new_rows() {
        let layout = layout(80, "> ", "a\nb", &[], "", "", 3);
        assert_eq!(layout.lines.len(), 2);
        assert_eq!(layout.cursor_row, 1);
        assert_eq!(layout.cursor_col, 1);
    }

    #[test]
    fn renders_menu_with_hint() {
        style::set_color_enabled(false);
        let items = vec![
            Candidate {
                value: "help".into(),
                display: "help".into(),
                description: "查看帮助".into(),
                kind: CandidateKind::Builtin,
                replace: 0..1,
            },
            Candidate {
                value: "history".into(),
                display: "history".into(),
                description: "历史".into(),
                kind: CandidateKind::Builtin,
                replace: 0..1,
            },
        ];
        let menu = Menu {
            items,
            selected: 1,
            offset: 0,
            title: String::new(),
        };
        let config = crate::config::Config::default();
        let lines = menu_lines(&menu, &config.menu, 60);
        assert_eq!(lines.len(), 3);
        assert!(lines[0].starts_with("  help"));
        assert!(lines[1].starts_with("▸ history"));
        assert!(lines[2].contains("2/2"));
    }
}

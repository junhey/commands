//! 交互式行编辑器。
//!
//! 核心体验（借鉴 fish）：
//! * 边输入边给出灰色历史建议，`→` / `End` / `Ctrl-F` 采纳，`Alt-→` 采纳一个词；
//! * `Tab` 弹出候选菜单（历史命令、内建、PATH 命令、路径、变量），`↑`/`↓` 选择；
//! * 语法高亮实时提示命令是否存在；
//! * `Ctrl-R` 历史模糊搜索，`↑`/`↓` 同样可选。

pub mod complete;
pub mod highlight;
pub mod render;
pub mod suggest;

use crate::config::Config;
use crate::prompt::Prompt;
use crate::shell::Shell;
use crate::style;
use complete::{Candidate, CandidateKind};
use crossterm::event::{
    self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyCode, KeyEvent, KeyEventKind,
    KeyModifiers,
};
use crossterm::terminal;
use crossterm::tty::IsTty;
use render::{Layout, Renderer};
use std::io::{self, BufRead, Stdout, Write};
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadOutcome {
    Line(String),
    /// Ctrl-C：放弃当前输入
    Interrupted,
    /// Ctrl-D 或输入结束
    Eof,
}

#[derive(Debug, Clone)]
pub struct Menu {
    pub items: Vec<Candidate>,
    pub selected: usize,
    pub offset: usize,
    pub title: String,
}

impl Menu {
    fn move_selection(&mut self, delta: isize, max_rows: usize) {
        if self.items.is_empty() {
            return;
        }
        let length = self.items.len() as isize;
        self.selected = (self.selected as isize + delta).rem_euclid(length) as usize;
        let rows = max_rows.max(1);
        if self.selected < self.offset {
            self.offset = self.selected;
        } else if self.selected >= self.offset + rows {
            self.offset = self.selected + 1 - rows;
        }
        if self.offset + rows > self.items.len() {
            self.offset = self.items.len().saturating_sub(rows);
        }
    }
}

/// 读取一行输入。非 TTY 环境（管道、重定向）退化为普通读取。
pub fn read_line(shell: &mut Shell, prompt: &Prompt) -> io::Result<ReadOutcome> {
    if !io::stdin().is_tty() {
        return read_line_plain();
    }
    Editor::new(shell).run(prompt)
}

fn read_line_plain() -> io::Result<ReadOutcome> {
    let mut line = String::new();
    if io::stdin().lock().read_line(&mut line)? == 0 {
        return Ok(ReadOutcome::Eof);
    }
    Ok(ReadOutcome::Line(
        line.trim_end_matches(['\r', '\n']).to_string(),
    ))
}

struct Editor<'a> {
    shell: &'a mut Shell,
    config: Rc<Config>,
    out: Stdout,
    buffer: String,
    cursor: usize,
    ghost: Option<String>,
    menu: Option<Menu>,
    search: Option<String>,
    message: Option<String>,
    history_index: Option<usize>,
    history_prefix: String,
    history_snapshot: String,
    renderer: Renderer,
}

impl<'a> Editor<'a> {
    fn new(shell: &'a mut Shell) -> Self {
        let config = Rc::clone(&shell.config);
        Self {
            shell,
            config,
            out: io::stdout(),
            buffer: String::new(),
            cursor: 0,
            ghost: None,
            menu: None,
            search: None,
            message: None,
            history_index: None,
            history_prefix: String::new(),
            history_snapshot: String::new(),
            renderer: Renderer::new(),
        }
    }

    fn run(&mut self, prompt: &Prompt) -> io::Result<ReadOutcome> {
        if !prompt.leading.is_empty() {
            write!(self.out, "{}", prompt.leading)?;
            self.out.flush()?;
        }
        terminal::enable_raw_mode()?;
        let _ = crossterm::execute!(self.out, EnableBracketedPaste);
        let outcome = self.event_loop(&prompt.last_line);
        let _ = crossterm::execute!(self.out, DisableBracketedPaste);
        let _ = terminal::disable_raw_mode();
        outcome
    }

    fn event_loop(&mut self, prompt: &str) -> io::Result<ReadOutcome> {
        loop {
            self.refresh_ghost();
            self.draw(prompt)?;
            match event::read()? {
                Event::Key(key) if key.kind != KeyEventKind::Release => {
                    self.message = None;
                    if let Some(outcome) = self.handle_key(key, prompt)? {
                        return Ok(outcome);
                    }
                }
                Event::Paste(text) => {
                    let sanitized = text.replace(['\r'], "");
                    self.buffer.insert_str(self.cursor, &sanitized);
                    self.cursor += sanitized.len();
                }
                _ => {}
            }
        }
    }

    // ── 绘制 ────────────────────────────────────────────────────────

    fn width(&self) -> usize {
        terminal::size()
            .map(|(columns, _)| columns as usize)
            .unwrap_or(80)
            .max(20)
    }

    fn build_layout(&mut self, prompt: &str) -> Layout {
        let width = self.width();
        let spans = highlight::spans(self.shell, &self.buffer, &self.config);
        let ghost_tail = self
            .ghost
            .as_ref()
            .and_then(|full| full.get(self.buffer.len()..))
            .unwrap_or("")
            .to_string();
        render::layout(
            width,
            prompt,
            &self.buffer,
            &spans,
            &ghost_tail,
            &self.config.autosuggest.style,
            self.cursor,
        )
    }

    fn draw(&mut self, prompt: &str) -> io::Result<()> {
        let layout = self.build_layout(prompt);
        let width = self.width();
        let mut extra: Vec<String> = Vec::new();
        if let Some(menu) = &self.menu {
            if !menu.items.is_empty() {
                extra = render::menu_lines(menu, &self.config.menu, width);
            }
        }
        if extra.is_empty() {
            if let Some(message) = &self.message {
                extra.push(style::paint(
                    &self.config.menu.hint_style,
                    &style::truncate_visible(message, width),
                ));
            }
        }
        let Self { renderer, out, .. } = self;
        renderer.draw(out, &layout, &extra)
    }

    fn close_line(&mut self, prompt: &str) -> io::Result<()> {
        self.menu = None;
        self.search = None;
        self.ghost = None;
        self.message = None;
        self.draw(prompt)?;
        let layout = self.build_layout(prompt);
        let Self { renderer, out, .. } = self;
        renderer.finish(out, &layout)
    }

    // ── 建议 / 菜单 ─────────────────────────────────────────────────

    fn refresh_ghost(&mut self) {
        self.ghost = None;
        if self.menu.is_some() || self.search.is_some() {
            return;
        }
        if self.cursor != self.buffer.len() {
            return;
        }
        let config = Rc::clone(&self.config);
        self.ghost = suggest::autosuggestion(self.shell, &self.buffer, &config);
    }

    fn open_menu(&mut self) {
        if !self.config.menu.enabled {
            return;
        }
        let config = Rc::clone(&self.config);
        let candidates = complete::complete(self.shell, &self.buffer, self.cursor, &config);
        if candidates.is_empty() {
            self.message = Some("没有可用候选".to_string());
            return;
        }
        if candidates.len() == 1 {
            let only = candidates[0].clone();
            self.accept_candidate(&only);
            return;
        }
        if config.menu.auto_insert_common_prefix {
            if let Some((range, prefix)) = complete::common_prefix(&candidates) {
                let current = self.buffer[range.clone()].to_string();
                if prefix.len() > current.len() && prefix.starts_with(&current) {
                    self.buffer.replace_range(range.clone(), &prefix);
                    self.cursor = range.start + prefix.len();
                    let refreshed =
                        complete::complete(self.shell, &self.buffer, self.cursor, &config);
                    if refreshed.len() == 1 {
                        let only = refreshed[0].clone();
                        self.accept_candidate(&only);
                        return;
                    }
                    if !refreshed.is_empty() {
                        self.menu = Some(Menu {
                            items: refreshed,
                            selected: 0,
                            offset: 0,
                            title: String::new(),
                        });
                        return;
                    }
                }
            }
        }
        self.menu = Some(Menu {
            items: candidates,
            selected: 0,
            offset: 0,
            title: String::new(),
        });
    }

    fn refresh_menu(&mut self) {
        if self.search.is_some() {
            self.refresh_search();
            return;
        }
        if self.menu.is_none() {
            return;
        }
        let config = Rc::clone(&self.config);
        let candidates = complete::complete(self.shell, &self.buffer, self.cursor, &config);
        match self.menu.as_mut() {
            Some(menu) if !candidates.is_empty() => {
                menu.items = candidates;
                menu.selected = 0;
                menu.offset = 0;
            }
            _ => self.menu = None,
        }
    }

    fn accept_candidate(&mut self, candidate: &Candidate) {
        let mut value = candidate.value.clone();
        value.push_str(candidate.kind.suffix());
        let start = candidate.replace.start.min(self.buffer.len());
        let end = candidate.replace.end.min(self.buffer.len()).max(start);
        self.buffer.replace_range(start..end, &value);
        self.cursor = start + value.len();
        self.menu = None;
        self.search = None;
        self.history_index = None;
    }

    fn accept_selected(&mut self) {
        let Some(menu) = self.menu.take() else {
            return;
        };
        if let Some(candidate) = menu.items.get(menu.selected).cloned() {
            self.accept_candidate(&candidate);
        }
    }

    fn accept_suggestion(&mut self) {
        if let Some(full) = self.ghost.take() {
            self.buffer = full;
            self.cursor = self.buffer.len();
        } else {
            self.cursor = self.buffer.len();
        }
    }

    fn accept_suggestion_word(&mut self) {
        let Some(full) = self.ghost.clone() else {
            self.move_word_right();
            return;
        };
        let Some(tail) = full.get(self.buffer.len()..) else {
            return;
        };
        let take = suggest::next_word_boundary(tail);
        let addition = tail[..take].to_string();
        self.buffer.push_str(&addition);
        self.cursor = self.buffer.len();
    }

    // ── 历史 ────────────────────────────────────────────────────────

    fn history_matches(&self) -> Vec<String> {
        self.shell
            .history
            .prefix_matches(&self.history_prefix, 300)
            .into_iter()
            .map(str::to_string)
            .collect()
    }

    fn history_previous(&mut self) {
        if self.history_index.is_none() {
            self.history_prefix = self.buffer[..self.cursor].to_string();
            self.history_snapshot = self.buffer.clone();
        }
        let matches = self.history_matches();
        if matches.is_empty() {
            self.message = Some("没有匹配的历史记录".to_string());
            return;
        }
        let next = match self.history_index {
            Some(index) => (index + 1).min(matches.len() - 1),
            None => 0,
        };
        self.history_index = Some(next);
        self.set_buffer(matches[next].clone());
    }

    fn history_next(&mut self) {
        let Some(index) = self.history_index else {
            return;
        };
        if index == 0 {
            self.history_index = None;
            let snapshot = self.history_snapshot.clone();
            self.set_buffer(snapshot);
            return;
        }
        let matches = self.history_matches();
        let next = index - 1;
        if next < matches.len() {
            self.history_index = Some(next);
            self.set_buffer(matches[next].clone());
        }
    }

    fn set_buffer(&mut self, text: String) {
        self.buffer = text;
        self.cursor = self.buffer.len();
        self.menu = None;
    }

    // ── Ctrl-R 搜索 ─────────────────────────────────────────────────

    fn start_search(&mut self) {
        self.search = Some(String::new());
        self.refresh_search();
    }

    fn refresh_search(&mut self) {
        let term = self.search.clone().unwrap_or_default();
        let replace = 0..self.buffer.len();
        let items: Vec<Candidate> = self
            .shell
            .history
            .ranked_matches(&term, 60)
            .into_iter()
            .map(|command| Candidate {
                value: command.to_string(),
                display: command.to_string(),
                description: "回车即执行".to_string(),
                kind: CandidateKind::History,
                replace: replace.clone(),
            })
            .collect();
        let empty = items.is_empty();
        let title = format!("搜索历史「{term}」");
        match self.menu.as_mut() {
            Some(menu) => {
                menu.items = items;
                menu.selected = 0;
                menu.offset = 0;
                menu.title = title;
            }
            None => {
                self.menu = Some(Menu {
                    items,
                    selected: 0,
                    offset: 0,
                    title,
                })
            }
        }
        if empty {
            self.message = Some(format!("没有匹配「{term}」的历史"));
        }
    }

    // ── 编辑动作 ────────────────────────────────────────────────────

    fn insert_char(&mut self, c: char) {
        self.buffer.insert(self.cursor, c);
        self.cursor += c.len_utf8();
        self.history_index = None;
        if c == ' ' {
            self.expand_abbreviation();
        }
        if self.menu.is_some() {
            self.refresh_menu();
        }
    }

    /// fish 式缩写：输入 `gco` + 空格 -> `git checkout `。
    fn expand_abbreviation(&mut self) {
        let space_at = self.cursor.saturating_sub(1);
        let head = &self.buffer[..space_at];
        let start = head
            .rfind(|c: char| c.is_whitespace())
            .map(|index| index + 1)
            .unwrap_or(0);
        let word = head[start..].to_string();
        if word.is_empty() {
            return;
        }
        let Some(expansion) = self.shell.abbreviations.get(&word).cloned() else {
            return;
        };
        self.buffer.replace_range(start..space_at, &expansion);
        self.cursor = self.cursor + expansion.len() - word.len();
    }

    fn move_left(&mut self) {
        if let Some((index, _)) = self.buffer[..self.cursor].char_indices().next_back() {
            self.cursor = index;
        }
    }

    fn move_right(&mut self) {
        if let Some(c) = self.buffer[self.cursor..].chars().next() {
            self.cursor += c.len_utf8();
        }
    }

    fn word_start(&self) -> usize {
        let head = self.buffer[..self.cursor].trim_end();
        head.rfind(|c: char| c.is_whitespace())
            .map(|index| index + 1)
            .unwrap_or(0)
    }

    fn move_word_left(&mut self) {
        self.cursor = self.word_start();
    }

    fn move_word_right(&mut self) {
        let tail = &self.buffer[self.cursor..];
        let skip: usize = tail
            .chars()
            .take_while(|c| c.is_whitespace())
            .map(char::len_utf8)
            .sum();
        let rest = &tail[skip..];
        let length: usize = rest
            .chars()
            .take_while(|c| !c.is_whitespace())
            .map(char::len_utf8)
            .sum();
        self.cursor = (self.cursor + skip + length).min(self.buffer.len());
    }

    fn delete_backward(&mut self) {
        if let Some((index, _)) = self.buffer[..self.cursor].char_indices().next_back() {
            self.buffer.remove(index);
            self.cursor = index;
            self.history_index = None;
            if self.menu.is_some() {
                self.refresh_menu();
            }
        }
    }

    fn delete_forward(&mut self) {
        if self.cursor < self.buffer.len() {
            self.buffer.remove(self.cursor);
            if self.menu.is_some() {
                self.refresh_menu();
            }
        }
    }

    fn delete_word_backward(&mut self) {
        let start = self.word_start();
        if start < self.cursor {
            self.buffer.drain(start..self.cursor);
            self.cursor = start;
        }
    }

    fn clear_screen(&mut self) -> io::Result<()> {
        self.out.write_all(b"\x1b[2J\x1b[3J\x1b[H")?;
        self.renderer.reset();
        self.out.flush()
    }

    // ── 按键分发 ────────────────────────────────────────────────────

    fn handle_key(&mut self, key: KeyEvent, prompt: &str) -> io::Result<Option<ReadOutcome>> {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let alt = key.modifiers.contains(KeyModifiers::ALT);
        let rows = self.config.menu.max_rows;

        match (key.code, ctrl, alt) {
            (KeyCode::Char('c'), true, _) => {
                self.close_line(prompt)?;
                return Ok(Some(ReadOutcome::Interrupted));
            }
            (KeyCode::Char('d'), true, _) => {
                if self.buffer.is_empty() {
                    self.close_line(prompt)?;
                    return Ok(Some(ReadOutcome::Eof));
                }
                self.delete_forward();
            }
            (KeyCode::Enter, ..) => {
                if self.menu.is_some() {
                    self.accept_selected();
                    return Ok(None);
                }
                self.close_line(prompt)?;
                return Ok(Some(ReadOutcome::Line(std::mem::take(&mut self.buffer))));
            }
            (KeyCode::Tab, ..) => match self.menu.as_mut() {
                Some(menu) => menu.move_selection(1, rows),
                None => self.open_menu(),
            },
            (KeyCode::BackTab, ..) => {
                if let Some(menu) = self.menu.as_mut() {
                    menu.move_selection(-1, rows);
                }
            }
            (KeyCode::Up, ..) | (KeyCode::Char('p'), true, _) => match self.menu.as_mut() {
                Some(menu) => menu.move_selection(-1, rows),
                None => self.history_previous(),
            },
            (KeyCode::Down, ..) | (KeyCode::Char('n'), true, _) => match self.menu.as_mut() {
                Some(menu) => menu.move_selection(1, rows),
                None => self.history_next(),
            },
            (KeyCode::Esc, ..) => {
                self.menu = None;
                self.search = None;
                self.ghost = None;
            }
            (KeyCode::Left, _, true) | (KeyCode::Char('b'), false, true) => self.move_word_left(),
            (KeyCode::Right, _, true) | (KeyCode::Char('f'), false, true) => {
                self.accept_suggestion_word()
            }
            (KeyCode::Left, ..) => self.move_left(),
            (KeyCode::Right, ..) => {
                if self.cursor == self.buffer.len() && self.ghost.is_some() {
                    self.accept_suggestion();
                } else {
                    self.move_right();
                }
            }
            (KeyCode::Home, ..) | (KeyCode::Char('a'), true, _) => self.cursor = 0,
            (KeyCode::End, ..) | (KeyCode::Char('e'), true, _) | (KeyCode::Char('f'), true, _) => {
                self.accept_suggestion()
            }
            (KeyCode::Backspace, ..) => {
                if self.search.is_some() {
                    if let Some(term) = self.search.as_mut() {
                        term.pop();
                    }
                    self.refresh_search();
                } else {
                    self.delete_backward();
                }
            }
            (KeyCode::Delete, ..) => self.delete_forward(),
            (KeyCode::Char('w'), true, _) => self.delete_word_backward(),
            (KeyCode::Char('u'), true, _) => {
                self.buffer.drain(..self.cursor);
                self.cursor = 0;
            }
            (KeyCode::Char('k'), true, _) => self.buffer.truncate(self.cursor),
            (KeyCode::Char('l'), true, _) => self.clear_screen()?,
            (KeyCode::Char('r'), true, _) => self.start_search(),
            (KeyCode::Char(c), false, false) => {
                if self.search.is_some() {
                    if let Some(term) = self.search.as_mut() {
                        term.push(c);
                    }
                    self.refresh_search();
                } else {
                    self.insert_char(c);
                }
            }
            _ => {}
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn editor(shell: &mut Shell) -> Editor<'_> {
        Editor::new(shell)
    }

    #[test]
    fn inserts_and_deletes_characters() {
        let mut shell = Shell::new(Config::default(), false);
        let mut editor = editor(&mut shell);
        for c in "ls -l".chars() {
            editor.insert_char(c);
        }
        assert_eq!(editor.buffer, "ls -l");
        assert_eq!(editor.cursor, 5);
        editor.delete_backward();
        assert_eq!(editor.buffer, "ls -");
        editor.delete_word_backward();
        assert_eq!(editor.buffer, "ls ");
    }

    #[test]
    fn expands_abbreviation_on_space() {
        let mut shell = Shell::new(Config::default(), false);
        shell
            .abbreviations
            .insert("gco".to_string(), "git checkout".to_string());
        let mut editor = editor(&mut shell);
        for c in "gco ".chars() {
            editor.insert_char(c);
        }
        assert_eq!(editor.buffer, "git checkout ");
        assert_eq!(editor.cursor, editor.buffer.len());
    }

    #[test]
    fn accepts_history_suggestion() {
        let mut shell = Shell::new(Config::default(), false);
        shell.history.record("cargo build --release");
        let mut editor = editor(&mut shell);
        for c in "cargo b".chars() {
            editor.insert_char(c);
        }
        editor.refresh_ghost();
        assert_eq!(editor.ghost.as_deref(), Some("cargo build --release"));
        editor.accept_suggestion();
        assert_eq!(editor.buffer, "cargo build --release");
    }

    #[test]
    fn accepts_suggestion_word_by_word() {
        let mut shell = Shell::new(Config::default(), false);
        shell.history.record("git commit --amend");
        let mut editor = editor(&mut shell);
        for c in "git".chars() {
            editor.insert_char(c);
        }
        editor.refresh_ghost();
        editor.accept_suggestion_word();
        assert_eq!(editor.buffer, "git commit");
    }

    #[test]
    fn tab_menu_navigates_and_accepts() {
        let mut shell = Shell::new(Config::default(), false);
        shell.history.record("history 10");
        let mut editor = editor(&mut shell);
        for c in "hi".chars() {
            editor.insert_char(c);
        }
        editor.open_menu();
        let menu = editor.menu.as_ref().expect("菜单应打开");
        assert!(menu.items.len() > 1);
        let rows = editor.config.menu.max_rows;
        editor.menu.as_mut().unwrap().move_selection(1, rows);
        let expected = editor.menu.as_ref().unwrap().items[1].clone();
        editor.accept_selected();
        assert!(editor.menu.is_none());
        assert!(editor.buffer.starts_with(expected.value.trim_end()));
    }

    #[test]
    fn single_candidate_is_inserted_directly() {
        let mut shell = Shell::new(Config::default(), false);
        let mut editor = editor(&mut shell);
        for c in "unab".chars() {
            editor.insert_char(c);
        }
        editor.open_menu();
        assert!(editor.menu.is_none());
        assert_eq!(editor.buffer, "unabbr ");
    }

    #[test]
    fn history_navigation_walks_entries() {
        let mut shell = Shell::new(Config::default(), false);
        shell.history.record("first command");
        shell.history.record("second command");
        let mut editor = editor(&mut shell);
        editor.history_previous();
        assert_eq!(editor.buffer, "second command");
        editor.history_previous();
        assert_eq!(editor.buffer, "first command");
        editor.history_next();
        assert_eq!(editor.buffer, "second command");
        editor.history_next();
        assert_eq!(editor.buffer, "");
    }

    #[test]
    fn history_navigation_respects_prefix() {
        let mut shell = Shell::new(Config::default(), false);
        shell.history.record("git status");
        shell.history.record("ls -la");
        shell.history.record("git diff");
        let mut editor = editor(&mut shell);
        for c in "git".chars() {
            editor.insert_char(c);
        }
        editor.history_previous();
        assert_eq!(editor.buffer, "git diff");
        editor.history_previous();
        assert_eq!(editor.buffer, "git status");
    }

    #[test]
    fn search_mode_lists_matches() {
        let mut shell = Shell::new(Config::default(), false);
        shell.history.record("cargo test -- --nocapture");
        shell.history.record("ls -la");
        let mut editor = editor(&mut shell);
        editor.start_search();
        for c in "nocap".chars() {
            if let Some(term) = editor.search.as_mut() {
                term.push(c);
            }
        }
        editor.refresh_search();
        let menu = editor.menu.as_ref().expect("搜索菜单");
        assert_eq!(menu.items.len(), 1);
        assert_eq!(menu.items[0].value, "cargo test -- --nocapture");
        editor.accept_selected();
        assert_eq!(editor.buffer, "cargo test -- --nocapture");
    }

    #[test]
    fn menu_selection_wraps_and_scrolls() {
        let items: Vec<Candidate> = (0..5)
            .map(|index| Candidate {
                value: format!("item{index}"),
                display: format!("item{index}"),
                description: String::new(),
                kind: CandidateKind::Command,
                replace: 0..0,
            })
            .collect();
        let mut menu = Menu {
            items,
            selected: 0,
            offset: 0,
            title: String::new(),
        };
        menu.move_selection(-1, 2);
        assert_eq!(menu.selected, 4);
        assert_eq!(menu.offset, 3);
        menu.move_selection(1, 2);
        assert_eq!(menu.selected, 0);
        assert_eq!(menu.offset, 0);
    }
}

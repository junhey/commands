//! 命令行词法/语法分析：引号、变量展开、管道、重定向、逻辑连接符。
//!
//! 与 fish 一致，变量展开后**不再做二次分词**，`$dir` 里的空格不会把一个参数拆开。

use crate::util;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fd {
    Out,
    Err,
    Both,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Word {
    pub text: String,
    /// 未被引号包裹且含通配符，执行时尝试 glob 展开。
    pub globbable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Word(Word),
    Pipe,
    And,
    Or,
    Semi,
    Amp,
    RedirOut { fd: Fd, append: bool },
    RedirIn,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxError {
    pub message: String,
    /// 语句尚未结束（引号未闭合、以 `|` 结尾等），交互式下应继续读取下一行。
    pub incomplete: bool,
}

impl SyntaxError {
    fn new(message: impl Into<String>, incomplete: bool) -> Self {
        Self {
            message: message.into(),
            incomplete,
        }
    }
}

impl std::fmt::Display for SyntaxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainOp {
    And,
    Or,
    Seq,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedirectKind {
    Out { fd: Fd, append: bool },
    In,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Redirect {
    pub kind: RedirectKind,
    pub target: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Command {
    pub words: Vec<Word>,
    pub redirects: Vec<Redirect>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pipeline {
    pub commands: Vec<Command>,
    pub background: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    Pipeline(Pipeline),
    Chain {
        left: Box<Node>,
        op: ChainOp,
        right: Box<Node>,
    },
}

/// 变量查询回调：返回 None 表示变量不存在（展开为空）。
pub type VarLookup<'a> = &'a dyn Fn(&str) -> Option<String>;

#[derive(Default)]
struct WordBuilder {
    text: String,
    globbable: bool,
    started: bool,
}

impl WordBuilder {
    fn push(&mut self, c: char) {
        self.started = true;
        if matches!(c, '*' | '?' | '[') {
            self.globbable = true;
        }
        self.text.push(c);
    }

    fn push_quoted(&mut self, text: &str) {
        self.started = true;
        self.text.push_str(text);
    }

    fn take(&mut self) -> Option<Word> {
        if !self.started {
            return None;
        }
        let word = Word {
            text: std::mem::take(&mut self.text),
            globbable: self.globbable,
        };
        self.globbable = false;
        self.started = false;
        Some(word)
    }
}

/// 把输入切成 token。
pub fn tokenize(input: &str, vars: VarLookup<'_>) -> Result<Vec<Token>, SyntaxError> {
    let chars: Vec<char> = input.chars().collect();
    let mut tokens: Vec<Token> = Vec::new();
    let mut builder = WordBuilder::default();
    let mut index = 0usize;

    macro_rules! flush {
        () => {
            if let Some(word) = builder.take() {
                tokens.push(Token::Word(word));
            }
        };
    }

    while index < chars.len() {
        let c = chars[index];
        let next = chars.get(index + 1).copied();
        match c {
            // BOM 当作空白，避免带 BOM 的脚本第一条命令解析失败
            ' ' | '\t' | '\r' | '\n' | '\u{feff}' => {
                flush!();
                index += 1;
            }
            '#' if !builder.started => break,
            '|' => {
                flush!();
                if next == Some('|') {
                    tokens.push(Token::Or);
                    index += 2;
                } else {
                    tokens.push(Token::Pipe);
                    index += 1;
                }
            }
            '&' => {
                flush!();
                match next {
                    Some('&') => {
                        tokens.push(Token::And);
                        index += 2;
                    }
                    Some('>') => {
                        let append = chars.get(index + 2) == Some(&'>');
                        tokens.push(Token::RedirOut {
                            fd: Fd::Both,
                            append,
                        });
                        index += if append { 3 } else { 2 };
                    }
                    _ => {
                        tokens.push(Token::Amp);
                        index += 1;
                    }
                }
            }
            ';' => {
                flush!();
                tokens.push(Token::Semi);
                index += 1;
            }
            '<' => {
                flush!();
                tokens.push(Token::RedirIn);
                index += 1;
            }
            '>' => {
                flush!();
                let append = next == Some('>');
                tokens.push(Token::RedirOut {
                    fd: Fd::Out,
                    append,
                });
                index += if append { 2 } else { 1 };
            }
            '1' | '2' if !builder.started && next == Some('>') => {
                let fd = if c == '1' { Fd::Out } else { Fd::Err };
                let append = chars.get(index + 2) == Some(&'>');
                tokens.push(Token::RedirOut { fd, append });
                index += if append { 3 } else { 2 };
            }
            '\'' => {
                let mut end = index + 1;
                let mut literal = String::new();
                loop {
                    match chars.get(end) {
                        Some('\'') => break,
                        Some(c) => {
                            literal.push(*c);
                            end += 1;
                        }
                        None => {
                            return Err(SyntaxError::new(
                                t!("unterminated single quote", "单引号未闭合"),
                                true,
                            ));
                        }
                    }
                }
                builder.push_quoted(&literal);
                index = end + 1;
            }
            '"' => {
                let mut end = index + 1;
                let mut literal = String::new();
                loop {
                    match chars.get(end) {
                        Some('"') => break,
                        Some('\\') => {
                            let escaped = chars.get(end + 1).copied();
                            match escaped {
                                Some(e @ ('"' | '\\' | '$' | '`')) => {
                                    literal.push(e);
                                    end += 2;
                                }
                                Some('n') => {
                                    literal.push('\n');
                                    end += 2;
                                }
                                Some('t') => {
                                    literal.push('\t');
                                    end += 2;
                                }
                                Some(other) => {
                                    literal.push('\\');
                                    literal.push(other);
                                    end += 2;
                                }
                                None => {
                                    return Err(SyntaxError::new(
                                        t!("unterminated double quote", "双引号未闭合"),
                                        true,
                                    ));
                                }
                            }
                        }
                        Some('$') => {
                            let mut cursor = end;
                            literal.push_str(&read_variable(&chars, &mut cursor, vars));
                            end = cursor;
                        }
                        Some(c) => {
                            literal.push(*c);
                            end += 1;
                        }
                        None => {
                            return Err(SyntaxError::new(
                                t!("unterminated double quote", "双引号未闭合"),
                                true,
                            ));
                        }
                    }
                }
                builder.push_quoted(&literal);
                index = end + 1;
            }
            '$' => {
                let mut cursor = index;
                let expanded = read_variable(&chars, &mut cursor, vars);
                builder.push_quoted(&expanded);
                index = cursor;
            }
            '~' if !builder.started
                && matches!(next, None | Some('/') | Some('\\') | Some(' ') | Some('\t')) =>
            {
                builder.push_quoted(&util::expand_tilde("~"));
                index += 1;
            }
            '\\' => match next {
                // 行尾反斜杠：续行
                None => {
                    return Err(SyntaxError::new(
                        t!(
                            "trailing backslash (line continuation)",
                            "行尾反斜杠（续行）"
                        ),
                        true,
                    ));
                }
                Some('\n') => index += 2,
                Some('\r') => {
                    index += if chars.get(index + 2) == Some(&'\n') {
                        3
                    } else {
                        2
                    }
                }
                Some(escaped) => {
                    // Windows 下反斜杠通常是路径分隔符，只在少数情况下当转义符
                    let keep = cfg!(windows) && !matches!(escaped, ' ' | '"' | '\'' | '\\');
                    if keep {
                        builder.push('\\');
                        index += 1;
                    } else {
                        builder.push_quoted(&escaped.to_string());
                        index += 2;
                    }
                }
            },
            c => {
                builder.push(c);
                index += 1;
            }
        }
    }

    flush!();
    Ok(tokens)
}

/// 读取 `$VAR` / `${VAR}` / `$?` 等，cursor 指向 `$`，返回展开结果并前移 cursor。
fn read_variable(chars: &[char], cursor: &mut usize, vars: VarLookup<'_>) -> String {
    let start = *cursor;
    debug_assert_eq!(chars.get(start), Some(&'$'));
    let mut index = start + 1;
    let name = match chars.get(index) {
        Some('{') => {
            index += 1;
            let mut name = String::new();
            while let Some(c) = chars.get(index) {
                if *c == '}' {
                    index += 1;
                    break;
                }
                name.push(*c);
                index += 1;
            }
            name
        }
        Some(c @ ('?' | '$' | '#')) => {
            let name = c.to_string();
            index += 1;
            name
        }
        _ => {
            let mut name = String::new();
            while let Some(c) = chars.get(index) {
                if c.is_alphanumeric() || *c == '_' {
                    name.push(*c);
                    index += 1;
                } else {
                    break;
                }
            }
            name
        }
    };
    *cursor = index;
    if name.is_empty() {
        return "$".to_string();
    }
    vars(&name).unwrap_or_default()
}

struct Parser<'a> {
    tokens: &'a [Token],
    position: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<&'a Token> {
        self.tokens.get(self.position)
    }

    fn parse_node(&mut self) -> Result<Option<Node>, SyntaxError> {
        let Some(first) = self.parse_pipeline()? else {
            return Ok(None);
        };
        let mut left = Node::Pipeline(first);
        loop {
            let op = match self.peek() {
                Some(Token::And) => ChainOp::And,
                Some(Token::Or) => ChainOp::Or,
                Some(Token::Semi) => ChainOp::Seq,
                _ => break,
            };
            self.position += 1;
            match self.parse_pipeline()? {
                Some(right) => {
                    left = Node::Chain {
                        left: Box::new(left),
                        op,
                        right: Box::new(Node::Pipeline(right)),
                    };
                }
                None => {
                    if op == ChainOp::Seq {
                        break;
                    }
                    return Err(SyntaxError::new(
                        t!(
                            "missing command after a logical operator",
                            "逻辑操作符后缺少命令"
                        ),
                        true,
                    ));
                }
            }
        }
        Ok(Some(left))
    }

    fn parse_pipeline(&mut self) -> Result<Option<Pipeline>, SyntaxError> {
        let mut commands: Vec<Command> = Vec::new();
        let mut background = false;
        loop {
            match self.parse_command()? {
                Some(command) => commands.push(command),
                None => {
                    if commands.is_empty() {
                        return Ok(None);
                    }
                    return Err(SyntaxError::new(
                        t!("missing command after a pipe", "管道后缺少命令"),
                        true,
                    ));
                }
            }
            match self.peek() {
                Some(Token::Pipe) => {
                    self.position += 1;
                }
                Some(Token::Amp) => {
                    self.position += 1;
                    background = true;
                    break;
                }
                _ => break,
            }
        }
        Ok(Some(Pipeline {
            commands,
            background,
        }))
    }

    fn parse_command(&mut self) -> Result<Option<Command>, SyntaxError> {
        let mut command = Command::default();
        loop {
            match self.peek() {
                Some(Token::Word(word)) => {
                    command.words.push(word.clone());
                    self.position += 1;
                }
                Some(Token::RedirOut { fd, append }) => {
                    let kind = RedirectKind::Out {
                        fd: *fd,
                        append: *append,
                    };
                    self.position += 1;
                    let target = self.expect_target(t!("a redirection target", "重定向目标"))?;
                    command.redirects.push(Redirect { kind, target });
                }
                Some(Token::RedirIn) => {
                    self.position += 1;
                    let target =
                        self.expect_target(t!("an input redirection target", "输入重定向目标"))?;
                    command.redirects.push(Redirect {
                        kind: RedirectKind::In,
                        target,
                    });
                }
                _ => break,
            }
        }
        if command.words.is_empty() && command.redirects.is_empty() {
            Ok(None)
        } else {
            Ok(Some(command))
        }
    }

    fn expect_target(&mut self, what: &str) -> Result<String, SyntaxError> {
        match self.peek() {
            Some(Token::Word(word)) => {
                let text = word.text.clone();
                self.position += 1;
                Ok(text)
            }
            _ => Err(SyntaxError::new(tf!("missing {what}", "缺少{what}"), true)),
        }
    }
}

/// 语法分析，空命令返回 `Ok(None)`。
pub fn parse(tokens: &[Token]) -> Result<Option<Node>, SyntaxError> {
    let mut parser = Parser {
        tokens,
        position: 0,
    };
    let node = parser.parse_node()?;
    if parser.position < tokens.len() {
        return Err(SyntaxError::new(
            tf!(
                "cannot parse the input (near token {})",
                "无法解析的输入（第 {} 个 token 附近）",
                parser.position + 1
            ),
            false,
        ));
    }
    Ok(node)
}

/// 便捷入口：词法 + 语法。
pub fn parse_line(input: &str, vars: VarLookup<'_>) -> Result<Option<Node>, SyntaxError> {
    let tokens = tokenize(input, vars)?;
    parse(&tokens)
}

/// 交互式多行输入判断：语句是否还没写完。
pub fn needs_continuation(input: &str) -> bool {
    let no_vars = |_: &str| None;
    match parse_line(input, &no_vars) {
        Err(error) => error.incomplete,
        Ok(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars(name: &str) -> Option<String> {
        match name {
            "HOME" => Some("/home/nova".to_string()),
            "EMPTY" => Some(String::new()),
            "?" => Some("0".to_string()),
            _ => None,
        }
    }

    fn lex(input: &str) -> Vec<Token> {
        tokenize(input, &vars).expect("词法分析失败")
    }

    fn words(input: &str) -> Vec<String> {
        lex(input)
            .into_iter()
            .filter_map(|t| match t {
                Token::Word(w) => Some(w.text),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn splits_words_and_quotes() {
        assert_eq!(words("echo hello world"), vec!["echo", "hello", "world"]);
        assert_eq!(
            words(r#"echo "two  words" 'raw $HOME'"#),
            vec!["echo", "two  words", "raw $HOME"]
        );
    }

    #[test]
    fn expands_variables_without_resplitting() {
        assert_eq!(words("echo $HOME/src"), vec!["echo", "/home/nova/src"]);
        assert_eq!(words("echo \"$HOME\""), vec!["echo", "/home/nova"]);
        assert_eq!(words("echo $MISSING end"), vec!["echo", "", "end"]);
        assert_eq!(words("echo $?"), vec!["echo", "0"]);
    }

    #[test]
    fn lexes_operators() {
        let tokens = lex("a | b && c || d ; e &");
        assert!(tokens.contains(&Token::Pipe));
        assert!(tokens.contains(&Token::And));
        assert!(tokens.contains(&Token::Or));
        assert!(tokens.contains(&Token::Semi));
        assert!(tokens.contains(&Token::Amp));
    }

    #[test]
    fn lexes_redirections() {
        let tokens = lex("cmd > out.txt 2>> err.log < in.txt &> all.log");
        assert!(tokens.contains(&Token::RedirOut {
            fd: Fd::Out,
            append: false
        }));
        assert!(tokens.contains(&Token::RedirOut {
            fd: Fd::Err,
            append: true
        }));
        assert!(tokens.contains(&Token::RedirIn));
        assert!(tokens.contains(&Token::RedirOut {
            fd: Fd::Both,
            append: false
        }));
    }

    #[test]
    fn marks_globbable_words() {
        let tokens = lex("ls *.rs \"*.rs\"");
        let globbable: Vec<bool> = tokens
            .iter()
            .filter_map(|t| match t {
                Token::Word(w) => Some(w.globbable),
                _ => None,
            })
            .collect();
        assert_eq!(globbable, vec![false, true, false]);
    }

    #[test]
    fn detects_incomplete_input() {
        assert!(needs_continuation("echo \"unterminated"));
        assert!(needs_continuation("ls |"));
        assert!(needs_continuation("true &&"));
        assert!(!needs_continuation("ls -la"));
        assert!(!needs_continuation("ls;"));
    }

    #[test]
    fn builds_pipeline_and_chain() {
        let node = parse_line("ls -la | grep rs && echo ok", &vars)
            .unwrap()
            .unwrap();
        match node {
            Node::Chain { left, op, right } => {
                assert_eq!(op, ChainOp::And);
                match *left {
                    Node::Pipeline(p) => {
                        assert_eq!(p.commands.len(), 2);
                        assert_eq!(p.commands[0].words[0].text, "ls");
                        assert_eq!(p.commands[1].words[0].text, "grep");
                    }
                    _ => panic!("左侧应为管道"),
                }
                match *right {
                    Node::Pipeline(p) => assert_eq!(p.commands[0].words[0].text, "echo"),
                    _ => panic!("右侧应为管道"),
                }
            }
            _ => panic!("应解析为链"),
        }
    }

    #[test]
    fn parses_background_and_redirects() {
        let node = parse_line("sleep 1 > log.txt &", &vars).unwrap().unwrap();
        match node {
            Node::Pipeline(p) => {
                assert!(p.background);
                assert_eq!(p.commands[0].redirects.len(), 1);
                assert_eq!(p.commands[0].redirects[0].target, "log.txt");
            }
            _ => panic!("应为管道"),
        }
    }

    #[test]
    fn ignores_comments() {
        assert_eq!(words("echo hi # 这是注释"), vec!["echo", "hi"]);
        assert_eq!(words("# 整行注释").len(), 0);
    }

    #[test]
    fn empty_input_is_none() {
        assert!(parse_line("   ", &vars).unwrap().is_none());
    }
}

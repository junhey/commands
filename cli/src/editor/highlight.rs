//! 输入行语法高亮（fish 风味：命令是否存在用颜色区分）。

use crate::config::Config;
use crate::shell::Shell;
use std::ops::Range;

/// 返回按字节范围划分的样式段。
pub fn spans(shell: &mut Shell, line: &str, config: &Config) -> Vec<(Range<usize>, String)> {
    let theme = &config.highlight;
    if !theme.enabled || line.is_empty() {
        return Vec::new();
    }
    let mut spans: Vec<(Range<usize>, String)> = Vec::new();
    let mut index = 0usize;
    let mut expect_command = true;

    while index < line.len() {
        let rest = &line[index..];
        let Some(ch) = rest.chars().next() else { break };

        if ch.is_whitespace() {
            index += ch.len_utf8();
            continue;
        }

        match ch {
            '#' => {
                spans.push((index..line.len(), theme.comment.clone()));
                break;
            }
            '|' | '&' | ';' | '<' | '>' => {
                let length = rest
                    .chars()
                    .take_while(|c| matches!(c, '|' | '&' | ';' | '<' | '>'))
                    .map(char::len_utf8)
                    .sum::<usize>();
                spans.push((index..index + length, theme.operator.clone()));
                index += length;
                expect_command = true;
            }
            '\'' | '"' => {
                let quote = ch;
                let mut end = index + ch.len_utf8();
                let mut closed = false;
                while let Some(c) = line[end..].chars().next() {
                    end += c.len_utf8();
                    if c == quote {
                        closed = true;
                        break;
                    }
                }
                let style = if closed {
                    theme.string.clone()
                } else {
                    theme.invalid_command.clone()
                };
                spans.push((index..end, style));
                index = end;
            }
            '$' => {
                let mut end = index + ch.len_utf8();
                if line[end..].starts_with('{') {
                    while let Some(c) = line[end..].chars().next() {
                        end += c.len_utf8();
                        if c == '}' {
                            break;
                        }
                    }
                } else {
                    while let Some(c) = line[end..].chars().next() {
                        if c.is_alphanumeric() || c == '_' || c == '?' {
                            end += c.len_utf8();
                        } else {
                            break;
                        }
                    }
                }
                spans.push((index..end, theme.variable.clone()));
                index = end;
            }
            _ => {
                let mut end = index;
                while let Some(c) = line[end..].chars().next() {
                    if c.is_whitespace()
                        || matches!(c, '|' | '&' | ';' | '<' | '>' | '\'' | '"' | '$')
                    {
                        break;
                    }
                    end += c.len_utf8();
                }
                let word = &line[index..end];
                if expect_command {
                    let style = if shell.is_known_command(word) {
                        theme.valid_command.clone()
                    } else {
                        theme.invalid_command.clone()
                    };
                    spans.push((index..end, style));
                    expect_command = false;
                } else if word.starts_with('-') {
                    spans.push((index..end, theme.option.clone()));
                }
                index = end;
            }
        }
    }
    spans
}

/// 查询某个字节位置对应的样式。
pub fn style_at(spans: &[(Range<usize>, String)], index: usize) -> &str {
    spans
        .iter()
        .find(|(range, _)| range.contains(&index))
        .map(|(_, style)| style.as_str())
        .unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn marks_valid_and_invalid_commands() {
        let mut shell = Shell::new(Config::default(), false);
        let config = Config::default();
        let valid = spans(&mut shell, "help", &config);
        assert_eq!(style_at(&valid, 0), config.highlight.valid_command);

        let invalid = spans(&mut shell, "nope-xyz", &config);
        assert_eq!(style_at(&invalid, 0), config.highlight.invalid_command);
    }

    #[test]
    fn marks_strings_operators_and_variables() {
        let mut shell = Shell::new(Config::default(), false);
        let config = Config::default();
        let line = "echo \"hi\" | grep $HOME -n";
        let spans = spans(&mut shell, line, &config);
        let quote = line.find('"').unwrap();
        assert_eq!(style_at(&spans, quote), config.highlight.string);
        let pipe = line.find('|').unwrap();
        assert_eq!(style_at(&spans, pipe), config.highlight.operator);
        let var = line.find('$').unwrap();
        assert_eq!(style_at(&spans, var), config.highlight.variable);
        let option = line.rfind("-n").unwrap();
        assert_eq!(style_at(&spans, option), config.highlight.option);
    }

    #[test]
    fn treats_command_after_pipe_as_command() {
        let mut shell = Shell::new(Config::default(), false);
        let config = Config::default();
        let line = "echo x | nope-xyz";
        let spans = spans(&mut shell, line, &config);
        let second = line.rfind("nope-xyz").unwrap();
        assert_eq!(style_at(&spans, second), config.highlight.invalid_command);
    }

    #[test]
    fn marks_comment_tail() {
        let mut shell = Shell::new(Config::default(), false);
        let config = Config::default();
        let line = "help # 说明";
        let spans = spans(&mut shell, line, &config);
        let hash = line.find('#').unwrap();
        assert_eq!(style_at(&spans, hash), config.highlight.comment);
    }
}

//! 极简 TOML 子集解析器（仅标准库）。
//!
//! 支持：注释、表头 `[a.b]`、点分键、基本/字面字符串、整数、浮点、布尔、
//! 数组（可跨行）、内联表。不支持数组表 `[[x]]`（配置里不需要）。

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Str(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Array(Vec<Value>),
    Table(Table),
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Table {
    entries: Vec<(String, Value)>,
}

impl Table {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &Value)> {
        self.entries.iter().map(|(k, v)| (k.as_str(), v))
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    /// 按 `a.b.c` 路径查找。
    pub fn lookup(&self, path: &str) -> Option<&Value> {
        let mut current = self;
        let mut parts = path.split('.').peekable();
        while let Some(part) = parts.next() {
            let value = current.get(part)?;
            if parts.peek().is_none() {
                return Some(value);
            }
            match value {
                Value::Table(inner) => current = inner,
                _ => return None,
            }
        }
        None
    }

    pub fn table(&self, path: &str) -> Option<&Table> {
        match self.lookup(path)? {
            Value::Table(t) => Some(t),
            _ => None,
        }
    }

    pub fn string(&self, path: &str) -> Option<String> {
        match self.lookup(path)? {
            Value::Str(s) => Some(s.clone()),
            Value::Int(i) => Some(i.to_string()),
            Value::Bool(b) => Some(b.to_string()),
            _ => None,
        }
    }

    pub fn bool(&self, path: &str) -> Option<bool> {
        match self.lookup(path)? {
            Value::Bool(b) => Some(*b),
            Value::Str(s) => match s.as_str() {
                "true" | "yes" | "on" => Some(true),
                "false" | "no" | "off" => Some(false),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn int(&self, path: &str) -> Option<i64> {
        match self.lookup(path)? {
            Value::Int(i) => Some(*i),
            Value::Float(f) => Some(*f as i64),
            Value::Str(s) => s.parse().ok(),
            _ => None,
        }
    }

    pub fn usize(&self, path: &str) -> Option<usize> {
        self.int(path).and_then(|v| usize::try_from(v).ok())
    }

    pub fn string_list(&self, path: &str) -> Option<Vec<String>> {
        match self.lookup(path)? {
            Value::Array(items) => Some(
                items
                    .iter()
                    .filter_map(|v| match v {
                        Value::Str(s) => Some(s.clone()),
                        Value::Int(i) => Some(i.to_string()),
                        _ => None,
                    })
                    .collect(),
            ),
            Value::Str(s) => Some(vec![s.clone()]),
            _ => None,
        }
    }

    /// 取出一个「字符串 -> 字符串」的表（alias / abbr / env 用）。
    pub fn str_pairs(&self, path: &str) -> Vec<(String, String)> {
        let Some(table) = self.table(path) else {
            return Vec::new();
        };
        table
            .iter()
            .filter_map(|(k, v)| match v {
                Value::Str(s) => Some((k.to_string(), s.clone())),
                Value::Int(i) => Some((k.to_string(), i.to_string())),
                Value::Bool(b) => Some((k.to_string(), b.to_string())),
                _ => None,
            })
            .collect()
    }

    fn set(&mut self, key: &str, value: Value) {
        if let Some(slot) = self.entries.iter_mut().find(|(k, _)| k == key) {
            slot.1 = value;
        } else {
            self.entries.push((key.to_string(), value));
        }
    }

    fn table_mut(&mut self, key: &str) -> &mut Table {
        let existing = self.entries.iter().position(|(k, _)| k == key);
        let index = match existing {
            Some(index) => {
                if !matches!(self.entries[index].1, Value::Table(_)) {
                    self.entries[index].1 = Value::Table(Table::new());
                }
                index
            }
            None => {
                self.entries
                    .push((key.to_string(), Value::Table(Table::new())));
                self.entries.len() - 1
            }
        };
        match &mut self.entries[index].1 {
            Value::Table(t) => t,
            _ => unreachable!("刚刚已确保是表"),
        }
    }

    fn insert_path(&mut self, path: &[String], value: Value) {
        let Some((last, parents)) = path.split_last() else {
            return;
        };
        let mut current = self;
        for part in parents {
            current = current.table_mut(part);
        }
        current.set(last, value);
    }

    fn ensure_path(&mut self, path: &[String]) {
        let mut current = self;
        for part in path {
            current = current.table_mut(part);
        }
    }
}

/// 解析 TOML 文本。
pub fn parse(src: &str) -> Result<Table, String> {
    let mut root = Table::new();
    let mut prefix: Vec<String> = Vec::new();
    let mut lines = src.lines().enumerate();

    while let Some((index, raw)) = lines.next() {
        let line_no = index + 1;
        let line = strip_comment(raw).trim().to_string();
        if line.is_empty() {
            continue;
        }
        if let Some(header) = line.strip_prefix('[') {
            if header.starts_with('[') {
                return Err(format!("第 {line_no} 行：暂不支持数组表 `[[...]]`"));
            }
            let Some(header) = header.strip_suffix(']') else {
                return Err(format!("第 {line_no} 行：表头缺少 `]`"));
            };
            prefix = split_key(header.trim()).map_err(|e| format!("第 {line_no} 行：{e}"))?;
            root.ensure_path(&prefix);
            continue;
        }

        let Some(eq) = find_unquoted(&line, '=') else {
            return Err(format!("第 {line_no} 行：缺少 `=`"));
        };
        let key = split_key(line[..eq].trim()).map_err(|e| format!("第 {line_no} 行：{e}"))?;
        let mut value_src = line[eq + 1..].trim().to_string();
        while unbalanced(&value_src) {
            let Some((_, next)) = lines.next() else {
                return Err(format!("第 {line_no} 行：数组/内联表未闭合"));
            };
            value_src.push(' ');
            value_src.push_str(strip_comment(next).trim());
        }
        let (value, rest) = parse_value(&value_src).map_err(|e| format!("第 {line_no} 行：{e}"))?;
        if !rest.trim().is_empty() {
            return Err(format!("第 {line_no} 行：多余内容 `{}`", rest.trim()));
        }
        let mut full = prefix.clone();
        full.extend(key);
        root.insert_path(&full, value);
    }
    Ok(root)
}

fn strip_comment(line: &str) -> &str {
    match find_unquoted(line, '#') {
        Some(index) => &line[..index],
        None => line,
    }
}

/// 找到第一个不在引号内的目标字符。
fn find_unquoted(text: &str, target: char) -> Option<usize> {
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;
    for (index, c) in text.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match c {
            '\\' if in_double => escaped = true,
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            c if c == target && !in_single && !in_double => return Some(index),
            _ => {}
        }
    }
    None
}

fn unbalanced(text: &str) -> bool {
    let mut depth = 0i32;
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;
    for c in text.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        match c {
            '\\' if in_double => escaped = true,
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            '[' | '{' if !in_single && !in_double => depth += 1,
            ']' | '}' if !in_single && !in_double => depth -= 1,
            _ => {}
        }
    }
    depth > 0
}

fn split_key(raw: &str) -> Result<Vec<String>, String> {
    if raw.is_empty() {
        return Err("键名为空".to_string());
    }
    let mut parts = Vec::new();
    let mut rest = raw;
    loop {
        let (part, remainder) = match find_unquoted(rest, '.') {
            Some(index) => (&rest[..index], Some(&rest[index + 1..])),
            None => (rest, None),
        };
        let part = part.trim();
        let part = part
            .strip_prefix('"')
            .and_then(|p| p.strip_suffix('"'))
            .or_else(|| part.strip_prefix('\'').and_then(|p| p.strip_suffix('\'')))
            .unwrap_or(part);
        if part.is_empty() {
            return Err(format!("非法键名 `{raw}`"));
        }
        parts.push(part.to_string());
        match remainder {
            Some(next) => rest = next,
            None => break,
        }
    }
    Ok(parts)
}

fn parse_value(src: &str) -> Result<(Value, &str), String> {
    let trimmed = src.trim_start();
    let offset = src.len() - trimmed.len();
    let _ = offset;
    let mut chars = trimmed.char_indices();
    let Some((_, first)) = chars.next() else {
        return Err("值为空".to_string());
    };
    match first {
        '"' => parse_basic_string(trimmed),
        '\'' => parse_literal_string(trimmed),
        '[' => parse_array(trimmed),
        '{' => parse_inline_table(trimmed),
        _ => parse_scalar(trimmed),
    }
}

fn parse_basic_string(src: &str) -> Result<(Value, &str), String> {
    let mut out = String::new();
    let mut chars = src.char_indices();
    chars.next(); // 跳过开头引号
    while let Some((index, c)) = chars.next() {
        match c {
            '"' => return Ok((Value::Str(out), &src[index + 1..])),
            '\\' => {
                let Some((_, esc)) = chars.next() else {
                    return Err("字符串转义未完成".to_string());
                };
                out.push(match esc {
                    'n' => '\n',
                    't' => '\t',
                    'r' => '\r',
                    '0' => '\0',
                    'e' => '\x1b',
                    '"' => '"',
                    '\\' => '\\',
                    other => other,
                });
            }
            c => out.push(c),
        }
    }
    Err("字符串缺少结束引号".to_string())
}

fn parse_literal_string(src: &str) -> Result<(Value, &str), String> {
    let body = &src[1..];
    match body.find('\'') {
        Some(end) => Ok((Value::Str(body[..end].to_string()), &body[end + 1..])),
        None => Err("字面字符串缺少结束引号".to_string()),
    }
}

fn parse_array(src: &str) -> Result<(Value, &str), String> {
    let mut rest = &src[1..];
    let mut items = Vec::new();
    loop {
        rest = rest.trim_start();
        if let Some(remainder) = rest.strip_prefix(']') {
            return Ok((Value::Array(items), remainder));
        }
        if rest.is_empty() {
            return Err("数组缺少 `]`".to_string());
        }
        let (value, remainder) = parse_value(rest)?;
        items.push(value);
        rest = remainder.trim_start();
        if let Some(remainder) = rest.strip_prefix(',') {
            rest = remainder;
        }
    }
}

fn parse_inline_table(src: &str) -> Result<(Value, &str), String> {
    let mut rest = &src[1..];
    let mut table = Table::new();
    loop {
        rest = rest.trim_start();
        if let Some(remainder) = rest.strip_prefix('}') {
            return Ok((Value::Table(table), remainder));
        }
        if rest.is_empty() {
            return Err("内联表缺少 `}`".to_string());
        }
        let Some(eq) = find_unquoted(rest, '=') else {
            return Err("内联表缺少 `=`".to_string());
        };
        let key = split_key(rest[..eq].trim())?;
        let (value, remainder) = parse_value(&rest[eq + 1..])?;
        table.insert_path(&key, value);
        rest = remainder.trim_start();
        if let Some(remainder) = rest.strip_prefix(',') {
            rest = remainder;
        }
    }
}

fn parse_scalar(src: &str) -> Result<(Value, &str), String> {
    let end = src
        .find([',', ']', '}'])
        .unwrap_or(src.len())
        .min(src.find(char::is_whitespace).unwrap_or(src.len()));
    let (token, rest) = src.split_at(end);
    let token = token.trim();
    let value = match token {
        "true" => Value::Bool(true),
        "false" => Value::Bool(false),
        _ => {
            let cleaned = token.replace('_', "");
            if let Ok(i) = cleaned.parse::<i64>() {
                Value::Int(i)
            } else if let Ok(f) = cleaned.parse::<f64>() {
                Value::Float(f)
            } else {
                return Err(format!("无法解析的值 `{token}`"));
            }
        }
    };
    Ok((value, rest))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_scalars_and_tables() {
        let table = parse(
            r#"
# 顶层
format = "$dir$character"
add_newline = true
threshold = 2_000
ratio = 0.5

[menu]
max_rows = 8
sources = ["history", "command",
           "path"]

[aliases]
ll = "ls -lah"
"#,
        )
        .expect("解析失败");

        assert_eq!(table.string("format").as_deref(), Some("$dir$character"));
        assert_eq!(table.bool("add_newline"), Some(true));
        assert_eq!(table.int("threshold"), Some(2000));
        assert_eq!(table.usize("menu.max_rows"), Some(8));
        assert_eq!(
            table.string_list("menu.sources"),
            Some(vec![
                "history".to_string(),
                "command".to_string(),
                "path".to_string()
            ])
        );
        assert_eq!(
            table.str_pairs("aliases"),
            vec![("ll".to_string(), "ls -lah".to_string())]
        );
    }

    #[test]
    fn keeps_hash_inside_strings() {
        let table = parse(r##"symbol = "#1 " # 真注释"##).expect("解析失败");
        assert_eq!(table.string("symbol").as_deref(), Some("#1 "));
    }

    #[test]
    fn supports_dotted_keys_and_inline_tables() {
        let table = parse("a.b.c = 1\nd = { e = \"x\", f = true }").expect("解析失败");
        assert_eq!(table.int("a.b.c"), Some(1));
        assert_eq!(table.string("d.e").as_deref(), Some("x"));
        assert_eq!(table.bool("d.f"), Some(true));
    }

    #[test]
    fn reports_errors() {
        assert!(parse("oops").is_err());
        assert!(parse("[unclosed").is_err());
    }
}

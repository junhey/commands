//! 够用就好的 JSON 读取：只解决「取顶层字段」这一件事，不建 AST。
//!
//! 为什么不引依赖：整个项目零运行时依赖（这也是 musl 静态链接的前提），
//! 而这里要读的只有 package.json 的 `version` 和 `scripts` 两处。
//!
//! 为什么要跟踪嵌套深度：`"version"` 和 `"scripts"` 在 package.json 里
//! 到处都会出现——`engines`、依赖的 lock 信息里都有同名键。只找第一个匹配
//! 会读到别人的值，而且读到的还是个看起来很合理的版本号，很难发现。

/// 遍历所有字符串**值**，把「所在层级的键路径」交给回调。
///
/// `keys[i]` 是第 `i + 1` 层当前的键，`keys.len()` 就是这个值所处的深度：
/// 顶层字段是 1，`scripts` 里的成员是 2。
fn scan_strings(text: &str, mut visit: impl FnMut(&[Option<String>], &str)) {
    let mut keys: Vec<Option<String>> = Vec::new();
    let mut in_string = false;
    let mut escaped = false;
    let mut literal = String::new();

    for (index, c) in text.char_indices() {
        if in_string {
            if escaped {
                // 转义字符原样保留，`"a \"b\" c"` 应该读成 `a "b" c`
                literal.push(c);
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
                // 字符串后面紧跟冒号说明它是键，否则是值
                let is_key = text[index + 1..].trim_start().starts_with(':');
                if is_key {
                    if let Some(slot) = keys.last_mut() {
                        *slot = Some(std::mem::take(&mut literal));
                    }
                } else {
                    visit(&keys, &literal);
                    // 值消费完就把键清掉，免得后面的裸值蹭上一个键名
                    if let Some(slot) = keys.last_mut() {
                        *slot = None;
                    }
                }
                literal.clear();
            } else {
                literal.push(c);
            }
            continue;
        }
        match c {
            '"' => {
                in_string = true;
                literal.clear();
            }
            '{' | '[' => keys.push(None),
            '}' | ']' => {
                keys.pop();
            }
            ',' => {
                if let Some(slot) = keys.last_mut() {
                    *slot = None;
                }
            }
            _ => {}
        }
    }
}

/// 取顶层的字符串字段，嵌套层里的同名键不算。
pub fn top_level_string(text: &str, key: &str) -> Option<String> {
    let mut found: Option<String> = None;
    scan_strings(text, |keys, value| {
        if found.is_none() && keys.len() == 1 && keys[0].as_deref() == Some(key) {
            found = Some(value.to_string());
        }
    });
    found
}

/// 取顶层某个对象里的字符串成员，按文件里的出现顺序返回。
/// 用于 package.json 的 `scripts`。
pub fn top_level_string_map(text: &str, key: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    scan_strings(text, |keys, value| {
        if keys.len() == 2 && keys[0].as_deref() == Some(key) {
            if let Some(name) = keys[1].as_deref() {
                out.push((name.to_string(), value.to_string()));
            }
        }
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const PACKAGE: &str = r#"{
  "name": "demo",
  "engines": { "node": ">=18", "version": "ignored-nested" },
  "version": "1.4.2",
  "scripts": {
    "dev": "vite",
    "build": "vite build && tsc",
    "test": "vitest run"
  },
  "dependencies": { "left-pad": { "version": "9.9.9" } }
}"#;

    #[test]
    fn reads_only_top_level_fields() {
        assert_eq!(
            top_level_string(PACKAGE, "version").as_deref(),
            Some("1.4.2")
        );
        assert_eq!(top_level_string(PACKAGE, "name").as_deref(), Some("demo"));
        // 顶层没有就返回 None，不能拿嵌套的凑数
        assert_eq!(top_level_string(PACKAGE, "node"), None);
        assert_eq!(top_level_string(r#"{"a":{"b":"c"}}"#, "b"), None);
    }

    #[test]
    fn reads_script_map_in_file_order() {
        let scripts = top_level_string_map(PACKAGE, "scripts");
        assert_eq!(
            scripts,
            vec![
                ("dev".to_string(), "vite".to_string()),
                ("build".to_string(), "vite build && tsc".to_string()),
                ("test".to_string(), "vitest run".to_string()),
            ]
        );
        // 嵌套两层以上的对象不该被当成 scripts 的成员
        assert!(top_level_string_map(PACKAGE, "dependencies").is_empty());
    }

    #[test]
    fn handles_escapes_without_losing_state() {
        let text = r#"{ "desc": "a \"quoted\" word", "version": "2.0.0" }"#;
        assert_eq!(
            top_level_string(text, "desc").as_deref(),
            Some(r#"a "quoted" word"#)
        );
        // 转义没让扫描状态错位，后面的字段还能正常读到
        assert_eq!(top_level_string(text, "version").as_deref(), Some("2.0.0"));
    }

    #[test]
    fn survives_arrays_and_empty_objects() {
        let text = r#"{ "files": ["a", "b"], "scripts": {}, "version": "3.0.0" }"#;
        assert!(top_level_string_map(text, "scripts").is_empty());
        assert_eq!(top_level_string(text, "version").as_deref(), Some("3.0.0"));
        // 数组元素不是键值对，不该被当成 files 的成员
        assert!(top_level_string_map(text, "files").is_empty());
    }

    #[test]
    fn tolerates_garbage() {
        assert_eq!(top_level_string("", "version"), None);
        assert_eq!(top_level_string("not json at all", "version"), None);
        // 截断的文件不该 panic
        assert_eq!(top_level_string(r#"{"version": "1.0"#, "version"), None);
    }
}

//! 把 cmds 的提示符接到其它 shell（bash / zsh / fish / powershell）。

fn binary() -> String {
    std::env::current_exe()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "cmds".to_string())
}

/// 返回可直接 `eval` 的集成脚本。
pub fn script(target: &str) -> Option<String> {
    let cmds = binary();
    // 注释会被 eval 进用户的 shell 配置，算用户可见文案，跟着界面语言走。
    let script = match target {
        "bash" => format!(
            r#"# {comment}
__cmds_prompt() {{
    local __cmds_status=$?
    PS1="$('{cmds}' prompt --shell bash --status $__cmds_status)"
}}
case "${{PROMPT_COMMAND:-}}" in
    *__cmds_prompt*) ;;
    "") PROMPT_COMMAND="__cmds_prompt" ;;
    *) PROMPT_COMMAND="__cmds_prompt;$PROMPT_COMMAND" ;;
esac
"#,
            comment = t!(
                r#"cmds prompt integration (bash). Use: eval "$(cmds init bash)""#,
                r#"cmds 提示符集成（bash）。用法：eval "$(cmds init bash)""#
            ),
        ),
        "zsh" => format!(
            r#"# {comment}
__cmds_prompt() {{
    PS1="$('{cmds}' prompt --shell zsh --status $?)"
}}
autoload -Uz add-zsh-hook
add-zsh-hook precmd __cmds_prompt
setopt PROMPT_SUBST
"#,
            comment = t!(
                r#"cmds prompt integration (zsh). Use: eval "$(cmds init zsh)""#,
                r#"cmds 提示符集成（zsh）。用法：eval "$(cmds init zsh)""#
            ),
        ),
        "fish" => format!(
            r#"# {comment}
function fish_prompt
    '{cmds}' prompt --shell fish --status $status --duration $CMD_DURATION
end
"#,
            comment = t!(
                "cmds prompt integration (fish). Use: cmds init fish | source",
                "cmds 提示符集成（fish）。用法：cmds init fish | source"
            ),
        ),
        "powershell" | "pwsh" => format!(
            r#"# {comment}
function global:prompt {{
    $status = if ($?) {{ 0 }} else {{ 1 }}
    & '{cmds}' prompt --shell plain --status $status
}}
"#,
            comment = t!(
                "cmds prompt integration (PowerShell). Use: Invoke-Expression (& cmds init powershell | Out-String)",
                "cmds 提示符集成（PowerShell）。用法：Invoke-Expression (& cmds init powershell | Out-String)"
            ),
        ),
        _ => return None,
    };
    Some(script)
}

pub const SUPPORTED: &[&str] = &["bash", "zsh", "fish", "powershell"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provides_scripts_for_supported_shells() {
        for target in SUPPORTED {
            let script = script(target).unwrap_or_else(|| panic!("{target} 应该有脚本"));
            assert!(script.contains("prompt"));
        }
        assert!(script("unknown-shell").is_none());
    }

    #[test]
    fn bash_script_sets_prompt_command() {
        let script = script("bash").unwrap();
        assert!(script.contains("PROMPT_COMMAND"));
        assert!(script.contains("--shell bash"));
    }
}

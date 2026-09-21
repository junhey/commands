//! 把 cmds 的提示符接到其它 shell（bash / zsh / fish / powershell）。

fn binary() -> String {
    std::env::current_exe()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "cmds".to_string())
}

/// 返回可直接 `eval` 的集成脚本。
pub fn script(target: &str) -> Option<String> {
    let cmds = binary();
    let script = match target {
        "bash" => format!(
            r#"# cmds 提示符集成（bash）：eval "$({cmds} init bash)"
__cmds_prompt() {{
    local __cmds_status=$?
    PS1="$('{cmds}' prompt --shell bash --status $__cmds_status)"
}}
case "${{PROMPT_COMMAND:-}}" in
    *__cmds_prompt*) ;;
    "") PROMPT_COMMAND="__cmds_prompt" ;;
    *) PROMPT_COMMAND="__cmds_prompt;$PROMPT_COMMAND" ;;
esac
"#
        ),
        "zsh" => format!(
            r#"# cmds 提示符集成（zsh）：eval "$({cmds} init zsh)"
__cmds_prompt() {{
    PS1="$('{cmds}' prompt --shell zsh --status $?)"
}}
autoload -Uz add-zsh-hook
add-zsh-hook precmd __cmds_prompt
setopt PROMPT_SUBST
"#
        ),
        "fish" => format!(
            r#"# cmds 提示符集成（fish）：{cmds} init fish | source
function fish_prompt
    '{cmds}' prompt --shell fish --status $status --duration $CMD_DURATION
end
"#
        ),
        "powershell" | "pwsh" => format!(
            r#"# cmds 提示符集成（PowerShell）：Invoke-Expression (& '{cmds}' init powershell | Out-String)
function global:prompt {{
    $status = if ($?) {{ 0 }} else {{ 1 }}
    & '{cmds}' prompt --shell plain --status $status
}}
"#
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

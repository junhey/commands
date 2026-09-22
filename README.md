# Commands (`cmds`)

[![crates.io](https://img.shields.io/crates/v/cmds.svg)](https://crates.io/crates/cmds)
[![CI](https://github.com/junhey/commands/actions/workflows/ci.yml/badge.svg)](https://github.com/junhey/commands/actions/workflows/ci.yml)
[![license](https://img.shields.io/badge/license-ISC-blue.svg)](https://github.com/junhey/commands/blob/master/LICENSE)

**English** · [简体中文](https://github.com/junhey/commands/blob/master/README.zh-CN.md)

> A small, fast interactive shell: it suggests from your history as you type, and `Tab` opens a
> candidate menu you navigate with `↑` `↓`. The prompt follows the modular approach of
> [starship](https://github.com/starship/starship); the input experience is borrowed from
> [fish](https://fishshell.com/). One executable, no runtime dependencies.

```
~/projects/commands on master !2 🦀 v1.95.0
❯ cargo bu[ild --release --locked]          ← grey text is a history suggestion, → to accept

❯ git c
▸ git commit -m "fix: guard the bounds"   history · used 12 times
  git checkout -b feature/menu            history · used 5 times
  checkout                                command · PATH
1/8 · Tab next · ↑↓ select · Enter accept · Esc close
```

Website and Playground: <https://junhey.github.io/commands/>　·　Guide: <https://junhey.github.io/commands/#guide>

---

## Install

```sh
# macOS / Linux / FreeBSD
curl -fsSL https://junhey.github.io/commands/install.sh | sh
```

```powershell
# Windows
irm https://junhey.github.io/commands/install.ps1 | iex
```

```sh
# From crates.io (requires Rust 1.85+)
cargo install --locked cmds
```

The installer detects your platform and prefers a prebuilt binary; when no prebuilt package
matches, it falls back to building with `cargo`. Every download is verified against the matching
`.sha256` published with the release, and the install aborts if the checksum does not match.

It installs for your user only: no `sudo`, and it never edits your shell startup files.
The binary goes to `/usr/local/bin` when that directory exists and is writable, otherwise
`~/.local/bin`.

Common flags: `--bin-dir <dir>`, `--version v0.1.2`, `--force`, `--build`, `--skip-verify`, `--yes`.
On Windows the equivalents are `-BinDir`, `-Version`, `-Force`, `-Build`, `-SkipVerify`, `-Yes`.
Reading a script before piping it into a shell is a good habit — the install dialog on the website
links to the source directly.

### Prebuilt targets

| Platform | Target |
| --- | --- |
| Linux (x86-64) | `x86_64-unknown-linux-gnu` |
| Linux (ARM64) | `aarch64-unknown-linux-gnu` |
| macOS (Intel) | `x86_64-apple-darwin` |
| macOS (Apple silicon) | `aarch64-apple-darwin` |
| Windows (x86-64) | `x86_64-pc-windows-msvc` |
| Windows (ARM64) | `aarch64-pc-windows-msvc` |

Anything else builds from source via `cargo`, which the installer does for you.

## Quick start

```sh
cmds                # start the interactive shell
help                # list every keybinding and builtin
cmds config init    # write a config template (optional)
```

## Features

| Capability | Details |
| --- | --- |
| History autosuggestion | The best matching history entry appears in grey after the cursor. `→` / `End` / `Ctrl-F` accepts the whole line, `Alt-→` accepts one word. Falls back to completion candidates when history has no match |
| `Tab` candidate menu | `Tab` opens a menu: whole history commands first, then builtins, aliases, abbreviations, `PATH` commands, files and directories, and variables. `↑` `↓` to select, `Enter` to accept, `Esc` to close |
| Completion details | A single candidate is inserted directly; multiple candidates first complete the shared prefix; `cd` only lists directories; paths containing spaces get quoted automatically |
| History search | `Ctrl-R` for fuzzy search, `↑` `↓` walks history filtered by the current prefix, ranked by frequency plus recency |
| Live syntax highlighting | Existing commands turn green, typos red; strings, operators, variables and flags each get their own color. Unknown commands get edit-distance suggestions, with builtins and your own aliases ranked first |
| Modular prompt | `$dir` `$git_branch` `$git_status` `$languages` `$cmd_duration` `$status` `$character` `$time` `$identity` `$jobs`, configured in TOML |
| fish-style abbreviations | `abbr gcm "git commit -m"` expands when you press space, and history stores the expanded command |
| Execution | Pipes, `&&` `\|\|` `;`, redirection (`>` `>>` `<` `2>` `2>>` `&>`), background `&`, globs (`*` `?` `[a-z]` `**`), variable expansion |
| Prompt-only mode | `eval "$(cmds init bash)"` or `cmds init fish \| source` plugs the prompt into your existing bash / zsh / fish / PowerShell |

## Keybindings (selection)

| Key | Action |
| --- | --- |
| `Tab` / `Shift-Tab` | Open the candidate menu / previous item in the menu |
| `↑` `↓` (`Ctrl-P` `Ctrl-N`) | Move within the menu; with the menu closed, walk history by prefix |
| `Enter` | Accept the candidate when the menu is open, otherwise run the line |
| `→` `End` `Ctrl-F` | Accept the whole grey suggestion |
| `Alt-→` / `Alt-←` | Accept one word of the suggestion / move left by word |
| `Ctrl-R` | Fuzzy-search history |
| `Ctrl-A` `Ctrl-E` `Ctrl-W` `Ctrl-U` `Ctrl-K` `Ctrl-L` | Start of line / end of line / delete word / delete to start / delete to end / clear screen |
| `Ctrl-C` / `Ctrl-D` | Discard the current line / exit on an empty line |

For the full list run `help` inside the shell, or read the
[guide](https://junhey.github.io/commands/#guide).

## Configuration

Config file: `~/.config/cmds/config.toml` (Windows: `%APPDATA%\cmds\config.toml`).
`CMDS_CONFIG` overrides the path, `cmds config init` writes a template, and `config reload`
reloads without restarting.

The **Config page** on the website generates this file from checkboxes, so you do not have to
write TOML by hand.

```toml
format = "$dir$git_branch$git_status$languages$cmd_duration$status$line_break$character"
add_newline = true

[autosuggest]
enabled = true
sources = ["history", "completion"]

[menu]
max_rows = 8
selected_style = "bold fg:black bg:cyan"

[git]
status_enabled = true      # worth disabling in very large repositories

[aliases]
ll = "ls -lah"

[abbreviations]
gcm = "git commit -m"
```

Startup script: `~/.cmdsrc` (or `~/.config/cmds/init.cmds`) may contain any cmds commands.

## Language

Everything — the CLI, the installer, the config template and the website — is **English by
default**. Chinese appears only when your environment asks for it, using the usual POSIX
precedence: `CMDS_LANG` > `LC_ALL` > `LC_MESSAGES` > `LANG`. A locale starting with `zh`
selects Chinese; `C`, `POSIX`, anything else and unset all mean English.

```sh
cmds                         # follows your locale
CMDS_LANG=zh cmds            # force Chinese
CMDS_LANG=en cmds            # force English on a Chinese system
CMDS_LANG=en sh install.sh   # the installer honours the same variable
```

The website follows your browser language and has a switch in the sidebar.

## Command-line usage

```sh
cmds                                   # interactive
cmds -c "cargo test && echo done"      # run a single command
cmds script.cmds arg1 arg2             # run a script ($1 $2 available inside)
cmds prompt --status 1 --duration 3500 # print just the prompt
cmds init bash|zsh|fish|powershell     # print the integration snippet
cmds config path|init|reload|show      # manage configuration
```

## How it compares

| | cmds | fish | starship | zsh-autosuggestions |
| --- | --- | --- | --- | --- |
| History autosuggestion | ✅ | ✅ | — | ✅ |
| Candidate menu with history entries | ✅ | ✅ | — | — |
| Modular prompt | ✅ | via config | ✅ | — |
| Works as a prompt for other shells | ✅ | — | ✅ | — |
| Scripting language | — | ✅ | — | n/a |
| Install footprint | one binary | package + config | one binary | zsh plugin |

cmds is not trying to replace any of them. It bundles the two things you feel on every keystroke —
fish-style input and a starship-style prompt — into a single binary with no plugin manager, and it
can also be reduced to just the prompt inside the shell you already use.

## Uninstall

```sh
rm -f "$(command -v cmds)"           # the binary
rm -rf ~/.config/cmds                # configuration
rm -rf ~/.local/share/cmds           # history
```

On Windows remove `cmds.exe` from the install directory, then `%APPDATA%\cmds` and
`%LOCALAPPDATA%\cmds`. Nothing else is left behind: the installer never touched your startup files.
If you had added cmds to `/etc/shells` or run `chsh`, switch your login shell back first.

## FAQ

**Is this a replacement for bash or zsh in scripts?** No. cmds targets interactive use and has no
functions or `if` / `for` syntax. Keep using `sh` / `bash` for scripts.

**Can I make it my login shell?** Yes, and the installer prints the commands for it, but use it
day to day for a while first.

**Can I keep my current shell and still get the prompt?** Yes — that is prompt-only mode:
`eval "$(cmds init bash)"`, `cmds init fish | source`, and the equivalents for zsh and PowerShell.

**Does it read my existing zsh/bash config?** No. Aliases and abbreviations live in cmds' own TOML
config, which keeps startup instant and behavior predictable.

**Where is my data?** History in `~/.local/share/cmds/history`
(Windows: `%LOCALAPPDATA%\cmds\history`), config in `~/.config/cmds/`. Nothing leaves the machine.

## Project layout

```
cli/                the cmds binary, written in Rust
├── src/
│   ├── main.rs         CLI entry (interactive / -c / script / prompt / init / config)
│   ├── shell.rs        runtime state and the REPL loop
│   ├── editor/         line editor: suggestions, candidate menu, highlighting, rendering
│   ├── prompt/         modular prompt
│   ├── exec.rs         pipes, redirection, logical operators, background jobs
│   ├── builtins.rs     builtin commands
│   ├── parser.rs       lexer and parser
│   ├── history.rs      history storage and ranking
│   ├── config/         TOML parsing and config types
│   └── glob.rs         small glob implementation
└── assets/         default config template
web/                website + browser Playground (Vite + React, fully static)
install/            install scripts (sh / ps1), served from the site root
docs/               additional documentation
```

## Development

```sh
# CLI
cargo test                    # 109 unit tests
cargo build --release --locked
cargo run -- -c "echo hello"

# Website
cd web && npm install
npm run dev                   # dev server, also serves /install.sh
npm test                      # Playground core tests
npm run build                 # static output in web/dist

# Default language (English everywhere unless the locale says otherwise)
sh scripts/check-cli-language.sh              # runs the real binary and install.sh
node scripts/check-web-language.mjs web/src/*.js web/src/*.jsx
```

`web/src/commands.js` deliberately mirrors several CLI rules (the builtin list, candidate ranking,
the suggestion algorithm, history weighting), so a change on one side needs the same change on the
other. Details in
[CONTRIBUTING.md](https://github.com/junhey/commands/blob/master/CONTRIBUTING.md).

Further reading:
[architecture](https://github.com/junhey/commands/blob/master/docs/architecture.md) ·
[deployment](https://github.com/junhey/commands/blob/master/docs/deployment.md)

## Privacy

No account, no cloud history, no telemetry, no network requests of any kind.

History is stored locally at `~/.local/share/cmds/history`
(Windows: `%LOCALAPPDATA%\cmds\history`), readable and writable by the owner only on Unix.
Commands starting with a space are not recorded. Everything in the website Playground is simulated,
and its history and preferences never leave your browser's localStorage.

## Scope

cmds is built for **interactive use**; it is not a POSIX sh replacement and has no functions or
`if` / `for` constructs. Keep using `sh` / `bash` for system scripts, and try cmds as your daily
driver for a while before making it your login shell.

## License

[ISC](https://github.com/junhey/commands/blob/master/LICENSE). Inspired by starship and fish —
thanks to both projects. No code from either is copied or redistributed here.

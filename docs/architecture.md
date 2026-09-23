# Architecture

> 中文版：[docs/architecture.zh-CN.md](https://github.com/junhey/commands/blob/master/docs/architecture.zh-CN.md)

## In one sentence

`cli/` is the product, `web/` is its website, Playground and install entry point, and `install/`
is the bridge between them.

```
                      ┌───────────────────────────┐
  try it in a browser │ web/  site + Playground   │
        ─────────────▶│  · simulated terminal     │
                      │  · Config page → TOML     │
                      │  · install dialog         │
                      └─────────────┬─────────────┘
                                    │ served from the site root
                                    ▼
                      ┌───────────────────────────┐
                      │ install/  install.sh/.ps1 │
                      │  · detect the platform    │
                      │  · download + verify      │
                      │  · fall back to cargo     │
                      └─────────────┬─────────────┘
                                    ▼
                      ┌───────────────────────────┐
  run it locally ────▶│ cli/  the cmds binary     │
                      │  · interactive shell      │
                      │  · modular prompt         │
                      └───────────────────────────┘
```

## CLI (`cli/`)

```
main.rs        argument dispatch: interactive / -c / script / prompt / init / config
shell.rs       shell runtime state + the interactive REPL loop
editor/        line editor
  mod.rs         key handling, render scheduling
  suggest.rs     ghost suggestions from history
  complete.rs    candidate generation and ranking
  subcommands.rs built-in subcommand tables (git / cargo / docker …)
  scripts.rs     project scripts: package.json scripts, Makefile targets
  highlight.rs   syntax highlighting
  render.rs      width calculation, wrapping, menu drawing
prompt/        modular prompt (dir / git / languages / container / duration …)
json.rs        minimal JSON reader for top-level fields in package.json
exec.rs        pipes, redirection, logical operators, background jobs
parser.rs      lexing and parsing
builtins.rs    21 builtin commands
history.rs     history storage and ranking (frequency + recency)
config/        TOML parsing and config structures
glob.rs        lightweight globbing
i18n.rs        interface language (English by default)
style.rs       style descriptions → ANSI
util.rs        paths, environment variables, PATH lookup, edit distance
```

A few deliberate trade-offs:

- **Accepting is not running.** `Enter` in the candidate menu only fills the line; a second
  `Enter` runs it. This is a safety decision — a stray keypress should not fire off a
  destructive command.
- **PATH is cached for 30 seconds.** Scanning PATH on every completion is too slow, so
  `Shell::path_commands()` caches for 30 seconds.
- **Spelling suggestions are ranked by category.** See below.
- **No scripting syntax.** No functions, no `if` / `for`. This targets interactive use and is
  not a replacement for sh.
- **English by default.** `i18n.rs` picks a language once per process from `CMDS_LANG` >
  `LC_ALL` > `LC_MESSAGES` > `LANG`. Both languages are written on the same line via
  `t!(en, zh)` — no gettext, no resource files shipped next to the binary. The text volume is
  small, and having both languages in front of you is the best defence against updating only one
  of them.

## Website (`web/`)

```
src/commands.js   Playground core: builtin list, ranking, simulated execution, config generation
src/install.js    install command generation (derived from origin + base)
src/i18n.js       interface language, mirrors cli/src/i18n.rs
src/App.jsx       every page and interaction
src/styles.css    styles
tests/            unit tests for the core and install URLs (node:test, no browser needed)
```

- **Fully static.** Hash routing (`#config`, `#guide`), so no server-side rewrite is needed.
- **No backend.** History, snippets and preferences live in localStorage; if writing fails in
  private mode it degrades silently and the current session still works.
- **The Playground is entirely simulated.** `simulateCommand` only does table lookups and string
  handling — no code execution, no file reads, no requests. Output is explicitly labelled as
  simulated.
- **Install URLs are derived at runtime.** `installMethods(origin, base)` builds them from
  `window.location.origin` and `import.meta.env.BASE_URL`, so changing domain or deploy path
  needs no code change and no fictional domain ever appears.
- **Install scripts are not copied.** The `cmds-install-scripts` Vite plugin reads the files in
  `install/` directly, so the dev server and the build output serve the same source and copies
  cannot go stale.

## Rules kept deliberately identical on both sides

The point of the website is "try before you install", so the Playground has to feel like the real
CLI. These rules are implemented once on each side, each with tests guarding them:

| Rule | Rust | JS |
| --- | --- | --- |
| Builtin command list | `BUILTINS` in `builtins.rs` | `builtins` in `commands.js` |
| Edit distance | `levenshtein` in `util.rs` | `levenshtein` |
| Spelling suggestion ranking | `similar_commands` in `shell.rs` | `similarCommands` |
| History ranking | frequency + recency | `weight` in `getSuggestions` |
| Ghost suggestions | `editor/suggest.rs` | `ghostSuggestion` |
| Config template | default template in `config/mod.rs` | `buildConfigToml` |
| Interface language | `i18n.rs` (`t!` / `tf!`) | `i18n.js` (`t`) |

### How spelling suggestions are ranked

```
edit distance → candidate category (builtin 0 > alias/abbr 1 > PATH 2)
              → same first letter → length difference → lexicographic
```

Ranking by "distance + lexicographic" alone is not enough. Typing `hepl` gives the PATH commands
`h2ph`, `head` and `heap` the same edit distance of 2 as the builtin `help`, and they sort before
it lexicographically — so with `limit = 3` the one you actually wanted got pushed out. Promoting
the category to second place fixes it: builtins and the aliases you defined yourself are
inherently more likely to be what you meant than a random PATH command with the same score.

Both `cli/src/shell.rs` and `web/tests/commands.test.js` have regression tests for this.

## Testing strategy

| Scope | Tool | Count |
| --- | --- | --- |
| CLI | `cargo test` (unit tests inline in each module) | 155 |
| Playground core | `node --test` | 42 |
| Install scripts | `sh -n` + shellcheck + PowerShell Parser | syntax level |
| Default language | `scripts/check-cli-language.sh`, `scripts/check-web-language.mjs` | behavioural + AST |
| End to end | CI smoke-tests `--version` / `-c` / `init` / `prompt` on three platforms | — |

**The parallel-test trap:** the `cd` builtin's tests call `std::env::set_current_dir`, which is a
process-wide side effect. Any test that depends on relative paths must create its own temporary
directory and set `shell.cwd` directly, otherwise it fails at random. CI additionally runs
`--test-threads=1` as a cross-check.

**Why the language checks run real processes:** asserting only "no Chinese in an English
environment" can be satisfied by deleting all the Chinese, which is not internationalisation.
`check-cli-language.sh` therefore also asserts that a Chinese environment still produces Chinese,
so neither direction can rot unnoticed.

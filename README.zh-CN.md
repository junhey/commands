# Commands（`cmds`）

[![crates.io](https://img.shields.io/crates/v/cmds.svg)](https://crates.io/crates/cmds)
[![CI](https://github.com/junhey/commands/actions/workflows/ci.yml/badge.svg)](https://github.com/junhey/commands/actions/workflows/ci.yml)
[![license](https://img.shields.io/badge/license-ISC-blue.svg)](https://github.com/junhey/commands/blob/master/LICENSE)

[English](https://github.com/junhey/commands/blob/master/README.md) · **简体中文**

> 轻量高效的交互式终端：边打字边给历史建议，`Tab` 弹候选菜单用 `↑` `↓` 选。
> 提示符沿用 [starship](https://github.com/starship/starship) 的模块化思路，
> 输入体验取自 [fish](https://fishshell.com/)。一个可执行文件，零运行时依赖。

```
~/projects/commands on master !2 🦀 v1.95.0
❯ cargo bu[ild --release --locked]          ← 灰色为历史建议，→ 采纳

❯ git c
▸ git commit -m "fix: guard the bounds"   历史 · 用过 12 次
  git checkout -b feature/menu    历史 · 用过 5 次
  checkout                        命令 · PATH
1/8 · Tab 下一项 · ↑↓ 选择 · Enter 采纳 · Esc 关闭
```

官网与 Playground：<https://junhey.github.io/commands/>　·　使用指南：<https://junhey.github.io/commands/#guide>

---

## 一键安装

```sh
# macOS / Linux / FreeBSD
curl -fsSL https://junhey.github.io/commands/install.sh | sh
```

```powershell
# Windows
irm https://junhey.github.io/commands/install.ps1 | iex
```

```sh
# 从 crates.io（需要 Rust 1.85+）
cargo install --locked cmds
```

安装脚本会自动识别平台并优先下载预编译二进制，没有对应平台的包时改用 `cargo` 构建。
下载后会用 Release 里同名的 `.sha256` 校验，校验不通过直接中止。

用户级安装，不需要 sudo，也不会改动你的 shell 启动文件。
二进制装到 `/usr/local/bin`（该目录存在且可写时），否则装到 `~/.local/bin`。

常用参数：`--bin-dir <目录>`、`--version v0.1.2`、`--force`、`--build`、`--skip-verify`、`--yes`。
Windows 侧对应 `-BinDir`、`-Version`、`-Force`、`-Build`、`-SkipVerify`、`-Yes`。
把脚本管道给 shell 之前先看一眼内容是个好习惯——站点上的安装对话框有直接查看链接。

### 预编译平台

| 平台 | Target |
| --- | --- |
| Linux（x86-64） | `x86_64-unknown-linux-musl` |
| Linux（ARM64） | `aarch64-unknown-linux-musl` |
| macOS（Intel） | `x86_64-apple-darwin` |
| macOS（Apple 芯片） | `aarch64-apple-darwin` |
| Windows（x86-64） | `x86_64-pc-windows-msvc` |
| Windows（ARM64） | `aarch64-pc-windows-msvc` |

其它平台由 `cargo` 从源码构建，安装脚本会自动完成这一步。

Linux 包是**静态链接 musl** 的，不依赖 glibc，任何发行版都能跑——Debian 10、
CentOS 7、Alpine、精简容器都可以。动态链接的包会把构建机的 glibc 版本写进二进制，
在更老的系统上直接 `version 'GLIBC_2.xx' not found`，v0.2.1 之前就栽在这上面。

## 30 秒上手

```sh
cmds                # 进入交互式
help                # 查看全部快捷键与内建命令
cmds config init    # 生成配置模板（可选）
```

## 核心特性

| 能力 | 说明 |
| --- | --- |
| 历史自动建议 | 输入时在光标后用灰色显示最近匹配的历史命令，`→` / `End` / `Ctrl-F` 采纳整条，`Alt-→` 采纳一个词；历史未命中时退回补全建议 |
| Tab 候选菜单 | `Tab` 弹出菜单，历史整条命令排最前，其后是内建 / 别名 / 缩写 / PATH 命令 / 目录文件 / 变量；`↑` `↓` 选择，`Enter` 采纳，`Esc` 关闭 |
| 智能补全细节 | 唯一候选直接补全；多候选先补公共前缀；`cd` 后只列目录；含空格的路径自动加引号 |
| 历史检索 | `Ctrl-R` 模糊搜索，`↑` `↓` 按当前前缀翻历史，按使用频率 + 新鲜度排序 |
| 实时语法高亮 | 命令存在绿色、拼错红色，字符串 / 操作符 / 变量 / 选项分色；命令找不到时按编辑距离给建议，内建与自定义别名优先 |
| 模块化提示符 | `$dir` `$git_branch` `$git_status` `$languages` `$cmd_duration` `$status` `$character` `$time` `$identity` `$jobs`，TOML 配置 |
| fish 式缩写 | `abbr gcm "git commit -m"`，按空格即展开，历史里存的是完整命令 |
| 执行能力 | 管道、`&&` `\|\|` `;`、重定向（`>` `>>` `<` `2>` `2>>` `&>`）、后台 `&`、glob（`*` `?` `[a-z]` `**`）、变量展开 |
| 也能只当提示符 | `eval "$(cmds init bash)"`、`cmds init fish \| source`，接到现有 bash / zsh / fish / PowerShell |

## 快捷键（节选）

| 按键 | 作用 |
| --- | --- |
| `Tab` / `Shift-Tab` | 打开候选菜单 / 菜单内上一项 |
| `↑` `↓`（`Ctrl-P` `Ctrl-N`） | 菜单内移动；菜单关闭时按前缀翻历史 |
| `Enter` | 菜单打开时采纳候选，否则执行 |
| `→` `End` `Ctrl-F` | 采纳整条灰色建议 |
| `Alt-→` / `Alt-←` | 采纳建议中的一个词 / 按词左移 |
| `Ctrl-R` | 模糊搜索历史 |
| `Ctrl-A` `Ctrl-E` `Ctrl-W` `Ctrl-U` `Ctrl-K` `Ctrl-L` | 行首 / 行尾 / 删词 / 删到行首 / 删到行尾 / 清屏 |
| `Ctrl-C` / `Ctrl-D` | 放弃当前输入 / 空行退出 |

完整列表见交互式命令 `help` 或[使用指南](https://junhey.github.io/commands/#guide)。

## 配置

配置文件：`~/.config/cmds/config.toml`（Windows：`%APPDATA%\cmds\config.toml`），
可用 `CMDS_CONFIG` 指定；`cmds config init` 生成模板，`config reload` 热重载。

官网的 **Config 页**可以勾选提示符模块直接生成这份文件，省得手写 TOML。

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
status_enabled = true      # 超大仓库可以关掉

[aliases]
ll = "ls -lah"

[abbreviations]
gcm = "git commit -m"
```

启动脚本：`~/.cmdsrc`（或 `~/.config/cmds/init.cmds`），里面可以写任意 cmds 命令。

## 界面语言

CLI、安装脚本、配置模板与官网**默认都是英文**，只有环境明确要求时才输出中文。
优先级与 POSIX 一致：`CMDS_LANG` > `LC_ALL` > `LC_MESSAGES` > `LANG`。
以 `zh` 开头的 locale 选中文；`C`、`POSIX`、其它语种和未设置都算英文。

```sh
cmds                         # 跟随系统 locale
CMDS_LANG=zh cmds            # 强制中文
CMDS_LANG=en cmds            # 中文系统上强制英文
CMDS_LANG=en sh install.sh   # 安装脚本认同一个变量
```

官网跟随浏览器语言，侧栏里也有切换按钮。

## 命令行用法

```sh
cmds                                   # 交互式
cmds -c "cargo test && echo done"      # 执行一条命令
cmds script.cmds arg1 arg2             # 执行脚本（脚本内可用 $1 $2）
cmds prompt --status 1 --duration 3500 # 只输出提示符
cmds init bash|zsh|fish|powershell     # 输出集成脚本
cmds config path|init|reload|show      # 配置管理
```

## 和相邻工具的关系

| | cmds | fish | starship | zsh-autosuggestions |
| --- | --- | --- | --- | --- |
| 历史自动建议 | ✅ | ✅ | — | ✅ |
| 候选菜单含历史整条命令 | ✅ | ✅ | — | — |
| 模块化提示符 | ✅ | 靠配置 | ✅ | — |
| 能给别的 shell 当提示符 | ✅ | — | ✅ | — |
| 脚本语言 | — | ✅ | — | 不适用 |
| 安装形态 | 单个二进制 | 包 + 配置 | 单个二进制 | zsh 插件 |

cmds 不打算取代其中任何一个。它把每次敲键都能感觉到的两件事——fish 式输入与 starship 式
提示符——收进一个二进制，不需要插件管理器；也可以只取提示符那部分，接在你现在用的 shell 上。

## 卸载

```sh
rm -f "$(command -v cmds)"           # 二进制
rm -rf ~/.config/cmds                # 配置
rm -rf ~/.local/share/cmds           # 历史
```

Windows 下删掉安装目录里的 `cmds.exe`，再删 `%APPDATA%\cmds` 与 `%LOCALAPPDATA%\cmds`。
不会有其它残留——安装脚本从没动过你的启动文件。
如果之前把 cmds 加进了 `/etc/shells` 或执行过 `chsh`，先把登录 shell 切回去。

## 常见问题

**能拿它替代脚本里的 bash / zsh 吗？** 不能。cmds 面向交互使用，没有函数，也没有
`if` / `for` 语法。系统脚本请继续用 `sh` / `bash`。

**能设成登录 shell 吗？** 可以，安装脚本会把命令打出来，但建议先日常用一段时间再说。

**想留着现在的 shell，只要那个提示符？** 可以，这就是「只当提示符」模式：
`eval "$(cmds init bash)"`、`cmds init fish | source`，zsh 与 PowerShell 同理。

**会读我现有的 zsh / bash 配置吗？** 不会。别名与缩写写在 cmds 自己的 TOML 配置里，
这样启动是瞬时的，行为也可预期。

**我的数据在哪？** 历史在 `~/.local/share/cmds/history`
（Windows：`%LOCALAPPDATA%\cmds\history`），配置在 `~/.config/cmds/`。什么都不会离开本机。

## 项目结构

```
cli/                Rust 实现的 cmds 二进制
├── src/
│   ├── main.rs         CLI 入口（交互 / -c / 脚本 / prompt / init / config）
│   ├── shell.rs        运行时状态与 REPL 主循环
│   ├── editor/         行编辑器：建议、候选菜单、高亮、渲染
│   ├── prompt/         模块化提示符
│   ├── exec.rs         管道、重定向、逻辑连接、后台任务
│   ├── builtins.rs     内建命令
│   ├── parser.rs       词法与语法分析
│   ├── history.rs      历史存储与排序
│   ├── config/         TOML 解析与配置结构
│   └── glob.rs         轻量通配符
└── assets/         默认配置模板
web/                官网 + 浏览器 Playground（Vite + React，纯静态）
install/            一键安装脚本（sh / ps1），随站点发布在根目录
docs/               补充文档
```

## 开发

```sh
# CLI
cargo test                    # 109 个单元测试
cargo build --release --locked
cargo run -- -c "echo hello"

# 站点
cd web && npm install
npm run dev                   # 本地开发，同时提供 /install.sh
npm test                      # Playground 内核测试
npm run build                 # 静态产物到 web/dist

# 默认语言检查（除 locale 指明中文，处处都该是英文）
sh scripts/check-cli-language.sh              # 跑真实二进制与 install.sh
node scripts/check-web-language.mjs web/src/*.js web/src/*.jsx
```

`web/src/commands.js` 刻意复刻了 CLI 的几条规则（内建命令清单、候选排序、
拼写建议算法、历史权重），改了一侧要同步另一侧。细节见
[CONTRIBUTING.md](https://github.com/junhey/commands/blob/master/CONTRIBUTING.md)。

延伸阅读：
[架构说明](https://github.com/junhey/commands/blob/master/docs/architecture.zh-CN.md) ·
[部署上线](https://github.com/junhey/commands/blob/master/docs/deployment.zh-CN.md)

## 隐私

不需要账号，没有云端历史，没有遥测，不发起任何网络请求。

本机历史在 `~/.local/share/cmds/history`（Windows：`%LOCALAPPDATA%\cmds\history`），
Unix 下仅所有者可读写。以空格开头的命令不写入历史。
官网 Playground 的所有命令都是模拟的，历史与偏好只存在浏览器 localStorage。

## 定位说明

cmds 面向**交互使用**，不是 POSIX sh 的替代品（没有函数、`if` / `for` 等脚本语法）。
系统脚本请继续用 `sh` / `bash`；把它设为登录 shell 前建议先日常使用一段时间。

## 许可

[ISC](https://github.com/junhey/commands/blob/master/LICENSE)。灵感来自 starship 与 fish，感谢这两个项目。
本项目没有复制或重新分发它们的任何代码。

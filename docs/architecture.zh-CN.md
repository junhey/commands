# 架构说明

> English: [docs/architecture.md](https://github.com/junhey/commands/blob/master/docs/architecture.md)

## 一句话

`cli/` 是产品本体，`web/` 是它的官网、Playground 与安装入口，`install/` 是两者之间的桥。

```
                     ┌───────────────────────────┐
  用户在浏览器试用 ──▶│ web/  官网 + Playground   │
                     │  · 模拟终端（规则对齐 CLI）│
                     │  · Config 页生成 TOML     │
                     │  · 安装对话框             │
                     └─────────────┬─────────────┘
                                   │ 站点根目录提供
                                   ▼
                     ┌───────────────────────────┐
                     │ install/  install.sh/.ps1 │
                     │  · 识别平台               │
                     │  · 下载 Release + 校验    │
                     │  · 回退 cargo 构建        │
                     └─────────────┬─────────────┘
                                   ▼
                     ┌───────────────────────────┐
  用户在本机使用 ───▶│ cli/  cmds 二进制         │
                     │  · 交互式 shell           │
                     │  · 模块化提示符           │
                     └───────────────────────────┘
```

## CLI（`cli/`）

```
main.rs        参数分发：交互 / -c / 脚本 / prompt / init / config
shell.rs       Shell 运行时状态 + 交互式 REPL 主循环
editor/        行编辑器
  mod.rs         按键处理、渲染调度
  suggest.rs     历史 ghost 建议
  complete.rs    候选生成与排序
  highlight.rs   语法高亮
  render.rs      宽度计算、折行、菜单绘制
prompt/        模块化提示符（dir / git / 语言 / 耗时 / 状态…）
exec.rs        管道、重定向、逻辑连接、后台任务
parser.rs      词法与语法分析
builtins.rs    21 个内建命令
history.rs     历史存储与排序（频率 + 新鲜度）
config/        TOML 解析与配置结构
glob.rs        轻量通配符
i18n.rs        界面语言（默认英文）
style.rs       样式描述 → ANSI
util.rs        路径、环境变量、PATH 查找、编辑距离
```

几个设计取舍：

- **采纳 ≠ 执行。** 候选菜单里按 `Enter` 只把候选填进输入行，需要再按一次
  `Enter` 才执行。这是刻意的安全设计，避免误触跑掉危险命令。
- **PATH 缓存 30 秒。** 每次补全都扫一遍 PATH 太慢，`Shell::path_commands()`
  带 30 秒缓存。
- **拼写建议按类型分层。** 见下节。
- **不做脚本语法。** 没有函数、`if/for`。定位是交互，不是 sh 的替代品。
- **默认英文。** `i18n.rs` 在进程内一次性决定语言，优先级 `CMDS_LANG` >
  `LC_ALL` > `LC_MESSAGES` > `LANG`。中英两份文案用 `t!(en, zh)` 写在同一行——
  不引 gettext，也不额外带资源文件。文案量本来就小，两种语言摆在眼前是防止
  「只改了一边」的最有效手段。

## 站点（`web/`）

```
src/commands.js   Playground 内核：内建清单、候选排序、模拟执行、配置生成
src/install.js    安装命令生成（origin + base 推导）
src/i18n.js       界面语言，与 cli/src/i18n.rs 同构
src/App.jsx       全部页面与交互
src/styles.css    样式
tests/            内核与安装地址的单元测试（node:test，无需浏览器）
```

- **纯静态。** hash 路由（`#config`、`#guide`），不需要服务端 rewrite。
- **无后端。** 历史、片段、偏好都在 localStorage，隐私模式下写入失败会静默降级，当次会话仍可用。
- **Playground 完全模拟。** `simulateCommand` 只做查表与字符串处理，
  不执行代码、不读文件、不发请求。输出里会明确标注「模拟输出」。
- **安装地址运行时推导。** `installMethods(origin, base)` 从
  `window.location.origin` 与 `import.meta.env.BASE_URL` 拼地址，
  换域名或换部署路径都不用改代码，也不会出现虚构的域名。
- **安装脚本不复制。** Vite 插件 `cmds-install-scripts` 直接读
  `install/` 下的源文件，开发服务器和构建产物都是同一份，不存在副本过期问题。

## 两侧刻意保持一致的规则

网站的价值在于「先试后装」，所以 Playground 的手感必须和真实 CLI 一致。
以下规则在两边各实现一次，并各有测试守着：

| 规则 | Rust | JS |
| --- | --- | --- |
| 内建命令清单 | `builtins.rs` 的 `BUILTINS` | `commands.js` 的 `builtins` |
| 编辑距离 | `util.rs` 的 `levenshtein` | `levenshtein` |
| 拼写建议排序 | `shell.rs` 的 `similar_commands` | `similarCommands` |
| 历史排序 | 频率 + 新鲜度 | `getSuggestions` 的 `weight` |
| ghost 建议 | `editor/suggest.rs` | `ghostSuggestion` |
| 配置模板 | `config/mod.rs` 的默认模板 | `buildConfigToml` |
| 界面语言 | `i18n.rs`（`t!` / `tf!`） | `i18n.js`（`t`） |

### 拼写建议的排序规则

```
编辑距离 → 候选类型（内建 0 > 别名/缩写 1 > PATH 2）
        → 首字母是否相同 → 长度差 → 字典序
```

只按「距离 + 字典序」排是不够的：输入 `hepl` 时，PATH 上的 `h2ph`、`head`、
`heap` 与内建 `help` 编辑距离都是 2，字典序还排在 `help` 前面，于是
`limit = 3` 时真正想要的 `help` 被挤出去了。把候选类型提到第二位就解决了——
内建命令和用户自己定义的别名，本来就比 PATH 上的随机同分命令更可能是意图。

`cli/src/shell.rs` 与 `web/tests/commands.test.js` 都有对应的回归测试。

## 测试策略

| 范围 | 工具 | 数量 |
| --- | --- | --- |
| CLI | `cargo test`（单元测试内嵌在各模块） | 109 |
| Playground 内核 | `node --test` | 42 |
| 安装脚本 | `sh -n` + shellcheck + PowerShell Parser | 语法级 |
| 默认语言 | `scripts/check-cli-language.sh`、`scripts/check-web-language.mjs` | 行为级 + AST |
| 端到端 | CI 里三平台跑 `--version` / `-c` / `init` / `prompt` 冒烟 | — |

**并行测试的坑：** `cd` 内建的测试会调用 `std::env::set_current_dir`，这是
进程级副作用。任何依赖相对路径的测试都必须自建临时目录并直接设置
`shell.cwd`，否则会随机失败。CI 里额外跑一次 `--test-threads=1` 做交叉验证。

**语言检查为什么要跑真实进程：** 只断言「英文环境下没有中文」，把中文全删掉也能
通过——那不是国际化，是砍功能。所以 `check-cli-language.sh` 同时断言中文环境下
中文还在，两个方向都不会悄悄烂掉。

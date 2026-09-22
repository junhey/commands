# 更新日志

本项目遵循 [语义化版本](https://semver.org/lang/zh-CN/)。

## [Unreleased]

## [0.2.0] - 2026-09-22

这一版只做一件事：**把「默认英文」从 README 扩展到整个产品**。

上一版只把 README 改成了英文，但 `curl -fsSL …/install.sh | sh` 装出来的东西
仍然是中文——CLI 输出、安装脚本、配置模板注释、官网界面全是中文。对非中文用户来说，
门面是英文、产品是中文，比全中文更让人困惑。

因为是用户可见行为的大改，按语义化版本升次版本号。

### 变更

- **CLI 默认英文。** 全部输出（`--help`、21 个内建命令、解析错误、补全候选标签、
  TOML 解析错误、集成脚本注释）改为默认英文，仅在 locale 要求中文时输出中文。
  语言由 `CMDS_LANG` > `LC_ALL` > `LC_MESSAGES` > `LANG` 决定，`C` / `POSIX` /
  未设置 / 空值都算英文
- **安装脚本默认英文。** `install.sh` 与 `install.ps1` 重写为双语，同样跟随环境；
  PowerShell 侧用 `Get-UICulture` 判断
- **配置模板默认英文。** `cmds config init` 生成的注释跟随语言，两份模板的键值
  严格一致（有测试钉住）
- **官网默认英文。** 跟随 `navigator.languages`，侧栏新增语言切换按钮，
  手动选择记在 localStorage
- 站点 release notes 与侧栏文案同步到 0.2.0

### 新增

- `CMDS_LANG` 环境变量：显式指定界面语言，优先级高于系统 locale。
  中文系统上想要英文界面（或反之）时用它。**CLI 与安装脚本认同一个变量名**——
  之前安装脚本支持、CLI 不支持，是被新加的检查脚本抓出来的
- `scripts/check-cli-language.sh`：跑真实进程做行为级验证，**双向**断言——
  英文环境下任何输出不得含中文，中文环境下中文必须还在。只查前者的话，
  把中文删干净也能「通过」，那不是国际化
- `scripts/check-web-language.mjs`：用 oxc 的真 AST 找站点源码里没走 `t()` 的
  中文字面量。刻意不手写词法分析——JSX 正文里的撇号（`Don't`）和正则字面量
  （`replace(/"/g, …)`）会让手写 tokenizer 状态错位，第一版就因此给出过一次假绿
- CI 三处新增语言约束：CLI job 跑行为探测、Web job 跑 AST 检查、
  安装脚本 job 直接跑 `install.sh --help` 比对两种语言
- CI 新增「宣传的测试数必须等于真实测试数」检查。站点和两份 README 都在写
  「N 个单元测试」，写死迟早变成假信息；数字现在只在 `BRAND.tests` 留一份
- README 新增「Language」章节说明语言规则；CONTRIBUTING 新增「界面语言：默认英文」
  一节，记下 `tf!` 与 `format!` 的关系、标点要算进文案、状态值不能用文案等几个坑
- `docs/architecture.md` 与 `docs/deployment.md` 改为英文默认，
  中文版移到 `.zh-CN.md`；两份 README 里补上入口（之前这两份文档没有任何链接指向它们）
- CLI 测试从 99 增至 109：locale 识别、`CMDS_LANG` 优先级、
  内建命令表两种语言都必须有说明、两份配置模板键值一致等

### 修复

- Playground 模拟输出里的版本号不再写死：`cargo build` / `cargo test` 的
  「Compiling cmds v…」与 `npm run dev` 的包名版本改为引用 `BRAND.version`，
  否则每次发版它们都会变成假信息（站点头部显示 0.1.2，演示里却还是 v0.1.0）
- `npm run dev` 的模拟输出里 Vite 版本从 `v6.1.0` 更新到 `v8.3.0`，
  依赖早升到 vite 8 了，演示没跟上
- `web/package.json` 的版本号与发布版本对齐
- 修掉一处潜在失效：`⌘1` 聚焦终端输入框用的是
  `querySelector('[aria-label="终端命令"]')`，而该 `aria-label` 已被 i18n 化，
  英文环境下会静默找不到元素。改为稳定的 `id="terminal-input"`
- 站点侧栏与 release notes 里写死的「99 个单元测试」已随检查一并纠正

## [0.1.2] - 2026-09-22

### 变更

- **README 默认改为英文**，中文版移到 `README.zh-CN.md`，两份顶部都有语言切换链接。
  英文那份同时是 GitHub 首页和 crates.io 页面的门面（`readme = "../README.md"`），
  之前是中文，对非中文读者不友好
- `cli/Cargo.toml` 的 `description` 也改成英文——crates.io 的搜索结果只显示这一句，
  中英混排会很割裂

### 新增

- 两份 README 都补齐了原先没写的内容：预编译平台对照表、卸载步骤、
  与 fish / starship / zsh-autosuggestions 的能力对比、五条常见问题，
  以及安装目录的实际选择规则（`/usr/local/bin` 可写时用它，否则 `~/.local/bin`）
- Release 资产里一并带上中文 README
- CI 新增 `文档一致性` job，三条约束：两份 README 必须互相链接、
  不得出现相对链接（crates.io 上会 404）、`##` / `###` 章节数必须相等。
  第二条以前只写在 CONTRIBUTING 里靠人记，现在是硬检查

## [0.1.1] - 2026-09-22

### 新增

- 发布到 crates.io，`cargo install --locked cmds` 可直接安装。
  `Release` 工作流打 tag 时自动发布，并在发布前校验 tag 与 `Cargo.toml` 版本一致；
  未配置 `CARGO_REGISTRY_TOKEN` 时跳过而不是让整条流水线失败
- README 加上 crates.io / CI / license 三个 badge

### 修复

- `install.ps1` 补上 SHA-256 校验。README、SECURITY.md 与站点都写了「两个安装脚本都会用
  同名 `.sha256` 校验」，但此前只有 `install.sh` 真的实现了，Windows 侧是文档与实现不符。
  现在会下载 `.sha256`、用 `Get-FileHash` 比对，不一致即中止；并新增 `-SkipVerify` 开关，
  与 `install.sh` 的 `--skip-verify` 对齐。安装脚本由站点直接提供，
  推送后即对 Windows 用户生效，不需要重新发版
- 打包给 crates.io 时会缺 LICENSE：registry 只打包 crate 目录内的文件。
  在 `cli/` 下放一份副本，并由 CI 校验两份一致
- README 里指向仓库文件的相对链接改为绝对 URL。
  crates.io 渲染 README 时相对链接会指向 crates.io 自身，点开是 404
- 安装脚本的源码回退改为优先走 crates.io（有版本语义、不必拉整个仓库），
  registry 上没有对应版本时再回退到 `--git`

### 其它

- CI 新增三项一致性检查：两个安装脚本必须同时具备校验逻辑与跳过开关；
  `cli/LICENSE` 与根 `LICENSE` 必须一致；`cargo publish --dry-run` 的打包内容
  必须包含 README 与 LICENSE。都是把文档里的承诺变成可检查的约束

## [0.1.0] - 2026-09-22

首个版本。由原先分开的两个原型（Rust 交互式 shell 与 React 演示站点）合并为一个产品：
CLI 是产品本体，站点是它的官网、Playground 与安装入口。

站点：<https://junhey.github.io/commands/>

### CLI (`cmds`)

- 交互式 shell：管道、`&&` / `||` / `;`、重定向、后台任务、glob、变量展开
- 历史自动建议：光标后灰色提示，`→` / `End` / `Ctrl-F` 采纳整条，`Alt-→` 采纳一个词
- `Tab` 候选菜单：历史整条命令最前，其后内建 / 别名 / 缩写 / PATH 命令 / 目录文件 / 变量
- 实时语法高亮，命令拼错时给出编辑距离建议
- 模块化提示符：`$dir` `$git_branch` `$git_status` `$languages` `$cmd_duration` `$status` `$character` `$time` `$identity` `$jobs`
- fish 式缩写（`abbr`），21 个内建命令
- 也可只作为提示符接入现有 shell：`cmds init bash|zsh|fish|powershell`
- TOML 配置，`config reload` 热重载
- 依赖只有 `crossterm` 与 `unicode-width`（Unix 下另加 `libc`）

### 官网与 Playground

- 浏览器内的模拟终端，规则与 CLI 对齐（内建命令清单、候选排序、拼写建议算法、历史权重）
- History / Snippets 页，数据只存在浏览器 localStorage
- **Config 页**：勾选提示符模块即时生成可用的 `config.toml`，支持复制与下载
- Integrations 页给出四种 shell 的 `init` 接入片段
- 使用指南覆盖安装、输入与补全、快捷键、配置、集成、隐私
- 安装地址由运行时 origin + base 推导，支持根域名与子路径两种部署

### 安装

- `install.sh` / `install.ps1` 自动识别平台，优先下载预编译二进制，无对应包时回退 `cargo` 构建
- **下载后用同名 `.sha256` 校验**，校验失败即中止（`--skip-verify` 可跳过）
- 用户级安装，不需要 sudo，不改动 shell 启动文件
- 脚本随站点一起发布在根目录，安装前可直接查看原文

### 修复

- 拼写建议排序：原先只按「编辑距离 + 字典序」排，`hepl` 会被 PATH 里的
  `h2ph` / `head` / `heap` 挤掉真正想要的内建 `help`。现在改为
  编辑距离 → 候选类型（内建 > 别名缩写 > PATH）→ 首字母是否相同 → 长度差 → 字典序，
  并且不再把用户输入的原词当作建议
- 修复测试并行时的随机失败：路径补全的测试依赖进程当前目录，而 `cd`
  内建的测试会调用 `std::env::set_current_dir`。改为使用独立临时目录
- 修复 clippy `field_reassign_with_default` 告警，CI 可以用 `-D warnings` 把关
- `vite.config.js` 的 `manualChunks` 改为函数形式。Vite 8 底层换成 rolldown，
  不再接受对象形式，否则构建直接抛 `manualChunks is not a function`
- `Github` 图标改为内联官方 Octicon 的 `mark-github` 路径。
  lucide-react v1 移除了全部品牌 logo，没有内置替代
- `release.yml`：不带 tag 的手动触发不再让发布步骤报错。
  之前会一路跑到 `GitHub Releases requires a tag` 才失败，
  现在无 tag 时只构建验证、跳过发布，也支持通过输入指定要发布的 tag

### 工程

- CI：三平台 × (fmt / clippy / test / 单线程复跑 / release 构建 / 冒烟测试)，
  web 测试与构建，安装脚本 `sh -n` + shellcheck + PowerShell 语法检查
- Release：六个平台的预编译二进制 + `.sha256` + `SHA256SUMS`
- GitHub Pages 部署工作流
- Dependabot 每周检查 cargo / npm 依赖
- 依赖基线：`vite` 8、`@vitejs/plugin-react` 6、`lucide-react` 1、
  GitHub Actions（`checkout` 7 / `upload-artifact` 7 / `download-artifact` 8 /
  `configure-pages` 6 / `deploy-pages` 5）

[unreleased]: https://github.com/junhey/commands/compare/v0.1.2...HEAD
[0.1.2]: https://github.com/junhey/commands/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/junhey/commands/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/junhey/commands/releases/tag/v0.1.0

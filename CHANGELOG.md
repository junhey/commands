# 更新日志

本项目遵循 [语义化版本](https://semver.org/lang/zh-CN/)。

## [Unreleased]

### 新增

- **提示符补上 6 个模块，信息量对齐 starship。** 新增 `$container`、`$git_state`、
  `$git_commit`、`$venv`、`$package`、`$os`、`$shlvl`，共 20 个模块：

  ```
  root in 🌐 joonhe-1rnzdldo2a in wetv-monorepo on  chore/ci-queue via  v20.20.2 took 4s
  ⬢ [Docker] ❯
  ```

  - `$container` 识别 Docker / Podman / Dev Container / Codespaces / WSL，显示
    `⬢ [Docker]`。只读环境变量和标记文件，不 fork 进程——提示符每次回车都要渲染，
    `systemd-detect-virt` 那种调用的代价不划算。识别不出来的环境可以用
    `CMDS_CONTAINER=名字` 手动标注。显式变量刻意排在文件探测之前：Codespaces 和
    devcontainer 内部同样有 `/.dockerenv`，顺序反了会一律显示成 Docker。
  - `$git_state` 显示 `REBASE 2/5` / `MERGING` / `CHERRY-PICKING` / `BISECTING`。
    只看分支名永远不知道自己正卡在 rebase 中间。
  - `$package` 读 package.json / Cargo.toml / pyproject.toml 的版本号。读文件而不是
    调包管理器：`npm version` 那类命令要几百毫秒，提示符等不起。
  - `$venv` 在虚拟环境目录叫 `.venv` / `venv` / `env` 时往上取一层项目名——
    一排 `(.venv)` 是没有信息量的。

- **用户名与主机名改为三态可见性。** `show_user` / `show_host` 现在接受
  `true` / `false` / `"auto"`，默认 `"auto"`：只在 root、SSH 会话或容器里显示。
  本地日常开发提示符保持干净，进了容器或远程机器自动带上身份。root 单独用红色。
  原来写 `true` / `false` 的配置照旧生效。

- **`$all` 占位符。** 展开为第一行的整个信息区。配置里写死模块列表的话，以后版本
  新增的模块永远不会出现（旧配置会盖掉新默认值），`format = "$all$line_break$character"`
  可以一直跟上。默认模板已改用它。

- **子命令补全：输入 `git ` 直接给出 `status` / `log` / `commit`。** 内置 17 张命令表
  （git、cargo、npm、pnpm、yarn、docker、kubectl、go、brew、systemctl…），含
  `git remote`、`docker compose` 这类二级命令，每项都带双语说明。匹配要求整条命令链
  精确命中，所以 `git status ` 之后不会再把一级子命令列一遍。可在 `[completions]`
  里补充或覆盖，命令链带空格时用引号键（`[completions."git remote"]`）。

- **项目脚本补全。** `npm run ` / `pnpm ` / `yarn ` 补全最近 `package.json` 里的
  `scripts`（monorepo 里会向上找，但到仓库根就停）；`make ` 补全 Makefile 的 target，
  并把 `target: ## 说明` 的自文档注释当作描述。这些名字只有项目自己知道，
  又最常用，是补全最该帮忙的地方。

- **`[scripts]` 预置脚本。** 名字和命令本身都参与匹配，所以 `gst` 和 `git st` 都能
  找到 `git status --short --branch`——只认缩写的话，得先记住缩写才用得上，
  那就失去意义了。模板预置 6 条，刻意都不带破坏性操作（`gclean` 是 dry-run），
  有测试钉住这一点。

- `config show` 增加预置脚本数、内置命令表数与全部可用模块名——想改 `format`
  不用再翻文档查占位符叫什么。

### 变更

- 默认 `format` 改为 `$all$line_break$container$shlvl$character`，`$languages`
  新增 `prefix`（默认 `via `），与示例中的 `via  v20.20.2` 一致。前缀跟内容一起
  出现，检测不到语言时不会留下孤立的 `via`。
- 单元测试 109 → 157。

## [0.2.1] - 2026-09-22

### 修复

- **Linux 二进制改为静态链接 musl，修掉「装完跑不起来」。** 此前 Linux 包动态链接
  构建机（`ubuntu-latest` = Ubuntu 24.04，glibc 2.39）的 glibc，产物把
  `GLIBC_2.39` 写进了 ELF 版本需求，于是在 Debian 10/11、Ubuntu 18.04~22.04、
  CentOS 7/8 以及大量开发容器上，一键安装装完就是

  ```
  cmds: /lib64/libc.so.6: version `GLIBC_2.29' not found (required by cmds)
  ```

  实际效果是「只有最新 Ubuntu 能用」，而 CI 一路绿灯——因为 CI 就跑在构建机上，
  这类问题它天然测不出来。

  改成 musl 静态链接后产物零动态依赖，任何 Linux 都能跑（含 Alpine）。
  能这么做的前提是项目没有任何 C 依赖：crossterm / unicode-width / libc 全是纯
  Rust，libc 只用到 `signal()`，用户名取自环境变量而非 NSS 查询——所以不存在
  静态 musl 的 NSS 陷阱。也因此用 `rust-lld` 配 rustup 自带的 musl std 就够，
  不需要 `musl-tools`，连 aarch64 的交叉工具链都省了。

  预编译目标名随之改变（`*-unknown-linux-gnu` → `*-unknown-linux-musl`）。
  `install.sh` 会先试 musl、失败再回退 gnu，所以 `--version v0.2.0` 这类指定
  旧版本的安装仍能匹配到那些 Release 实际发布的资产。

- 修掉两处 `$VAR` 紧跟全角标点的写法（`check-cli-language.sh` 与新脚本各一处）。
  全角字符的首字节会被 shell 当成变量名的一部分，`set -u` 下直接
  `unbound variable`，没有 `set -u` 时更糟——静默展开成空字符串。

### 新增

- `scripts/check-linux-portability.sh`：两层验证。① 静态断言——产物不得有动态
  依赖、不得引用 glibc 符号版本；② 行为验证——在 `debian:10`（glibc 2.28，
  正是用户报错那一档）、`centos:7`（2.17）、`alpine`（无 glibc）里真的把产物跑起来。
  第一层只能说明「没链错」，第二层才是用户真正关心的事。
  反向验证过：拿 v0.2.0 的旧产物跑，三条断言全部报红并指出 `最高 GLIBC_2.39`。
- CI 新增 `Linux 可移植性` job 调用上面的脚本；Release 工作流在打包前也跑同一个
  脚本，坏产物流不出去。
- `scripts/check-shell-quoting.sh`：揪出 `$VAR` 紧跟多字节字符的写法。
  这个坑在本仓库踩过三次，而 shellcheck 认为它语法合法、查不出来，所以做成检查
  而不是继续靠记性。

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
- `scripts/check-cli-language.sh` 与 `scripts/check-install-language.sh`：
  跑真实进程做行为级验证，**双向**断言——英文环境下任何输出不得含中文，
  中文环境下中文必须还在。只查前者的话，把中文删干净也能「通过」，
  那不是国际化。这个双向设计当场就救了一次：CI 上 grep 因 locale 问题恒为假，
  是中文方向报红才暴露出来的，否则会是一片假绿
- `scripts/cjk.sh`：中文检测的公共实现，用 Perl 的 `\p{Han}`。
  **不要用 `grep '[一-龥]'`**——字符区间受 collation 影响，`LC_ALL=C` 下
  GNU grep 报 `Invalid collation character` 并以错误码退出，所有断言恒为假；
  macOS 的 BSD grep 恰好能跑，所以本地看不出来。这个文件在被引用时会用
  一个正例 + 一个反例自检，不对就直接退出，宁可不检查也不给不可信的结论
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

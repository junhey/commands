# 更新日志

本项目遵循 [语义化版本](https://semver.org/lang/zh-CN/)。

## [Unreleased]

### 修复

- `install.ps1` 补上 SHA-256 校验。README、SECURITY.md 与站点都写了「两个安装脚本都会用
  同名 `.sha256` 校验」，但此前只有 `install.sh` 真的实现了，Windows 侧是文档与实现不符。
  现在会下载 `.sha256`、用 `Get-FileHash` 比对，不一致即中止；并新增 `-SkipVerify` 开关，
  与 `install.sh` 的 `--skip-verify` 对齐。安装脚本由站点直接提供，
  推送后即对 Windows 用户生效，不需要重新发版
- CI 新增一致性检查：两个安装脚本必须同时具备校验逻辑与跳过开关，
  避免同类「只改了一半」的问题再次溜过

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

[unreleased]: https://github.com/junhey/commands/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/junhey/commands/releases/tag/v0.1.0

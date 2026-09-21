# 安全策略

## 支持范围

安全修复只针对最新的 `0.1.x`。请先升级到最新版本再反馈。

## 报告漏洞

**不要**用公开 issue 报告安全问题。请通过 GitHub 的
[Security Advisory](https://github.com/junhey/commands/security/advisories/new)
私下提交，包含：

- 影响的版本与平台
- 复现步骤或 PoC
- 你评估的影响面

## 设计上的安全边界

了解这些边界有助于判断某个行为是不是漏洞：

**CLI（`cmds`）**

- 作为交互式 shell，它会按用户输入执行本机命令，这是预期功能而非漏洞。
- 历史文件在 Unix 下以 `0600` 创建。以空格开头的命令不写入历史。
- 不发起任何网络请求，不做遥测，不上传命令或历史。
- 会读取 `~/.config/cmds/config.toml` 与启动脚本（`~/.cmdsrc`）。这些文件由用户自己掌控；启动脚本里的内容会被执行，这一点与其它 shell 的 rc 文件一致。
- 提示符集成脚本（`cmds init <shell>`）输出的内容会被 `eval`，其中的可执行文件路径来自 `current_exe()`。

**官网与 Playground**

- Playground 的命令**全部是模拟的**：只做查表与字符串处理，不执行代码、不读本地文件、不发网络请求。
- 历史、片段、偏好只写入浏览器 localStorage，没有服务端。
- 站点是纯静态的，没有后端、没有数据库、没有账号体系。

**安装脚本**

- `install.sh` / `install.ps1` 下载 GitHub Releases 上的资产，并用同名 `.sha256` 校验；校验不通过会中止安装。
- 不需要 sudo（除非用户显式指定了需要提权的安装目录），不修改任何 shell 启动文件。
- 强烈建议在执行前先审阅脚本内容，站点上的安装对话框提供了直接查看链接。

## 依赖

CLI 的运行时依赖只有 `crossterm` 与 `unicode-width`（Unix 下另加 `libc`）。
依赖更新由 Dependabot 每周检查。

# 参与开发

## 仓库结构

```
cli/            Rust 实现的 cmds 二进制（产品本体）
  src/          行编辑器、提示符、执行引擎、配置、历史
  assets/       默认配置模板
web/            官网 + 浏览器 Playground（Vite + React）
  src/commands.js   Playground 内核，规则与 cli/ 保持一致
  src/install.js    安装命令生成
install/        一键安装脚本（sh / ps1），站点根目录直接提供
docs/           面向使用者的补充文档
```

根目录是一个 Cargo workspace，`cargo` 命令在根目录执行即可。

## 环境要求

- Rust 1.85+（`rustup` 安装即可，edition 2024）
- Node.js 20.19+ 或 22+

## 常用命令

```sh
# CLI
cargo test                      # 单元测试
cargo clippy --all-targets -- -D warnings
cargo fmt --all
cargo run -- -c "echo hello"    # 跑一条命令
cargo run                       # 进交互式

# 站点
cd web
npm install
npm run dev                     # 本地开发
npm test                        # Playground 内核测试
npm run build                   # 静态产物到 web/dist

# 安装脚本
sh -n install/install.sh
```

## 提交前自查

CI 会跑这些，本地先过一遍能省一轮往返：

```sh
cargo fmt --all --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
cargo test --locked -- --test-threads=1
cd web && npm test && npm run build
sh -n install/install.sh
```

## 写测试时的两个坑

1. **不要依赖进程当前目录。** `cd` 内建的测试会调用
   `std::env::set_current_dir`，这是进程级的。任何依赖相对路径的测试都要自己建临时目录并直接设置
   `shell.cwd`，否则并行跑测试时会随机失败。参考
   `cli/src/editor/complete.rs` 里的 `completes_paths`。
2. **`Config::default()` 后不要逐字段赋值。** clippy 的
   `field_reassign_with_default` 会报错，用结构体字面量加 `..Default::default()`。

## 两侧规则要保持一致

`web/src/commands.js` 是 Playground 的内核，它刻意复刻了 CLI 的几条规则：

| 规则 | Rust 侧 | JS 侧 |
| --- | --- | --- |
| 内建命令清单 | `cli/src/builtins.rs` 的 `BUILTINS` | `builtins` |
| 拼写建议排序 | `cli/src/shell.rs` 的 `similar_commands` | `similarCommands` |
| 编辑距离 | `cli/src/util.rs` 的 `levenshtein` | `levenshtein` |
| 历史排序 | 频率 + 新鲜度 | `getSuggestions` 的 `weight` |

**改了一侧就要同步另一侧**，否则网站演示的手感会和真实体验脱节。两侧都有对应的测试。

## 定位边界

cmds 面向**交互使用**。以下不在范围内：

- 函数、`if` / `for` / `while` 等脚本语法
- 成为 POSIX sh 的替代品
- 任何形式的遥测、云同步、账号体系

## 发布

1. 更新 `CHANGELOG.md`
2. 改 `Cargo.toml` 的 `workspace.package.version` 与 `web/src/commands.js` 里的 `BRAND.version`
3. `cargo build --locked` 刷新 `Cargo.lock`
4. 打标签 `git tag -a v0.1.1 -m "..." && git push origin v0.1.1`

打 tag 后 `Release` 工作流会做三件事：

- 构建六个平台的二进制，附上 `.sha256` 与汇总的 `SHA256SUMS`
- 创建 GitHub Release
- 发布到 crates.io（需要仓库 Secret `CARGO_REGISTRY_TOKEN`；没配就跳过并给出警告）

发布前工作流会校验 **tag 版本号与 `Cargo.toml` 一致**，不一致直接失败。
crates.io 的版本不可撤回，所以只有真正打 tag 才会发 registry，
不带 tag 的手动触发只做构建验证。

### crates.io 相关的两个约束

- **`cli/LICENSE` 必须与根目录 `LICENSE` 一致。** crates.io 只打包 crate 目录内的文件，
  所以 `cli/` 下要自带一份。改许可证时记得 `cp LICENSE cli/LICENSE`，CI 会校验。
- **README 里不要用相对链接。** crates.io 渲染 README 时相对链接会指向 crates.io 自身，
  点开是 404。指向仓库文件请写完整的 `https://github.com/junhey/commands/blob/master/...`。

### 首次发布 crates.io 的前置条件

除了仓库 Secret `CARGO_REGISTRY_TOKEN`，**crates.io 账号必须已验证邮箱**，
否则上传会被拒：

```
error: failed to publish cmds to registry at https://crates.io
Caused by:
  the remote server responded with an error (status 400 Bad Request):
  A verified email address is required to publish crates to crates.io.
```

到 <https://crates.io/settings/profile> 填邮箱并点开验证邮件即可。
这一步只需做一次，`cargo publish --dry-run` 检查不到它（dry-run 不触达服务端校验）。

补救方式：验证邮箱后**单独重跑失败的那个 job**，不用重新打 tag——
GitHub Release 与预编译二进制已经产出，`crates` job 是独立的：

```sh
gh run rerun <run-id> --job <job-id>
```

发布同一版本两次会被 registry 拒绝（`already exists`），工作流已把这种情况视为成功，
所以重跑是安全的。

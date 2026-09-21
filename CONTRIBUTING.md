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
4. 打标签 `git tag v0.1.1 && git push --tags`

Release 工作流会构建六个平台的二进制，附上 `.sha256`，并生成 Release 说明。

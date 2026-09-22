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

## 双语 README

`README.md` 是**英文**版，`README.zh-CN.md` 是中文版。英文那份同时被 GitHub 首页和
crates.io 页面使用（`cli/Cargo.toml` 的 `readme = "../README.md"`），所以它是默认门面。

三条规则，CI 的 `文档一致性` job 会逐条检查：

1. **两份必须互相链接。** 顶部各有一行语言切换，指向对方的完整 GitHub URL。
2. **不要用相对链接。** crates.io 渲染 README 时相对链接会解析到 crates.io 自身，点开是
   404。指向仓库文件请写完整的 `https://github.com/junhey/commands/blob/master/...`。
3. **章节要一一对应。** 两份的 `##` / `###` 标题数量必须相等，改了一侧就翻译到另一侧，
   不要只动一边。

`cli/Cargo.toml` 的 `description` 也保持英文——crates.io 的搜索结果只显示这一句。

## 界面语言：默认英文

**任何用户可见的文案都不许直接写中文。** CLI 输出、安装脚本、配置模板注释、站点 UI
都默认英文，只有环境明确要求（`CMDS_LANG` > `LC_ALL` > `LC_MESSAGES` > `LANG`，或浏览器
语言）才切中文。

中英两份文案写在同一行，而不是拆成资源文件：

```rust
io.err(t!("no such directory", "目录不存在"));
io.err(tf!("cannot read {}", "无法读取 {}", path));
```

```jsx
<span>{t("Install cmds", "安装 cmds")}</span>
```

这么做的理由是文案量小，把两种语言摆在眼前，改的时候不会只改一半——引入 gettext
那一套反而更容易出现「英文改了、中文还是旧的」。

几条容易踩的：

- **`format!` 的格式串必须是字面量**，所以要格式化时用 `tf!` 而不是先 `t!` 拿格式串。
- **标点也要算进文案里。** `msg "platform: " "平台："` 是对的；把 `：` 留在外面，
  英文输出就会变成 `platform：`。
- **状态值不要用文案。** 筛选项、tab 标识这类东西用稳定 id，用文案的话切换语言后
  选中项会丢。同理，不要用 `aria-label` 之类会翻译的文本当 `querySelector` 的钩子。
- **测试断言不要依赖环境语言。** 用 `t()` 算出期望值，或显式固定语言，否则本地
  `zh_CN` 跑过、CI 上 `LANG=en` 就红。
- **刻意的双语数据表**（如 `[名称, 英文说明, 中文说明]`）用
  `i18n-pairs-begin` / `i18n-pairs-end` 注释标出范围，检查脚本会跳过。

CI 有三道检查，本地也可以跑：

```sh
sh scripts/check-cli-language.sh              # 跑真实二进制：英文环境无中文，中文环境仍有中文
sh scripts/check-install-language.sh          # 安装脚本，不需要先构建
node scripts/check-web-language.mjs web/src/*.js web/src/*.jsx   # AST 找裸中文
```

两条设计原则，都是被真实故障逼出来的，改这些脚本时请保留：

1. **双向断言。** 只查「英文环境没有中文」的话，把中文全删了也能过——那不是
   国际化，是砍功能。所以同时断言「中文环境下中文必须还在」。
2. **检测器自己要先自检。** 检测器坏掉比漏检更糟：会给出「通过」的假绿。
   `scripts/cjk.sh` 在被引用时会用一个正例 + 一个反例验自己，不对就以退出码 2
   拒绝给出结论。

具体踩过的：**不要用 `grep '[一-龥]'` 判断中文。** 字符区间受 collation 影响，
在 `LC_ALL=C` 下 GNU grep 直接报 `Invalid collation character` 并以错误码退出，
于是所有断言恒为假、CI 全绿而实际什么都没查。macOS 的 BSD grep 恰好能跑，本地
看不出来。现在统一用 `scripts/cjk.sh` 里的 `has_cjk`（Perl 的 `\p{Han}`，
与 locale 无关，也不会像手写区间那样漏掉扩展区）。

## 发布产物：Linux 必须是 musl 静态

**不要把 Linux 目标改回 `-gnu`。** 动态链接的产物会把**构建机**的 glibc 版本写进
ELF 版本需求。`ubuntu-latest` 是 24.04（glibc 2.39），于是产物要求 `GLIBC_2.39`，
在 Debian 10/11、Ubuntu 18.04~22.04、CentOS 7/8 和大多数开发容器上装完直接报
`version 'GLIBC_2.29' not found`。

**CI 自己测不出这个**——它就跑在构建机上。所以有两道检查：

```sh
# 静态断言 + 在 debian:10 / centos:7 / alpine 里实跑（有 docker 才跑第二层）
sh scripts/check-linux-portability.sh

# $VAR 紧跟全角标点会吃掉变量名一个字节，shellcheck 查不出来
sh scripts/check-shell-quoting.sh install/install.sh scripts/*.sh
```

能用 musl 静态的前提是**项目没有任何 C 依赖**：crossterm / unicode-width / libc
全是纯 Rust，libc 只用到 `signal()`，用户名取自环境变量而不是 NSS 查询。
如果以后引入了需要 NSS（`getpwuid`、`getaddrinfo`）或要编译 C 代码的依赖，
静态 musl 的限制就要重新评估。

改动 `release.yml` 的目标名时，`install.sh` 的 `detect_targets` 要同步——
CI 有断言钉住这两者一致。

上面两道检查验的都是**本次构建**的产物，而用户走的是另一条路：线上的
`install.sh` 去下载线上 Release 的资产。中间那一段（脚本挑的资产名对不对、
解压后能不能跑、挑错了会不会静默退化成源码构建）由
`.github/workflows/e2e-install.yml` 负责，它在 Debian 10/9、CentOS 7、Alpine
四个容器里真的敲一遍 `curl … | sh`。断言逻辑在
`scripts/check-e2e-install.sh`，本地拿构造的假数据就能反向验证：

```sh
# 工作流把容器输出丢进 e2e-out/，这个脚本只读文件下结论，不碰网络和 docker
sh scripts/check-e2e-install.sh e2e-out
```

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

5. **发版后手动触发一次 `端到端安装验证`**（Actions → 端到端安装验证 → Run）

这一步验的是「用户真正会敲的那条命令现在能不能用」，跑的是**线上**的
`install.sh` 和**线上** Release 的资产，所以只能在发布完成之后跑，
挂在 push / PR 上没有意义（那时候线上还是旧版本）。
这个工作流另外每周一自己跑一遍，线上哪天悄悄坏掉能在用户报障前先知道。

### crates.io 相关的两个约束

- **`cli/LICENSE` 必须与根目录 `LICENSE` 一致。** crates.io 只打包 crate 目录内的文件，
  所以 `cli/` 下要自带一份。改许可证时记得 `cp LICENSE cli/LICENSE`，CI 会校验。
- **README 里不要用相对链接**，详见上面的「双语 README」。crates.io 上点开会 404。

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

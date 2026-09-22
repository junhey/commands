#!/bin/sh
# 验证「线上一键安装」这条路径真的能用——
#
#   curl -fsSL https://junhey.github.io/commands/install.sh | sh
#
# 为什么需要它：CI 的可移植性检查验的是**本次构建**的产物在旧发行版能跑，
# 而用户走的是另一条路——线上的 install.sh 去下载线上 Release 的资产。
# 中间这一段（脚本挑的资产名对不对、解压后能不能跑、挑错了会不会静默
# 退化成源码构建）以前没人看着，v0.2.0 的 GLIBC_2.29 事故就出在这里，
# 当时 CI 还是全绿的——因为 CI 就跑在构建机上。
#
# 这个脚本只做断言，不碰网络也不碰 docker：容器跑完把输出丢进一个目录，
# 它读那些文件下结论。这样断言逻辑能在本地拿构造的假数据反向验证，
# 不用等 CI（.github/workflows/e2e-install.yml 负责产出这些文件）。
#
# 期望的输入文件：
#   <目录>/install.log     安装全过程（含 stderr），默认语言环境
#   <目录>/install.zh.log  安装全过程，LANG=zh_CN.UTF-8
#   <目录>/version.txt     `cmds --version` 的 stdout
#   <目录>/behavior.txt    `cmds -c 'echo e2e-ok'` 的 stdout
#
# 用法：scripts/check-e2e-install.sh [输出目录]   （默认 e2e-out）

set -eu

ROOT=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
# shellcheck source=scripts/cjk.sh
. "$ROOT/scripts/cjk.sh"

OUT=${1:-e2e-out}

if [ ! -d "$OUT" ]; then
	echo "找不到输出目录：$OUT" >&2
	exit 2
fi

LOG="$OUT/install.log"
ZH_LOG="$OUT/install.zh.log"

fail=0
pass() { printf '  \033[32m✓\033[0m %s\n' "$1"; }
bad() {
	printf '  \033[31m✗\033[0m %s\n' "$1"
	fail=1
}

# 安装日志是所有结论的地基，空的话后面每条断言都没有意义，直接退出，
# 免得输出一屏「失败」让人以为是产物有十个毛病。
if [ ! -s "$LOG" ]; then
	echo "$LOG 不存在或为空：安装根本没跑起来，先看容器日志。" >&2
	exit 1
fi

# ① 用户报的那个错不能再出现。这条是这个脚本存在的理由。
if grep -q 'GLIBC_' "$LOG"; then
	bad "输出里出现了 GLIBC_ 字样（旧发行版跑不起来的那个错又回来了）"
	grep -n 'GLIBC_' "$LOG" | head -5 | sed 's/^/      /'
else
	pass "没有 GLIBC_xx not found"
fi

# ② 装到位了。
if grep -q 'installed to' "$LOG"; then
	pass "安装完成（installed to）"
else
	bad "没看到 installed to，安装没走完"
fi

# ③ 必须走预编译包。资产名一对不上，install.sh 会静默退化成 cargo 源码构建：
#    容器里没有 Rust 会直接失败，就算宿主上碰巧成功了也说明平台映射错了。
if grep -q 'building from source' "$LOG"; then
	bad "退化成了源码构建，说明没匹配到预编译资产（平台映射和 Release 资产名不一致）"
else
	pass "用的是预编译包，没退化成源码构建"
fi

# ④ Linux 上应当选中 musl。gnu 只是留给「指定安装 v0.2.0 这类旧版本」的回退，
#    正常路径不该走到它——走到了就说明 musl 资产没发布成功。
if grep -q 'musl' "$LOG"; then
	pass "选中的是 musl 静态包"
else
	bad "没选中 musl（看安装日志里的 platform 行）"
fi

# ⑤ 装完要真的能跑。这正是 GLIBC 事故当年缺的那一环：
#    文件下下来了、权限也对，一执行就 version GLIBC_2.29 not found。
version=$(cat "$OUT/version.txt" 2>/dev/null || true)
case "$version" in
"cmds "*) pass "cmds --version → $version" ;;
"") bad "cmds --version 没有输出（二进制没跑起来）" ;;
*) bad "cmds --version 输出不像版本号（实际：${version}）" ;;
esac

# 只看退出码的话，输出为空也算「通过」，所以这里要求严格相等。
behavior=$(cat "$OUT/behavior.txt" 2>/dev/null || true)
if [ "$behavior" = e2e-ok ]; then
	pass "cmds -c 'echo e2e-ok' → e2e-ok"
else
	bad "命令执行结果不对（期望 e2e-ok，实际：${behavior}）"
fi

# ⑥ 语言要双向断言。只查「英文环境没中文」是不够的——把中文删光也能过，
#    那是砍功能不是国际化。所以中文环境必须还说中文。
if file_has_cjk "$LOG"; then
	bad "默认语言环境下的安装输出出现了中文"
	printf '      %s\n' "$(first_cjk_line "$(cat "$LOG")")"
else
	pass "默认语言环境下输出全英文"
fi

if [ ! -s "$ZH_LOG" ]; then
	bad "$ZH_LOG 不存在或为空，中文方向没验到"
elif file_has_cjk "$ZH_LOG"; then
	pass "LANG=zh_CN.UTF-8 下仍然说中文"
else
	bad "LANG=zh_CN.UTF-8 下没切中文（i18n 可能被删成纯英文了）"
fi

if [ "$fail" != 0 ]; then
	echo "端到端安装验证失败。" >&2
	exit 1
fi

echo "端到端安装验证通过：一键安装可用，装完能跑，语言正确。"

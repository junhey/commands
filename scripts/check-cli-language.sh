#!/bin/sh
# 验证「默认英文」在真实输出里成立——不是看源码，而是真的跑一遍。
#
# 两个方向都要查，缺一个都不算数：
#
#   1. 英文环境（locale 未设置 / C）下，任何输出都不许出现中文。
#   2. 中文环境（LANG=zh_CN.UTF-8）下，中文必须还在。
#
# 只查方向 1 的话，把所有中文删掉就能「通过」——那不是国际化，是砍功能。
# 方向 2 把这条路堵死。（这个双向设计当场救过一次：grep 在 C locale 下对中文
# 区间报错、恒为假，方向 1 全部「通过」，是方向 2 报红才暴露出来的。）
#
# 安装脚本的语言检查在 scripts/check-install-language.sh，不需要二进制，单独跑。
#
# 用法：scripts/check-cli-language.sh [cmds 可执行文件路径]
# 默认用 workspace 的 target/debug/cmds（注意是仓库根的 target/，不是 cli/target/）。

set -eu

ROOT=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
# shellcheck source=scripts/cjk.sh
. "$ROOT/scripts/cjk.sh"

BIN=${1:-"$ROOT/target/debug/cmds"}

if [ ! -x "$BIN" ]; then
	echo "找不到可执行文件：$BIN" >&2
	echo "先运行：cargo build" >&2
	exit 2
fi

failures=0

# 英文环境：清掉全部 locale 变量，再把 LC_ALL 钉成 C。
en() {
	env -u LANG -u LC_MESSAGES -u LC_CTYPE -u CMDS_LANG LC_ALL=C "$@" 2>&1 || true
}

zh() {
	env -u LC_ALL -u LC_MESSAGES -u CMDS_LANG LANG=zh_CN.UTF-8 "$@" 2>&1 || true
}

fail() {
	failures=$((failures + 1))
	printf '  ✗ %s\n' "$1" >&2
}

# expect_en <描述> <参数...> —— 英文环境下输出不许有中文
expect_en() {
	desc=$1
	shift
	out=$(en "$BIN" "$@")
	if has_cjk "$out"; then
		fail "$desc：英文环境下出现中文 → $(first_cjk_line "$out")"
	fi
}

# expect_en_c <命令串> —— 同上，走 -c
expect_en_c() {
	out=$(en "$BIN" -c "$1")
	if has_cjk "$out"; then
		fail "-c '$1'：英文环境下出现中文 → $(first_cjk_line "$out")"
	fi
}

echo "英文环境：检查 CLI 参数与子命令"
expect_en "--help" --help
expect_en "-h" -h
expect_en "--version" --version
expect_en "未知参数" --bogus-flag
expect_en "init bash" init bash
expect_en "init zsh" init zsh
expect_en "init fish" init fish
expect_en "init powershell" init powershell
expect_en "init 未知 shell" init tcsh
expect_en "init 缺参数" init
expect_en "prompt" prompt
expect_en "prompt --status 1" prompt --status 1
expect_en "不存在的脚本" /nonexistent-script-xyz.cmds
expect_en "-c 缺命令" -c

echo "英文环境：检查内建命令输出"
for cmd in \
	'help' 'config show' 'config path' 'config bogus' \
	'type ls' 'type help' 'type nonexistent-xyz' 'type' \
	'jobs' 'history' 'history search zzzz' 'history -x' \
	'cd -' 'cd /nonexistent-dir-xyz' 'exit abc' \
	'alias' 'alias =bad' 'unalias' 'unalias nope' \
	'abbr' 'unabbr nope' 'unset' 'export' \
	'source' 'source /nonexistent-xyz' \
	'echo hi' 'pwd' 'true' 'false' 'nonexistent-command-xyz'; do
	expect_en_c "$cmd"
done

echo "英文环境：检查解析错误"
for bad in \
	'echo "unterminated' 'echo |' 'echo &&' 'echo >' 'ls [a-' \
	'echo `' 'a=' '| echo' 'echo ;;'; do
	expect_en_c "$bad"
done

echo "英文环境：检查配置模板"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

out=$(env -u LANG -u LC_MESSAGES -u LC_CTYPE -u CMDS_LANG LC_ALL=C \
	CMDS_CONFIG="$tmp/en.toml" "$BIN" -c 'config init' 2>&1 || true)
if has_cjk "$out"; then
	fail "config init：英文环境下提示含中文"
fi
if [ -f "$tmp/en.toml" ] && file_has_cjk "$tmp/en.toml"; then
	fail "config init：英文模板文件里含中文"
fi

echo "中文环境：确认中文没有被砍掉"
# 这里反过来：没有中文才是 bug。选几处最有代表性的。
# 刻意不查 `type ls` 之类：输出依赖 PATH 里有什么，命中与否不稳定。
zh_has() {
	out=$(zh "$BIN" "$@")
	if ! has_cjk "$out"; then
		fail "中文环境下 $* 没有输出中文（i18n 可能被误删）"
	fi
}
zh_has --help
zh_has -c 'help'
zh_has -c 'nonexistent-command-xyz'

env -u LC_ALL -u LC_MESSAGES -u CMDS_LANG LANG=zh_CN.UTF-8 \
	CMDS_CONFIG="$tmp/zh.toml" "$BIN" -c 'config init' >/dev/null 2>&1 || true
if [ -f "$tmp/zh.toml" ] && ! file_has_cjk "$tmp/zh.toml"; then
	fail "config init：中文环境下模板没有中文注释"
fi

echo "强制语言：CMDS_LANG 必须能盖过 locale"
out=$(env -u LC_ALL -u LC_MESSAGES LANG=zh_CN.UTF-8 CMDS_LANG=en "$BIN" --help 2>&1 || true)
if has_cjk "$out"; then
	fail "CMDS_LANG=en 没能盖过 LANG=zh_CN → $(first_cjk_line "$out")"
fi
out=$(env -u LANG -u LC_MESSAGES -u LC_CTYPE LC_ALL=C CMDS_LANG=zh "$BIN" --help 2>&1 || true)
if ! has_cjk "$out"; then
	fail "CMDS_LANG=zh 没能盖过 LC_ALL=C"
fi

if [ "$failures" -gt 0 ]; then
	printf '\n默认语言检查失败：%d 处问题。\n' "$failures" >&2
	exit 1
fi

echo "默认语言检查通过：英文环境无中文残留，中文环境仍为中文。"

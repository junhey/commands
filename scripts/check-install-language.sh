#!/bin/sh
# 安装脚本的语言检查。用户是从 `curl … | sh` 开始接触这个产品的，
# 安装脚本的语言就是第一印象，所以单独一道检查，不需要先构建二进制。
#
# 三个方向都查：英文环境必须英文、中文环境必须还有中文、CMDS_LANG 必须能盖过 locale。
# 只查第一条的话，把中文删干净也能「通过」——那不是国际化。
#
# 用法：scripts/check-install-language.sh

set -eu

ROOT=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
# shellcheck source=scripts/cjk.sh
. "$ROOT/scripts/cjk.sh"

SCRIPT="$ROOT/install/install.sh"
failures=0

fail() {
	failures=$((failures + 1))
	printf '  ✗ %s\n' "$1" >&2
}

# 英文环境：清掉全部 locale 变量，再把 LC_ALL 钉成 C。
out=$(env -u LANG -u LC_MESSAGES -u LC_CTYPE -u CMDS_LANG LC_ALL=C \
	sh "$SCRIPT" --help 2>&1 || true)
if has_cjk "$out"; then
	fail "英文环境下输出了中文 → $(first_cjk_line "$out")"
fi

out=$(env -u LC_ALL -u LC_MESSAGES -u CMDS_LANG LANG=zh_CN.UTF-8 \
	sh "$SCRIPT" --help 2>&1 || true)
if ! has_cjk "$out"; then
	fail "中文环境下没有输出中文（i18n 可能被误删）"
fi

out=$(env -u LC_ALL -u LC_MESSAGES LANG=zh_CN.UTF-8 CMDS_LANG=en \
	sh "$SCRIPT" --help 2>&1 || true)
if has_cjk "$out"; then
	fail "CMDS_LANG=en 没能盖过 LANG=zh_CN → $(first_cjk_line "$out")"
fi

out=$(env -u LANG -u LC_MESSAGES -u LC_CTYPE LC_ALL=C CMDS_LANG=zh \
	sh "$SCRIPT" --help 2>&1 || true)
if ! has_cjk "$out"; then
	fail "CMDS_LANG=zh 没能盖过 LC_ALL=C"
fi

if [ "$failures" -gt 0 ]; then
	printf '\n安装脚本语言检查失败：%d 处问题。\n' "$failures" >&2
	exit 1
fi

echo "安装脚本语言检查通过：默认英文、中文可用、CMDS_LANG 双向生效。"

#!/bin/sh
# 「查中文」这件事，仓库里只准有一种实现：scripts/cjk.sh 的 Perl `\p{Han}`。
#
# 为什么值得单独立一条检查：`grep '[一-龥]'` 这个写法害过两次。
#
#   1. CI 上两个 job 一起红。grep 的字符区间受 collation 影响，`LC_ALL=C` 下
#      GNU grep 直接报 "Invalid collation character" 并以非零码退出，于是
#      `if … grep` 恒为假——「英文环境不得有中文」那类断言全部「通过」，
#      一片假绿，实际什么都没查。macOS 的 BSD grep 在 C locale 下恰好能跑，
#      所以本地完全看不出来。
#
#   2. 脚本改完了，docs/deployment.md 的「上线前检查」里还留着这条命令，
#      而且它自己就设了 `LC_ALL=C`——照抄的人拿到的是一条永远打印
#      「默认语言正常」的命令。文字规范拦不住复制粘贴，所以变成检查。
#
# 检查范围与豁免——豁免的都是「说明」而不是「会被执行的命令」：
#   注释行（^\s*#）一律跳过，连代码块里的注释也跳过：本文件和
#     docs/deployment.md 的注释里就写着这个反例，正是为了教人别这么写。
#   *.md 还额外要求必须在围栏代码块内，正文叙述里提这个写法是允许的。
#
# 用法：scripts/check-cjk-detection.sh [文件...]   （默认扫全仓库相关文件）

set -eu

ROOT=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)

if [ "$#" -gt 0 ]; then
	set -- "$@"
else
	# 只扫会被执行或会被照抄的地方，node_modules / dist 之类不看。
	cd "$ROOT"
	set -- install/install.sh scripts/*.sh .github/workflows/*.yml \
		CONTRIBUTING.md CHANGELOG.md README.md README.zh-CN.md docs/*.md
fi

# 违规模式：同一行里既调 grep，又用了「汉字-汉字」的字符区间。
# 只认字符区间，不会误伤正确的 `grep -P '\p{Han}'` 或 perl 的 \p{Han}。
#
# 单引号是故意的：$ARGV、$. 这些是 Perl 的变量，必须原样交给 perl，
# 换成双引号会先被 shell 展开成空字符串，整段逻辑就废了。
# shellcheck disable=SC2016
CHECK_PERL='
	my ($in_fence, $is_md, $bad) = (0, 0, 0);
	while (<>) {
		if (!defined $last || $ARGV ne $last) {
			$in_fence = 0;
			$last = $ARGV;
			$is_md = ($ARGV =~ /\.md$/);
		}
		if ($is_md) {
			if (/^\s*```/) { $in_fence = !$in_fence; next }
			next unless $in_fence;   # 正文叙述里提这个写法是允许的
		}
		next if /^\s*#/;             # 注释是说明，不是会被执行的命令
		next unless /grep/;
		next unless /\[\s*\p{Han}\s*-\s*\p{Han}\s*\]/;
		printf "%s:%d: %s", $ARGV, $., $_;
		$bad = 1;
	}
	continue {
		# $. 在 <> 里是跨文件累加的，不重置的话报出来的行号没法用来定位。
		close ARGV if eof;
	}
	exit($bad ? 1 : 0);
'

if perl -CSD -e "$CHECK_PERL" "$@"; then
	echo "中文检测方式检查通过：没有用 grep 字符区间查中文的写法。"
	exit 0
fi

cat >&2 <<'MSG'

上面这些地方用 grep 的字符区间查中文。这个写法不可靠：
字符区间受 collation 影响，LC_ALL=C 下 GNU grep 会报
"Invalid collation character" 并以非零码退出，断言因此恒为假——
看起来通过，其实什么都没查（macOS 的 BSD grep 恰好能跑，本地看不出来）。

改用 Perl 的 \p{Han}：与 locale 无关，也不会漏掉扩展区。
脚本里直接 `. scripts/cjk.sh` 复用 has_cjk / file_has_cjk；
文档里写成：

  … | perl -CSD -ne 'if (/\p{Han}/) { print "有中文：$_"; exit 1 }'
MSG
exit 1

# shellcheck shell=sh
# 中文检测的公共实现，供 check-*-language.sh 引用（`. scripts/cjk.sh`）。
#
# 用 Perl 的 `\p{Han}` 而不是 `grep '[一-龥]'`，是踩过坑之后换的：
#
#   grep 的字符区间受 collation 影响。`grep '[一-龥]'` 在 `LC_ALL=C` 下，
#   GNU grep 直接报 "Invalid collation character" 并以错误码退出，于是
#   `if … grep "$CJK"` 恒为假——「英文环境不得有中文」这类断言会全部
#   「通过」，CI 一片绿，实际什么都没查。macOS 的 BSD grep 在 C locale 下
#   恰好能跑，所以本地完全看不出来，是 CI 上才暴露的。
#
#   试过「给 grep 单独指定 UTF-8 locale」并加自检来兜，但那是在给一个
#   本就依赖 collation 的方案打补丁：无效 locale 下 BSD grep 会退化成
#   逐字节匹配，自检还可能被糊弄过去。
#
# `\p{Han}` 没有这些问题：它是 Unicode 属性，与 locale 无关（实测 C / POSIX /
# en_US.UTF-8 / zh_CN.UTF-8 / 无效 locale 五种情况下行为一致），语义也比手写
# 区间更准——`[一-龥]` 漏掉扩展区，`\p{Han}` 不会。
#
# Perl 在 macOS 与 ubuntu runner 上都是标配；Windows 上这些检查本来就不跑。

# 判断脚本：读完 stdin，命中汉字则 exit 0，否则 exit 1。
# -CSD 让 stdin/stdout 按 UTF-8 解码，否则 \p{Han} 面对的是字节而不是字符。
CJK_PERL='while (<STDIN>) { exit 0 if /\p{Han}/ } exit 1'

# has_cjk <文本> —— 文本里有汉字则返回 0
has_cjk() {
	printf '%s' "$1" | perl -CSD -e "$CJK_PERL" 2>/dev/null
}

# file_has_cjk <文件>
file_has_cjk() {
	perl -CSD -e "$CJK_PERL" <"$1" 2>/dev/null
}

# first_cjk_line <文本> —— 取第一处含汉字的行，报错时给现场
first_cjk_line() {
	printf '%s' "$1" |
		perl -CSD -ne 'if (/\p{Han}/) { print; last }' 2>/dev/null |
		cut -c1-90
}

# 自检：检测器本身坏了比漏检更糟——会给出「通过」的假绿。
# 两个方向都验，用的是和真实检查完全同一条路径（命令替换 + has_cjk）。
_cjk_zh=$(printf '%s\n' 'prefix 中文 suffix')
_cjk_en=$(printf '%s\n' 'prefix plain ascii suffix')
if ! has_cjk "$_cjk_zh" || has_cjk "$_cjk_en"; then
	echo "中文检测器自检失败，结论不可信，直接退出。" >&2
	echo "（需要可用的 perl：perl -CSD -e '$CJK_PERL'）" >&2
	exit 2
fi
unset _cjk_zh _cjk_en

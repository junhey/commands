#!/bin/sh
# 揪出 `$VAR` 紧跟多字节字符的写法。
#
# 这个坑踩过三次了，所以做成检查而不是靠记性：
#
#     echo "已完成（$COUNT）"      # ← 坏的
#     echo "已完成（${COUNT}）"    # ← 对的
#
# 全角「）」「：」这类字符的首字节会被 shell 当成变量名的一部分，于是变量名变成
# `COUNT\xef`，在 `set -u` 下直接 `unbound variable` 退出；没有 set -u 时更糟——
# 静默展开成空字符串，看起来「没报错」但内容丢了。
#
# 注意 shellcheck 查不出这个：在它看来 `$COUNT）` 语法完全合法。
# （行首不能写成 "shellcheck ..."，那会被当成 shellcheck 指令解析而报错。）
#
# 用法：scripts/check-shell-quoting.sh <文件>...

set -eu

if [ "$#" -eq 0 ]; then
	echo "用法：scripts/check-shell-quoting.sh <文件>..." >&2
	exit 2
fi

failures=0

for file in "$@"; do
	[ -f "$file" ] || continue
	# 用 perl 而不是 grep：要按「字符」判断非 ASCII，grep 的区间又会掉进 locale 坑。
	# 只给输入解码（-CD），输出保持原始字节：脚本里的提示语本身是 UTF-8 字面量，
	# 再经一层编码就会变成乱码。匹配到的那个字符用 encode_utf8 手动转回字节。
	# 跳过注释行：本脚本自己的注释里就写着反例，不该把它当成真代码报出来。
	hits=$(perl -CD -MEncode=encode_utf8 -ne '
		next if /^\s*#/;
		while (/\$([A-Za-z_]\w*)([^\x00-\x7f])/g) {
			printf "%d: \$%s 紧跟 %s → %s\n",
				$., $1, encode_utf8($2), encode_utf8(substr($_, 0, 90));
		}
	' "$file" 2>/dev/null || true)
	if [ -n "$hits" ]; then
		printf '=== %s ===\n' "$file" >&2
		printf '%s\n' "$hits" | sed 's/^/  /' >&2
		failures=$((failures + 1))
	fi
done

if [ "$failures" -gt 0 ]; then
	echo "" >&2
	echo "上面这些 \$VAR 紧跟了多字节字符，变量名会被吃掉一个字节。" >&2
	echo "改成 \${VAR} 即可。" >&2
	exit 1
fi

echo "shell 变量界定检查通过：没有 \$VAR 紧跟多字节字符的写法。"

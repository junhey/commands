#!/bin/sh
# 验证 Linux 产物能在**任何**发行版上跑，而不只是构建机上。
#
# 由来：v0.2.0 的 Linux 包动态链接了构建机（Ubuntu 24.04）的 glibc，产物要求
# GLIBC_2.39。于是在 Debian 10/11、Ubuntu 18.04~22.04、CentOS 7/8 以及大量开发
# 容器上，一键安装装完就是
#
#     cmds: /lib64/libc.so.6: version `GLIBC_2.29' not found (required by cmds)
#
# 而 CI 一路绿灯——因为 CI 跑在构建机自己身上，永远测不出这个问题。
#
# 所以这里查两层：
#   1. 静态断言：产物不得有动态依赖、不得引用 glibc 符号版本
#   2. 行为验证：在旧 glibc 镜像里真的把它跑起来（有 docker 才跑）
#
# 第 1 层只能说明「没链错」，第 2 层才是用户真正关心的事。
#
# 用法：scripts/check-linux-portability.sh [产物路径]
# 默认 target/x86_64-unknown-linux-musl/release/cmds

set -eu

ROOT=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
TARGET=x86_64-unknown-linux-musl
BIN=${1:-"$ROOT/target/$TARGET/release/cmds"}

failures=0
fail() {
	failures=$((failures + 1))
	printf '  ✗ %s\n' "$1" >&2
}

if [ ! -f "$BIN" ]; then
	echo "找不到产物：$BIN" >&2
	echo "先构建：" >&2
	echo "  rustup target add $TARGET" >&2
	echo "  cargo build --release --locked --target $TARGET \\" >&2
	echo "    --config 'target.$TARGET.linker=\"rust-lld\"'" >&2
	exit 2
fi

# GNU readelf 在 Linux 上叫 readelf，macOS 上通常只有 llvm-readelf（Xcode 自带）。
READELF=''
for candidate in readelf llvm-readelf eu-readelf; do
	if command -v "$candidate" >/dev/null 2>&1; then
		READELF=$candidate
		break
	fi
done

if [ -z "$READELF" ]; then
	echo "找不到 readelf / llvm-readelf，无法检查 ELF，跳过静态断言。" >&2
	echo "（macOS 上 Xcode 自带 llvm-readelf；Linux 上装 binutils）" >&2
	exit 2
fi

# 变量名要用花括号界定：紧跟的全角「）」是多字节字符，不界定时它的首字节会被
# 当成变量名的一部分，在 set -u 下报 unbound variable。
echo "静态断言（${READELF}）"

if "$READELF" -d "$BIN" 2>/dev/null | grep -q NEEDED; then
	fail "有动态库依赖，应当是全静态："
	"$READELF" -d "$BIN" | grep NEEDED | sed 's/^/      /' >&2
fi

if "$READELF" --dyn-syms "$BIN" 2>/dev/null | grep -qE 'GLIBC_[0-9]+\.[0-9]+'; then
	highest=$("$READELF" --dyn-syms "$BIN" | grep -oE 'GLIBC_[0-9]+\.[0-9]+' |
		sort -u -V | tail -n 1)
	fail "引用了 glibc 符号版本（最高 ${highest}），低于此版本的发行版会跑不起来"
fi

if ! file "$BIN" | grep -qE 'static'; then
	fail "不是静态链接：$(file "$BIN")"
fi

# 行为验证。只在「产物架构 == 宿主架构」时做：拿 x86 的容器去跑 aarch64 产物
# 只会得到 exec format error，那是环境限制、不是产物问题，不该算失败。
# 交叉架构的产物只做上面的静态断言。
bin_arch=unknown
case "$(file "$BIN")" in
*x86-64*) bin_arch=x86_64 ;;
*aarch64*) bin_arch=aarch64 ;;
esac
host_arch=$(uname -m)
case "$host_arch" in
amd64) host_arch=x86_64 ;;
arm64) host_arch=aarch64 ;;
esac

if [ "$bin_arch" != "$host_arch" ]; then
	echo "（产物是 ${bin_arch}、宿主是 ${host_arch}，跳过容器实跑，只做静态断言）"
elif command -v docker >/dev/null 2>&1 && docker info >/dev/null 2>&1; then
	echo "行为验证（旧发行版容器实跑）"
	# debian:10 = glibc 2.28（用户报错那一档）
	# centos:7  = glibc 2.17（目前还在用的最老一档）
	# alpine    = 根本没有 glibc
	#
	# 刻意把容器里的真实输出打出来：这是「确实在旧系统上跑起来了」的唯一证据。
	# 吞掉输出的话，日志里只剩一行「通过」，出问题时也无从判断。
	for image in debian:10 centos:7 alpine:3.18; do
		printf '  ── %s\n' "$image"

		# 必须先单独把镜像拉下来。docker 把拉取进度写到 stderr，如果连着
		# `2>&1` 一起捕获，进度文本就会混进待断言的字符串里。
		# 第一版没这么做：CI 上恰好是第一条命令吸收了拉取噪音、第二条拿到干净输出
		# 所以通过了，发布时同样的代码却全部报红——断言靠运气就是这个后果。
		if ! docker pull -q "$image" >/dev/null 2>&1; then
			fail "${image} 镜像拉取失败（网络问题，不是产物问题）"
			continue
		fi

		# 只捕获 stdout；stderr 留着失败时单独打印。
		version=$(docker run --rm -v "$BIN:/cmds:ro" "$image" \
			/cmds --version 2>/dev/null || true)
		echoed=$(docker run --rm -v "$BIN:/cmds:ro" "$image" \
			/cmds -c 'echo container-ok' 2>/dev/null || true)

		# 断言实际内容，不只看退出码：输出为空时退出码也可能是 0。
		if [ -n "$version" ] && [ "$echoed" = "container-ok" ]; then
			printf '     %s / %s\n' "$version" "$echoed"
		else
			fail "${image} 里跑不起来"
			docker run --rm -v "$BIN:/cmds:ro" "$image" /cmds --version 2>&1 |
				head -n 3 | sed 's/^/       /' >&2
		fi
	done
else
	echo "（没有可用的 docker，跳过容器实跑；CI 的 portability job 会做这一层）"
fi

if [ "$failures" -gt 0 ]; then
	printf '\nLinux 可移植性检查失败：%d 处问题。\n' "$failures" >&2
	exit 1
fi

echo "Linux 可移植性检查通过：静态、无 glibc 依赖。"

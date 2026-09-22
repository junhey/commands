#!/usr/bin/env sh
# Commands (cmds) 一键安装脚本
#
#   curl -fsSL https://junhey.github.io/commands/install.sh | sh
#
# 可选参数：
#   -b, --bin-dir <目录>   安装目录（默认 /usr/local/bin，不可写时用 ~/.local/bin）
#   -v, --version <版本>   指定版本（默认 latest）
#   -f, --force            覆盖已存在的 cmds
#       --build            跳过下载，直接用 cargo 从源码构建
#   -y, --yes              不询问，直接安装
#   -h, --help             查看帮助

set -eu

REPO="junhey/commands"
BIN="cmds"
VERSION="${CMDS_VERSION:-latest}"
BIN_DIR="${CMDS_BIN_DIR:-}"
FORCE="${FORCE:-}"
ASSUME_YES="${ASSUME_YES:-}"
FORCE_BUILD="${FORCE_BUILD:-}"
SKIP_VERIFY="${SKIP_VERIFY:-}"

BOLD="$(tput bold 2>/dev/null || printf '')"
DIM="$(tput dim 2>/dev/null || printf '')"
RED="$(tput setaf 1 2>/dev/null || printf '')"
GREEN="$(tput setaf 2 2>/dev/null || printf '')"
YELLOW="$(tput setaf 3 2>/dev/null || printf '')"
BLUE="$(tput setaf 4 2>/dev/null || printf '')"
CYAN="$(tput setaf 6 2>/dev/null || printf '')"
RESET="$(tput sgr0 2>/dev/null || printf '')"

info() { printf '%s\n' "${DIM}▸${RESET} $*"; }
warn() { printf '%s\n' "${YELLOW}!${RESET} $*"; }
error() { printf '%s\n' "${RED}✗${RESET} $*" >&2; }
ok() { printf '%s\n' "${GREEN}✓${RESET} $*"; }

has() { command -v "$1" >/dev/null 2>&1; }

usage() {
	printf '%s\n' \
		"install.sh —— 安装 Commands (cmds)" \
		"" \
		"用法：install.sh [选项]" \
		"  -b, --bin-dir <目录>   安装目录" \
		"  -v, --version <版本>   指定版本，例如 v0.1.0（默认 latest）" \
		"  -f, --force            覆盖已安装的 cmds" \
		"      --build            直接用 cargo 从源码构建" \
		"      --skip-verify      跳过 SHA-256 校验（不建议）" \
		"  -y, --yes              不询问直接安装" \
		"  -h, --help             显示本帮助"
}

while [ "$#" -gt 0 ]; do
	case "$1" in
	-b | --bin-dir)
		BIN_DIR="${2:-}"
		shift 2
		;;
	-v | --version)
		VERSION="${2:-latest}"
		shift 2
		;;
	-f | --force)
		FORCE=1
		shift
		;;
	--build)
		FORCE_BUILD=1
		shift
		;;
	--skip-verify)
		SKIP_VERIFY=1
		shift
		;;
	-y | --yes)
		ASSUME_YES=1
		shift
		;;
	-h | --help)
		usage
		exit 0
		;;
	*)
		error "未知参数：$1"
		usage
		exit 1
		;;
	esac
done

# ── 平台探测 ────────────────────────────────────────────────────────────
detect_target() {
	os="$(uname -s)"
	arch="$(uname -m)"
	case "$os" in
	Linux) os_part="unknown-linux-gnu" ;;
	Darwin) os_part="apple-darwin" ;;
	FreeBSD) os_part="unknown-freebsd" ;;
	MINGW* | MSYS* | CYGWIN*) os_part="pc-windows-msvc" ;;
	*)
		error "暂不支持的系统：$os"
		exit 1
		;;
	esac
	case "$arch" in
	x86_64 | amd64) arch_part="x86_64" ;;
	arm64 | aarch64) arch_part="aarch64" ;;
	armv7l) arch_part="armv7" ;;
	riscv64) arch_part="riscv64gc" ;;
	*)
		error "暂不支持的架构：$arch"
		exit 1
		;;
	esac
	printf '%s-%s' "$arch_part" "$os_part"
}

writable() {
	target="${1:-}/.cmds-write-test"
	if touch "$target" 2>/dev/null; then
		rm -f "$target"
		return 0
	fi
	return 1
}

choose_bin_dir() {
	if [ -n "$BIN_DIR" ]; then
		printf '%s' "$BIN_DIR"
		return
	fi
	for candidate in /usr/local/bin "$HOME/.local/bin" "$HOME/bin"; do
		if [ -d "$candidate" ] && writable "$candidate"; then
			printf '%s' "$candidate"
			return
		fi
	done
	printf '%s' "$HOME/.local/bin"
}

download() {
	url="$1"
	out="$2"
	if has curl; then
		curl --fail --silent --show-error --location --output "$out" "$url"
	elif has wget; then
		wget --quiet --output-document="$out" "$url"
	else
		error "需要 curl 或 wget"
		return 1
	fi
}

# 用 sha256sum / shasum / openssl 里任意可用的一个算摘要
sha256_of() {
	file="$1"
	if has sha256sum; then
		sha256sum "$file" | cut -d' ' -f1
	elif has shasum; then
		shasum -a 256 "$file" | cut -d' ' -f1
	elif has openssl; then
		openssl dgst -sha256 "$file" | awk '{print $NF}'
	else
		printf ''
	fi
}

# 校验下载到的压缩包。Release 里每个资产都带同名的 .sha256。
# 拿不到校验文件或本机没有摘要工具时只告警，不阻断安装。
verify_checksum() {
	archive_path="$1"
	checksum_url="$2"
	tmp_dir="$3"

	if [ -n "$SKIP_VERIFY" ]; then
		warn "已按要求跳过校验"
		return 0
	fi
	if ! download "$checksum_url" "$tmp_dir/checksum" 2>/dev/null; then
		warn "没有取到校验文件，跳过校验"
		return 0
	fi

	expected="$(cut -d' ' -f1 <"$tmp_dir/checksum" | tr -d '\r\n')"
	actual="$(sha256_of "$archive_path")"
	if [ -z "$actual" ]; then
		warn "本机没有 sha256sum / shasum / openssl，跳过校验"
		return 0
	fi
	if [ "$expected" != "$actual" ]; then
		error "SHA-256 校验失败，已放弃安装"
		error "  期望：$expected"
		error "  实际：$actual"
		return 1
	fi
	ok "SHA-256 校验通过"
	return 0
}

install_from_release() {
	target="$1"
	bin_dir="$2"
	case "$target" in
	*windows*) archive="${BIN}-${target}.zip" ;;
	*) archive="${BIN}-${target}.tar.gz" ;;
	esac
	if [ "$VERSION" = "latest" ]; then
		base_url="https://github.com/${REPO}/releases/latest/download"
	else
		base_url="https://github.com/${REPO}/releases/download/${VERSION}"
	fi
	url="${base_url}/${archive}"

	tmp="$(mktemp -d 2>/dev/null || printf '/tmp/cmds-install-%s' "$$")"
	mkdir -p "$tmp"
	info "下载 ${BLUE}${url}${RESET}"
	if ! download "$url" "$tmp/$archive"; then
		rm -rf "$tmp"
		return 1
	fi

	if ! verify_checksum "$tmp/$archive" "${url}.sha256" "$tmp"; then
		rm -rf "$tmp"
		return 1
	fi

	case "$archive" in
	*.tar.gz) tar -xzf "$tmp/$archive" -C "$tmp" ;;
	*.zip) unzip -qo "$tmp/$archive" -d "$tmp" ;;
	esac

	binary="$(find "$tmp" -type f -name "${BIN}" -o -type f -name "${BIN}.exe" 2>/dev/null | head -n 1)"
	if [ -z "$binary" ]; then
		error "压缩包里没有找到 ${BIN}"
		rm -rf "$tmp"
		return 1
	fi
	chmod +x "$binary"
	mkdir -p "$bin_dir"
	if writable "$bin_dir"; then
		mv "$binary" "$bin_dir/$(basename "$binary")"
	else
		warn "$bin_dir 需要管理员权限，使用 sudo"
		sudo mkdir -p "$bin_dir"
		sudo mv "$binary" "$bin_dir/$(basename "$binary")"
	fi
	rm -rf "$tmp"
	return 0
}

install_from_source() {
	bin_dir="$1"
	if ! has cargo; then
		error "没有预编译包，也没有 cargo；请先安装 Rust：https://rustup.rs"
		return 1
	fi
	root="$(dirname "$bin_dir")"

	# 先走 crates.io：有版本语义、不需要拉整个仓库。
	# --version 传的是 vX.Y.Z，cargo 要的是 X.Y.Z，去掉前缀。
	if [ "$VERSION" = "latest" ]; then
		info "从 crates.io 安装（首次编译约 1 分钟）"
		if cargo install --locked --root "$root" "$BIN"; then
			return 0
		fi
	else
		info "从 crates.io 安装 ${VERSION}（首次编译约 1 分钟）"
		if cargo install --locked --root "$root" --version "${VERSION#v}" "$BIN"; then
			return 0
		fi
	fi

	# crates.io 上还没有这个版本时，回退到仓库源码
	warn "crates.io 安装未成功，改从仓库源码构建"
	cargo install --locked --git "https://github.com/${REPO}" --root "$root" "$BIN"
}

# ── 主流程 ──────────────────────────────────────────────────────────────
printf '\n%s\n' "${BOLD}${CYAN}Commands${RESET} — 轻量高效的交互式终端"
printf '%s\n\n' "${DIM}历史自动建议 · Tab 候选菜单 · starship 风格提示符${RESET}"

TARGET="$(detect_target)"
BIN_DIR="$(choose_bin_dir)"
info "平台：${BOLD}${TARGET}${RESET}"
info "安装目录：${BOLD}${BIN_DIR}${RESET}"

if [ -x "$BIN_DIR/$BIN" ] && [ -z "$FORCE" ]; then
	warn "$BIN_DIR/$BIN 已存在，将覆盖升级（可用 --force 静默覆盖）"
fi

if [ -z "$ASSUME_YES" ] && [ -t 0 ]; then
	printf '%s' "继续安装？[Y/n] "
	read -r answer
	case "$answer" in
	n | N | no | NO) info "已取消"; exit 0 ;;
	esac
fi

if [ -n "$FORCE_BUILD" ]; then
	install_from_source "$BIN_DIR" || exit 1
elif ! install_from_release "$TARGET" "$BIN_DIR"; then
	warn "没有对应平台的预编译包，改为源码构建"
	install_from_source "$BIN_DIR" || exit 1
fi

if [ ! -x "$BIN_DIR/$BIN" ]; then
	error "安装失败：$BIN_DIR/$BIN 不存在"
	exit 1
fi

ok "已安装到 ${BOLD}${BIN_DIR}/${BIN}${RESET}"
"$BIN_DIR/$BIN" --version 2>/dev/null || true

printf '\n%s\n' "${BOLD}接下来${RESET}"
case ":$PATH:" in
*":$BIN_DIR:"*) ;;
*)
	printf '  %s\n' "1. 把安装目录加入 PATH："
	printf '     %s\n' "${CYAN}echo 'export PATH=\"$BIN_DIR:\$PATH\"' >> ~/.profile${RESET}"
	;;
esac
printf '  %s\n' "• 直接启动：${CYAN}cmds${RESET}（输入 ${BOLD}help${RESET} 查看全部快捷键）"
printf '  %s\n' "• 生成配置模板：${CYAN}cmds config init${RESET}"
printf '  %s\n' "• 只想用它的提示符？${CYAN}eval \"\$(cmds init bash)\"${RESET} / ${CYAN}cmds init fish | source${RESET}"
printf '  %s\n' "• 设为默认 shell：${CYAN}echo $BIN_DIR/cmds | sudo tee -a /etc/shells && chsh -s $BIN_DIR/cmds${RESET}"
printf '\n%s\n\n' "文档：${BLUE}https://junhey.github.io/commands/#guide${RESET}"

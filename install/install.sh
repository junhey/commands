#!/usr/bin/env sh
# Commands (cmds) installer
#
#   curl -fsSL https://junhey.github.io/commands/install.sh | sh
#
# Options:
#   -b, --bin-dir <dir>    install directory (default /usr/local/bin, or ~/.local/bin)
#   -v, --version <ver>    install a specific version (default latest)
#   -f, --force            overwrite an existing cmds
#       --build            skip the download, build from source with cargo
#       --skip-verify      skip the SHA-256 check (not recommended)
#   -y, --yes              don't ask, just install
#   -h, --help             show help
#
# Messages are English by default and switch to Chinese when the locale asks
# for it (LC_ALL / LC_MESSAGES / LANG starting with "zh"). Set CMDS_LANG=zh or
# CMDS_LANG=en to force one.

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

# ── Language ────────────────────────────────────────────────────────────
# English unless the locale explicitly asks for Chinese. Same precedence as
# POSIX: LC_ALL > LC_MESSAGES > LANG. C/POSIX/unset all mean English.
USE_ZH=''
case "${CMDS_LANG:-${LC_ALL:-${LC_MESSAGES:-${LANG:-}}}}" in
zh* | ZH*) USE_ZH=1 ;;
esac

# msg <english> <chinese>
msg() {
	if [ -n "$USE_ZH" ]; then
		printf '%s' "$2"
	else
		printf '%s' "$1"
	fi
}

info() { printf '%s\n' "${DIM}▸${RESET} $*"; }
warn() { printf '%s\n' "${YELLOW}!${RESET} $*"; }
error() { printf '%s\n' "${RED}✗${RESET} $*" >&2; }
ok() { printf '%s\n' "${GREEN}✓${RESET} $*"; }

has() { command -v "$1" >/dev/null 2>&1; }

usage() {
	if [ -n "$USE_ZH" ]; then
		printf '%s\n' \
			"install.sh —— 安装 Commands (cmds)" \
			"" \
			"用法：install.sh [选项]" \
			"  -b, --bin-dir <目录>   安装目录" \
			"  -v, --version <版本>   指定版本，例如 v0.2.0（默认 latest）" \
			"  -f, --force            覆盖已安装的 cmds" \
			"      --build            直接用 cargo 从源码构建" \
			"      --skip-verify      跳过 SHA-256 校验（不建议）" \
			"  -y, --yes              不询问直接安装" \
			"  -h, --help             显示本帮助" \
			"" \
			"环境变量 CMDS_LANG=en 可强制英文输出。"
	else
		printf '%s\n' \
			"install.sh — install Commands (cmds)" \
			"" \
			"Usage: install.sh [options]" \
			"  -b, --bin-dir <dir>    install directory" \
			"  -v, --version <ver>    install a specific version, e.g. v0.2.0 (default latest)" \
			"  -f, --force            overwrite an existing cmds" \
			"      --build            build from source with cargo" \
			"      --skip-verify      skip the SHA-256 check (not recommended)" \
			"  -y, --yes              don't ask, just install" \
			"  -h, --help             show this help" \
			"" \
			"Set CMDS_LANG=zh to force Chinese output."
	fi
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
		error "$(msg "unknown option: $1" "未知参数：$1")"
		usage
		exit 1
		;;
	esac
done

# ── Platform detection ──────────────────────────────────────────────────
# 每行输出一个候选 target，按优先级从高到低。
#
# Linux 上 musl 排在 gnu 前面：从 v0.2.1 起 Linux 包改成 musl 静态链接，
# 因为动态链接 glibc 的包会把构建机的 glibc 版本写进 ELF 的版本需求，
# 在旧发行版上直接 `version GLIBC_2.xx not found`。保留 gnu 作为回退，
# 是为了让 `--version v0.2.0` 这类指定旧版本的安装仍然能找到资产。
detect_targets() {
	os="$(uname -s)"
	arch="$(uname -m)"
	case "$os" in
	Linux) os_parts="unknown-linux-musl unknown-linux-gnu" ;;
	Darwin) os_parts="apple-darwin" ;;
	FreeBSD) os_parts="unknown-freebsd" ;;
	MINGW* | MSYS* | CYGWIN*) os_parts="pc-windows-msvc" ;;
	*)
		error "$(msg "unsupported system: $os" "暂不支持的系统：$os")"
		exit 1
		;;
	esac
	case "$arch" in
	x86_64 | amd64) arch_part="x86_64" ;;
	arm64 | aarch64) arch_part="aarch64" ;;
	armv7l) arch_part="armv7" ;;
	riscv64) arch_part="riscv64gc" ;;
	*)
		error "$(msg "unsupported architecture: $arch" "暂不支持的架构：$arch")"
		exit 1
		;;
	esac
	for os_part in $os_parts; do
		printf '%s-%s\n' "$arch_part" "$os_part"
	done
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
		error "$(msg "curl or wget is required" "需要 curl 或 wget")"
		return 1
	fi
}

# Use whichever of sha256sum / shasum / openssl is available.
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

# Verify the downloaded archive. Every release asset ships a matching .sha256.
# A missing checksum file or no digest tool only warns; a mismatch aborts.
verify_checksum() {
	archive_path="$1"
	checksum_url="$2"
	tmp_dir="$3"

	if [ -n "$SKIP_VERIFY" ]; then
		warn "$(msg "skipping verification as requested" "已按要求跳过校验")"
		return 0
	fi
	if ! download "$checksum_url" "$tmp_dir/checksum" 2>/dev/null; then
		warn "$(msg "no checksum file available, skipping verification" "没有取到校验文件，跳过校验")"
		return 0
	fi

	expected="$(cut -d' ' -f1 <"$tmp_dir/checksum" | tr -d '\r\n')"
	actual="$(sha256_of "$archive_path")"
	if [ -z "$actual" ]; then
		warn "$(msg "no sha256sum / shasum / openssl here, skipping verification" "本机没有 sha256sum / shasum / openssl，跳过校验")"
		return 0
	fi
	if [ "$expected" != "$actual" ]; then
		error "$(msg "SHA-256 mismatch, install aborted" "SHA-256 校验失败，已放弃安装")"
		error "  $(msg "expected: " "期望：")$expected"
		error "  $(msg "actual:   " "实际：")$actual"
		return 1
	fi
	ok "$(msg "SHA-256 verified" "SHA-256 校验通过")"
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
	info "$(msg "downloading" "下载") ${BLUE}${url}${RESET}"
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
		error "$(msg "${BIN} was not found inside the archive" "压缩包里没有找到 ${BIN}")"
		rm -rf "$tmp"
		return 1
	fi
	chmod +x "$binary"
	mkdir -p "$bin_dir"
	if writable "$bin_dir"; then
		mv "$binary" "$bin_dir/$(basename "$binary")"
	else
		warn "$(msg "$bin_dir needs elevated rights, using sudo" "$bin_dir 需要管理员权限，使用 sudo")"
		sudo mkdir -p "$bin_dir"
		sudo mv "$binary" "$bin_dir/$(basename "$binary")"
	fi
	rm -rf "$tmp"
	return 0
}

# 依次尝试各候选 target，第一个成功就收工。
# 第一个候选失败通常只是「这个版本的 Release 里没有这个资产名」，不是错误，
# 所以不报错，只在切换时说明一下，免得用户看到一条 404 的下载地址一头雾水。
install_any_release() {
	bin_dir="$1"
	tried=0
	for target in $TARGETS; do
		if [ "$tried" -ne 0 ]; then
			info "$(msg "not published for that target, trying " \
				"该目标没有发布包，改用 ")${BOLD}${target}${RESET}"
		fi
		tried=1
		if install_from_release "$target" "$bin_dir"; then
			return 0
		fi
	done
	return 1
}

install_from_source() {
	bin_dir="$1"
	if ! has cargo; then
		error "$(msg \
			"no prebuilt package and no cargo; install Rust first: https://rustup.rs" \
			"没有预编译包，也没有 cargo；请先安装 Rust：https://rustup.rs")"
		return 1
	fi
	root="$(dirname "$bin_dir")"

	# Prefer crates.io: proper version semantics, no need to clone the repo.
	# --version takes vX.Y.Z but cargo wants X.Y.Z, so strip the prefix.
	if [ "$VERSION" = "latest" ]; then
		info "$(msg "installing from crates.io (first build takes about a minute)" \
			"从 crates.io 安装（首次编译约 1 分钟）")"
		if cargo install --locked --root "$root" "$BIN"; then
			return 0
		fi
	else
		info "$(msg "installing ${VERSION} from crates.io (first build takes about a minute)" \
			"从 crates.io 安装 ${VERSION}（首次编译约 1 分钟）")"
		if cargo install --locked --root "$root" --version "${VERSION#v}" "$BIN"; then
			return 0
		fi
	fi

	# That version may not be on crates.io yet; fall back to the repository.
	warn "$(msg "crates.io install did not succeed, building from the repository" \
		"crates.io 安装未成功，改从仓库源码构建")"
	cargo install --locked --git "https://github.com/${REPO}" --root "$root" "$BIN"
}

# ── Main ────────────────────────────────────────────────────────────────
printf '\n%s\n' "${BOLD}${CYAN}Commands${RESET} — $(msg "a small, fast interactive shell" "轻量高效的交互式终端")"
printf '%s\n\n' "${DIM}$(msg \
	"history autosuggestions · Tab candidate menu · starship-style prompt" \
	"历史自动建议 · Tab 候选菜单 · starship 风格提示符")${RESET}"

TARGETS="$(detect_targets)"
# 展示用只取首选那个，回退目标是实现细节，没必要摆到用户眼前。
TARGET="$(printf '%s\n' "$TARGETS" | head -n 1)"
BIN_DIR="$(choose_bin_dir)"
# 标点也要跟着语言走：英文用半角冒号加空格，中文用全角冒号。
info "$(msg "platform: " "平台：")${BOLD}${TARGET}${RESET}"
info "$(msg "install directory: " "安装目录：")${BOLD}${BIN_DIR}${RESET}"

if [ -x "$BIN_DIR/$BIN" ] && [ -z "$FORCE" ]; then
	warn "$(msg \
		"$BIN_DIR/$BIN already exists and will be replaced (--force skips this notice)" \
		"$BIN_DIR/$BIN 已存在，将覆盖升级（可用 --force 静默覆盖）")"
fi

if [ -z "$ASSUME_YES" ] && [ -t 0 ]; then
	printf '%s' "$(msg "Continue? [Y/n] " "继续安装？[Y/n] ")"
	read -r answer
	case "$answer" in
	n | N | no | NO)
		info "$(msg "cancelled" "已取消")"
		exit 0
		;;
	esac
fi

if [ -n "$FORCE_BUILD" ]; then
	install_from_source "$BIN_DIR" || exit 1
elif ! install_any_release "$BIN_DIR"; then
	warn "$(msg "no prebuilt package for this platform, building from source" \
		"没有对应平台的预编译包，改为源码构建")"
	install_from_source "$BIN_DIR" || exit 1
fi

if [ ! -x "$BIN_DIR/$BIN" ]; then
	error "$(msg "install failed: $BIN_DIR/$BIN does not exist" "安装失败：$BIN_DIR/$BIN 不存在")"
	exit 1
fi

ok "$(msg "installed to" "已安装到") ${BOLD}${BIN_DIR}/${BIN}${RESET}"
"$BIN_DIR/$BIN" --version 2>/dev/null || true

# 收尾这段整块分语言写：句子里穿插了命令和颜色变量，
# 逐词 msg 会把标点和空格拆散，反而更容易出现「prompt?eval」这种缺空格的错。
printf '\n%s\n' "${BOLD}$(msg "Next" "接下来")${RESET}"
needs_path=''
case ":$PATH:" in
*":$BIN_DIR:"*) ;;
*) needs_path=1 ;;
esac

if [ -n "$USE_ZH" ]; then
	if [ -n "$needs_path" ]; then
		printf '  %s\n' "1. 把安装目录加入 PATH："
		printf '     %s\n' "${CYAN}echo 'export PATH=\"$BIN_DIR:\$PATH\"' >> ~/.profile${RESET}"
	fi
	printf '  %s\n' "• 直接启动：${CYAN}cmds${RESET}（输入 ${BOLD}help${RESET} 查看全部快捷键）"
	printf '  %s\n' "• 生成配置模板：${CYAN}cmds config init${RESET}"
	printf '  %s\n' "• 只想用它的提示符？${CYAN}eval \"\$(cmds init bash)\"${RESET} / ${CYAN}cmds init fish | source${RESET}"
	printf '  %s\n' "• 设为默认 shell：${CYAN}echo $BIN_DIR/cmds | sudo tee -a /etc/shells && chsh -s $BIN_DIR/cmds${RESET}"
	printf '\n%s\n\n' "文档：${BLUE}https://junhey.github.io/commands/#guide${RESET}"
else
	if [ -n "$needs_path" ]; then
		printf '  %s\n' "1. Add the install directory to PATH:"
		printf '     %s\n' "${CYAN}echo 'export PATH=\"$BIN_DIR:\$PATH\"' >> ~/.profile${RESET}"
	fi
	printf '  %s\n' "• Start it: ${CYAN}cmds${RESET} (type ${BOLD}help${RESET} for every keybinding)"
	printf '  %s\n' "• Write a config template: ${CYAN}cmds config init${RESET}"
	printf '  %s\n' "• Only want the prompt? ${CYAN}eval \"\$(cmds init bash)\"${RESET} / ${CYAN}cmds init fish | source${RESET}"
	printf '  %s\n' "• Make it your login shell: ${CYAN}echo $BIN_DIR/cmds | sudo tee -a /etc/shells && chsh -s $BIN_DIR/cmds${RESET}"
	printf '\n%s\n\n' "Docs: ${BLUE}https://junhey.github.io/commands/#guide${RESET}"
fi

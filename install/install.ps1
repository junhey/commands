<#
.SYNOPSIS
    Commands (cmds) 一键安装脚本（Windows）

.EXAMPLE
    irm https://junhey.github.io/commands/install.ps1 | iex

.EXAMPLE
    .\install.ps1 -BinDir "$env:LOCALAPPDATA\Programs\cmds" -Version v0.1.0
#>
[CmdletBinding()]
param(
    [string]$BinDir = "$env:LOCALAPPDATA\Programs\cmds",
    [string]$Version = 'latest',
    [switch]$Force,
    [switch]$Build
)

$ErrorActionPreference = 'Stop'
$Repo = 'junhey/commands'

function Write-Info { param($Message) Write-Host "▸ $Message" -ForegroundColor DarkGray }
function Write-Ok { param($Message) Write-Host "✓ $Message" -ForegroundColor Green }
function Write-Warn2 { param($Message) Write-Host "! $Message" -ForegroundColor Yellow }
function Write-Err { param($Message) Write-Host "✗ $Message" -ForegroundColor Red }

function Get-Target {
    $arch = switch ($env:PROCESSOR_ARCHITECTURE) {
        'AMD64' { 'x86_64' }
        'ARM64' { 'aarch64' }
        'x86' { 'i686' }
        default { 'x86_64' }
    }
    "$arch-pc-windows-msvc"
}

function Install-FromRelease {
    param([string]$Target, [string]$Destination)

    $archive = "cmds-$Target.zip"
    $url = if ($Version -eq 'latest') {
        "https://github.com/$Repo/releases/latest/download/$archive"
    } else {
        "https://github.com/$Repo/releases/download/$Version/$archive"
    }

    $temp = Join-Path $env:TEMP "cmds-install-$([System.Guid]::NewGuid().ToString('N'))"
    New-Item -ItemType Directory -Path $temp -Force | Out-Null
    $zip = Join-Path $temp $archive

    Write-Info "下载 $url"
    try {
        Invoke-WebRequest -Uri $url -OutFile $zip -UseBasicParsing
    } catch {
        Remove-Item $temp -Recurse -Force -ErrorAction SilentlyContinue
        return $false
    }

    Expand-Archive -Path $zip -DestinationPath $temp -Force
    $binary = Get-ChildItem -Path $temp -Recurse -Filter 'cmds.exe' | Select-Object -First 1
    if (-not $binary) {
        Write-Err '压缩包里没有找到 cmds.exe'
        Remove-Item $temp -Recurse -Force -ErrorAction SilentlyContinue
        return $false
    }

    New-Item -ItemType Directory -Path $Destination -Force | Out-Null
    Copy-Item $binary.FullName (Join-Path $Destination 'cmds.exe') -Force
    Remove-Item $temp -Recurse -Force -ErrorAction SilentlyContinue
    return $true
}

function Install-FromSource {
    param([string]$Destination)

    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Write-Err '没有预编译包，也没有 cargo；请先安装 Rust：https://rustup.rs'
        return $false
    }
    Write-Info '使用 cargo 从源码构建（首次约 1 分钟）'
    $root = Split-Path $Destination -Parent
    cargo install --locked --git "https://github.com/$Repo" --root $root cmds
    return $LASTEXITCODE -eq 0
}

Write-Host ''
Write-Host 'Commands' -ForegroundColor Cyan -NoNewline
Write-Host ' — 轻量高效的交互式终端'
Write-Host '历史自动建议 · Tab 候选菜单 · starship 风格提示符' -ForegroundColor DarkGray
Write-Host ''

$target = Get-Target
Write-Info "平台：$target"
Write-Info "安装目录：$BinDir"

$exe = Join-Path $BinDir 'cmds.exe'
if ((Test-Path $exe) -and -not $Force) {
    Write-Warn2 "$exe 已存在，将覆盖升级"
}

$installed = $false
if ($Build) {
    $installed = Install-FromSource -Destination $BinDir
} else {
    $installed = Install-FromRelease -Target $target -Destination $BinDir
    if (-not $installed) {
        Write-Warn2 '没有对应平台的预编译包，改为源码构建'
        $installed = Install-FromSource -Destination $BinDir
    }
}

if (-not $installed -or -not (Test-Path $exe)) {
    Write-Err '安装失败'
    exit 1
}

Write-Ok "已安装到 $exe"
& $exe --version

$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
if ($userPath -notlike "*$BinDir*") {
    [Environment]::SetEnvironmentVariable('Path', "$BinDir;$userPath", 'User')
    Write-Ok "已把 $BinDir 加入用户 PATH（新开终端生效）"
}

Write-Host ''
Write-Host '接下来' -ForegroundColor White
Write-Host '  • 直接启动：' -NoNewline; Write-Host 'cmds' -ForegroundColor Cyan -NoNewline; Write-Host '（输入 help 查看全部快捷键）'
Write-Host '  • 生成配置模板：' -NoNewline; Write-Host 'cmds config init' -ForegroundColor Cyan
Write-Host '  • 只想用提示符：' -NoNewline; Write-Host 'Invoke-Expression (& cmds init powershell | Out-String)' -ForegroundColor Cyan
Write-Host ''
Write-Host '文档：https://junhey.github.io/commands/#guide' -ForegroundColor Blue
Write-Host ''

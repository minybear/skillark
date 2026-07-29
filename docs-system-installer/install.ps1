<#
.SYNOPSIS
  文档沉淀体系一键安装（Windows / PowerShell）。
  把 docs 项目化归档规范 + 4 个 skill + Stop hook + 铁律装进当前 git 项目。

.DESCRIPTION
  - 在 .claude/skills/ 下安装 4 个 skill（init-project-docs / write-worklog / write-issue / write-plan）
  - 在 .claude/hooks/ 下安装 archive-reminder.sh，并把 Stop hook 写入 .claude/settings.json
  - 把"任务收尾必归档"铁律追加进 CLAUDE.md 与 AGENTS.md（缺失才追加，幂等）
  - Codex 读 AGENTS.md 与 SKILL.md，Claude Code 读 CLAUDE.md，二者通用

.PARAMETER ProjectRoot
  目标项目根目录（默认取当前 git 仓库根，取不到则用当前目录）。

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File install.ps1
  powershell -ExecutionPolicy Bypass -File install.ps1 -ProjectRoot D:\work\myproj
#>
param(
    [string]$ProjectRoot = ""
)

$ErrorActionPreference = "Stop"

# 无 BOM 的 UTF8 编码（避免 Claude Code / 解析器读出 BOM 头）
$script:Utf8NoBom = New-Object System.Text.UTF8Encoding($false)

function Write-Utf8NoBom($path, $content) {
    [System.IO.File]::WriteAllText($path, $content, $script:Utf8NoBom)
}
function Read-Utf8($path) {
    return [System.IO.File]::ReadAllText($path)
}

function Write-Ok($msg)   { Write-Host "  [OK] $msg" -ForegroundColor Green }
function Write-Skip($msg) { Write-Host "  [--] $msg（已存在，跳过）" -ForegroundColor DarkGray }
function Write-Info($msg) { Write-Host "  [..] $msg" -ForegroundColor Cyan }

# 自动检索 payload 目录（从任意位置运行均可）：
# 依次看 脚本同目录 → 当前目录 → 当前目录下 docs-system-installer\payload
# → 脚本父目录的 docs-system-installer\payload → 兜底：向下最多 3 层找含 rule.md 的 payload 目录
function Find-Payload {
    $candidates = @(
        (Join-Path $PSScriptRoot "payload"),
        (Join-Path (Get-Location) "payload"),
        (Join-Path (Get-Location) "docs-system-installer\payload"),
        (Join-Path $PSScriptRoot "..\docs-system-installer\payload")
    )
    foreach ($c in $candidates) {
        $resolved = (Resolve-Path $c -ErrorAction SilentlyContinue).Path
        if ($resolved -and (Test-Path (Join-Path $resolved "rule.md"))) { return $resolved }
    }
    foreach ($base in @($PSScriptRoot, (Get-Location).Path)) {
        $hit = Get-ChildItem -Path $base -Recurse -Depth 3 -Directory -Filter "payload" -ErrorAction SilentlyContinue |
               Where-Object { Test-Path (Join-Path $_.FullName "rule.md") } |
               Select-Object -First 1
        if ($hit) { return $hit.FullName }
    }
    return $null
}

# ---- 解析项目根 ----
if ([string]::IsNullOrWhiteSpace($ProjectRoot)) {
    try {
        $ProjectRoot = (git rev-parse --show-toplevel 2>$null).Trim()
    } catch { $ProjectRoot = "" }
    if ([string]::IsNullOrWhiteSpace($ProjectRoot)) { $ProjectRoot = (Get-Location).Path }
}
$ProjectRoot = $ProjectRoot -replace '/', '\'
Write-Host "`n==> 安装目标项目：$ProjectRoot`n" -ForegroundColor Yellow

$PayloadDir = Find-Payload
if (-not $PayloadDir) {
    Write-Host "找不到 payload 目录（已在脚本目录、当前目录及其下 docs-system-installer\payload 检索）" -ForegroundColor Red
    Write-Host "请确认 payload\（含 skills\ hooks\ rule.md）存在，或把安装包完整解压后重试" -ForegroundColor Red
    exit 1
}
Write-Host "==> 使用 payload：$PayloadDir`n" -ForegroundColor Yellow

# ---- 1. 安装 skills ----
Write-Host "1) 安装 skills 到 .claude/skills/" -ForegroundColor White
$skillsTarget = Join-Path $ProjectRoot ".claude\skills"
$skillDirs = Get-ChildItem -Path (Join-Path $PayloadDir "skills") -Directory
foreach ($s in $skillDirs) {
    $dest = Join-Path $skillsTarget $s.Name
    $destFile = Join-Path $dest "SKILL.md"
    if (Test-Path $destFile) { Write-Skip "skill $($s.Name)"; continue }
    New-Item -ItemType Directory -Force -Path $dest | Out-Null
    Copy-Item -Path (Join-Path $s.FullName "SKILL.md") -Destination $destFile -Force
    Write-Ok "skill $($s.Name)"
}

# ---- 2. 安装 Stop hook ----
Write-Host "2) 安装 Stop hook（任务收尾归档提醒）" -ForegroundColor White
$hooksTarget = Join-Path $ProjectRoot ".claude\hooks"
if (Test-Path (Join-Path $PayloadDir "hooks\archive-reminder.sh")) {
    New-Item -ItemType Directory -Force -Path $hooksTarget | Out-Null
$hookSrc = Join-Path $PayloadDir "hooks\archive-reminder.sh"
$hookDest = Join-Path $hooksTarget "archive-reminder.sh"
Copy-Item -Path $hookSrc -Destination $hookDest -Force
Write-Ok "hook 脚本 archive-reminder.sh"

# 写入 .claude/settings.json 的 hooks.Stop（保留已有 permissions）
$settingsPath = Join-Path $ProjectRoot ".claude\settings.json"
$HOOK_CMD = 'cd "${CLAUDE_PROJECT_DIR:-.}" && bash .claude/hooks/archive-reminder.sh'
$settings = $null
if (Test-Path $settingsPath) {
    try { $settings = Read-Utf8 $settingsPath | ConvertFrom-Json } catch { $settings = $null }
}
if ($null -eq $settings) { $settings = [pscustomobject]@{} }

# 确保 hooks.Stop 存在且包含我们的命令（避免重复添加）
$already = $false
if ($settings.PSObject.Properties.Name -contains "hooks" -and
    $settings.hooks.PSObject.Properties.Name -contains "Stop") {
    foreach ($grp in $settings.hooks.Stop) {
        foreach ($h in $grp.hooks) {
            if ($h.command -eq $HOOK_CMD) { $already = $true }
        }
    }
}
if ($already) {
    Write-Skip "settings.json Stop hook"
} else {
    $hookEntry = [pscustomobject]@{ type = "command"; command = $HOOK_CMD }
    $hookGroup = [pscustomobject]@{ hooks = @($hookEntry) }
    if (-not ($settings.PSObject.Properties.Name -contains "hooks")) {
        $settings | Add-Member -NotePropertyName "hooks" -NotePropertyValue ([pscustomobject]@{})
    }
    if (-not ($settings.hooks.PSObject.Properties.Name -contains "Stop")) {
        $settings.hooks | Add-Member -NotePropertyName "Stop" -NotePropertyValue @($hookGroup)
    } else {
        $settings.hooks.Stop = @($settings.hooks.Stop) + $hookGroup
    }
    Write-Utf8NoBom $settingsPath ($settings | ConvertTo-Json -Depth 10)
    Write-Ok "settings.json 已注册 Stop hook"
}

} else {
    Write-Host "  [--] hook 未包含（v1），跳过" -ForegroundColor DarkGray
}

# ---- 3. 铁律写入 CLAUDE.md 与 AGENTS.md ----
Write-Host "3) 写入铁律到 CLAUDE.md / AGENTS.md" -ForegroundColor White
$rulePath = Join-Path $PayloadDir "rule.md"
$ruleText = Read-Utf8 $rulePath
$marker = "任务收尾必归档"

foreach ($fname in @("CLAUDE.md", "AGENTS.md")) {
    $fpath = Join-Path $ProjectRoot $fname
    if (Test-Path $fpath) {
        $existing = Read-Utf8 $fpath
        if ($existing -match [regex]::Escape($marker)) {
            Write-Skip "$fname 铁律"
            continue
        }
        Write-Utf8NoBom $fpath ($existing.TrimEnd() + "`r`n`r`n" + $ruleText.TrimEnd() + "`r`n")
        Write-Ok "$fname 已追加铁律"
    } else {
        $header = if ($fname -eq "CLAUDE.md") { "# CLAUDE.md`r`n`r`n" } else { "# AGENTS.md`r`n`r`n" }
        Write-Utf8NoBom $fpath ($header + $ruleText.TrimEnd() + "`r`n")
        Write-Ok "$fname 已创建并写入铁律"
    }
}

# ---- 完成 ----
Write-Host "`n==> 安装完成 ✔`n" -ForegroundColor Green
Write-Host "后续使用：" -ForegroundColor White
Write-Host "  · 开始新项目：对 AI 说「开始 XX 项目」，或直接说「在 docs 下给 XX 建目录」（init-project-docs）"
Write-Host "  · 功能/修复完成：AI 会用 write-worklog 把日志写到 docs/{项目}/logs/"
Write-Host "  · 发现待处理问题：AI 会用 write-issue 登记到 docs/{项目}/issues/open/"
Write-Host "  · Claude Code 归档提醒 hook：v1 暂未包含（v2 发布后生效）`n"

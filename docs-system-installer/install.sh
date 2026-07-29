#!/usr/bin/env bash
# 文档沉淀体系一键安装（Mac / Linux / bash）。
# 把 docs 项目化归档规范 + 4 个 skill + Stop hook + 铁律装进当前 git 项目。
# Codex 读 AGENTS.md 与 SKILL.md，Claude Code 读 CLAUDE.md，二者通用。
#
# 用法：
#   bash install.sh [项目根目录]
#   或 chmod +x install.sh && ./install.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CWD="$(pwd)"

green()  { printf '  \033[32m[OK]\033[0m %s\n' "$1"; }
gray()   { printf '  \033[90m[--]\033[0m %s（已存在，跳过）\n' "$1"; }
yellow() { printf '\n\033[33m==> %s\033[0m\n' "$1"; }
red()    { printf '\033[31m%s\033[0m\n' "$1"; }

# 自动检索 payload 目录（从任意位置运行均可）：
# 依次看 脚本同目录 → 当前目录 → 当前目录下 docs-system-installer/payload
# → 脚本父目录的 docs-system-installer/payload → 兜底：向下最多 3 层找含 rule.md 的 payload 目录
find_payload() {
  local c
  for c in \
    "$SCRIPT_DIR/payload" \
    "$CWD/payload" \
    "$CWD/docs-system-installer/payload" \
    "$SCRIPT_DIR/../docs-system-installer/payload"; do
    [ -d "$c" ] && [ -f "$c/rule.md" ] && { echo "$c"; return 0; }
  done
  local p hit=""
  while IFS= read -r p; do
    if [ -f "$p/rule.md" ]; then hit="$p"; break; fi
  done < <(find "$SCRIPT_DIR" "$CWD" -maxdepth 3 -type d -name payload 2>/dev/null)
  [ -n "$hit" ] && { echo "$hit"; return 0; }
  return 1
}

# ---- 解析项目根 ----
PROJECT_ROOT="${1:-}"
if [ -z "$PROJECT_ROOT" ]; then
  PROJECT_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
fi
yellow "安装目标项目：$PROJECT_ROOT"

PAYLOAD_DIR="$(find_payload || true)"
if [ -z "$PAYLOAD_DIR" ]; then
  red "找不到 payload 目录（已在脚本目录、当前目录及其下 docs-system-installer/payload 检索）"
  red "请确认 payload/（含 skills/ hooks/ rule.md）存在，或把安装包完整解压后重试"
  exit 1
fi
yellow "使用 payload：$PAYLOAD_DIR"

# ---- 1. 安装 skills ----
echo "1) 安装 skills 到 .claude/skills/"
for s in "$PAYLOAD_DIR"/skills/*/; do
  name="$(basename "$s")"
  dest="$PROJECT_ROOT/.claude/skills/$name/SKILL.md"
  if [ -f "$dest" ]; then
    gray "skill $name"
    continue
  fi
  mkdir -p "$(dirname "$dest")"
  cp "$s/SKILL.md" "$dest"
  green "skill $name"
done

# ---- 2. 安装 Stop hook ----
echo "2) 安装 Stop hook（任务收尾归档提醒）"
if [ -f "$PAYLOAD_DIR/hooks/archive-reminder.sh" ]; then
  mkdir -p "$PROJECT_ROOT/.claude/hooks"
  cp "$PAYLOAD_DIR/hooks/archive-reminder.sh" "$PROJECT_ROOT/.claude/hooks/archive-reminder.sh"
chmod +x "$PROJECT_ROOT/.claude/hooks/archive-reminder.sh" 2>/dev/null || true
green "hook 脚本 archive-reminder.sh"

HOOK_CMD='cd "${CLAUDE_PROJECT_DIR:-.}" && bash .claude/hooks/archive-reminder.sh'
SETTINGS="$PROJECT_ROOT/.claude/settings.json"
mkdir -p "$PROJECT_ROOT/.claude"

# 用 python 优先、jq 兜底合并 settings.json（保留已有 permissions）
if command -v python3 >/dev/null 2>&1; then
  python3 - "$SETTINGS" "$HOOK_CMD" <<'PY'
import json, sys, os
path, cmd = sys.argv[1], sys.argv[2]
data = {}
if os.path.exists(path):
    try:
        data = json.load(open(path, encoding="utf-8"))
    except Exception:
        data = {}
hooks = data.setdefault("hooks", {})
stop = hooks.setdefault("Stop", [])
for grp in stop:
    for h in grp.get("hooks", []):
        if h.get("command") == cmd:
            print("  [--] settings.json Stop hook（已存在，跳过）")
            break
    else:
        continue
    break
else:
    stop.append({"hooks": [{"type": "command", "command": cmd}]})
    json.dump(data, open(path, "w", encoding="utf-8"), ensure_ascii=False, indent=2)
    print("  [OK] settings.json 已注册 Stop hook")
PY
elif command -v jq >/dev/null 2>&1; then
  if grep -q "$HOOK_CMD" "$SETTINGS" 2>/dev/null; then
    gray "settings.json Stop hook"
  else
    tmp="$(mktemp)"
    [ -f "$SETTINGS" ] || echo '{}' > "$SETTINGS"
    jq --arg cmd "$HOOK_CMD" '.hooks.Stop += [{hooks:[{type:"command",command:$cmd}]}]' "$SETTINGS" > "$tmp" && mv "$tmp" "$SETTINGS"
    green "settings.json 已注册 Stop hook"
  fi
else
  red "  需要 python3 或 jq 来合并 settings.json，请手动添加 Stop hook：$HOOK_CMD"
fi

else
  echo "  [--] payload/hooks/ 不存在，跳过 hook 安装（v1 不含 hook，v2 恢复）"
fi

# ---- 3. 铁律写入 CLAUDE.md 与 AGENTS.md ----
echo "3) 写入铁律到 CLAUDE.md / AGENTS.md"
MARKER="任务收尾必归档"
for fname in CLAUDE.md AGENTS.md; do
  fpath="$PROJECT_ROOT/$fname"
  if [ -f "$fpath" ] && grep -q "$MARKER" "$fpath"; then
    gray "$fname 铁律"
    continue
  fi
  if [ ! -f "$fpath" ]; then
    [ "$fname" = "CLAUDE.md" ] && printf '# CLAUDE.md\n\n' > "$fpath" || printf '# AGENTS.md\n\n' > "$fpath"
  fi
  printf '\n' >> "$fpath"
  cat "$PAYLOAD_DIR/rule.md" >> "$fpath"
  green "$fname 已写入铁律"
done

# ---- 完成 ----
green ""; yellow "安装完成 ✔"
cat <<'EOF'
后续使用：
  · 开始新项目：对 AI 说「开始 XX 项目」，或直接说「在 docs 下给 XX 建目录」（init-project-docs）
  · 功能/修复完成：AI 会用 write-worklog 把日志写到 docs/{项目}/logs/
  · 发现待处理问题：AI 会用 write-issue 登记到 docs/{项目}/issues/open/
  · Claude Code 归档提醒 hook：v1 暂未包含（v2 发布后生效）
EOF
echo ""

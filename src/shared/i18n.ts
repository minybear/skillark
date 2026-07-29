export type Lang = "en" | "zh";

export type BiStr = { en: string; zh: string };

export const I18N = {
  nav: {
    en: ["Overview", "Skill Library", "Agents", "Workspaces", "Operations"],
    zh: ["概览", "技能库", "代理", "工作区", "操作记录"],
  },
  navIcons: ["⌂", "◇", "◎", "▦", "↗"],

  brandSub: { en: "Local skill control", zh: "本地技能管理" },
  eyebrowProject: { en: "PROJECT FOUNDATION", zh: "项目基础" },
  eyebrowLocal: { en: "LOCAL FIRST", zh: "本地优先" },
  eyebrowFoundation: { en: "FOUNDATION", zh: "基础架构" },
  eyebrowDiscovery: { en: "AGENT MANAGEMENT", zh: "代理管理" },
  eyebrowSignals: { en: "DETECTION SIGNALS", zh: "检测信号" },
  eyebrowRoadmap: { en: "ROADMAP", zh: "路线图" },

  h1: { en: "One library. Every agent.", zh: "一个库，适配每个代理。" },
  vaultReady: { en: "The core workflow is ready.", zh: "核心工作流已就绪。" },
  heroDesc: {
    en: "Import a local Skill, deploy it to multiple agents, verify drift, and uninstall safely with a complete local audit trail.",
    zh: "导入本地 Skill 后，可向多个代理分发、验证漂移并安全卸载，全程保留本地审计记录。",
  },
  discover: { en: "Discover agents", zh: "发现代理" },
  rescan: { en: "Rescan", zh: "重新扫描" },
  scanning: { en: "Scanning this device…", zh: "正在扫描本机…" },
  cancel: { en: "Cancel", zh: "取消" },
  next: { en: "Next:", zh: "下一步：" },
  vaultFoundations: { en: "FOUNDATIONS", zh: "基础" },
  vaultTitle: { en: "SkillArk foundation status", zh: "SkillArk 基础状态" },

  foundationReady: { en: "Built into the first commit", zh: "构建于首个提交" },
  foundationMeta: { en: "4 / 4 ready", zh: "4 / 4 就绪" },
  agentsHeading: { en: "Agents on this device", zh: "本机代理" },
  agentsPageTitle: { en: "Agent Detection", zh: "代理探测" },
  agentsPageSub: {
    en: "Multi-signal detection of supported coding agents installed on this device.",
    zh: "通过多信号检测本机已安装的受支持编程代理。",
  },
  notScanned: { en: "Not scanned yet", zh: "尚未扫描" },
  idleMsg: {
    en: "Run a scan to check CLI, configuration, Skill directories, and active processes. Nothing leaves this device.",
    zh: "运行扫描以检查命令行工具、配置文件、Skill 目录和运行中的进程。所有数据仅留在本机。",
  },
  idleMsgAgents: {
    en: "Click the scan button above to detect installed coding agents. Each agent is evaluated across multiple signals: CLI presence, config directory, skill directory, and running process.",
    zh: "点击上方扫描按钮检测已安装的编程代理。每个代理通过多信号评估：命令行工具、配置目录、Skill 目录和运行中进程。",
  },
  errorPrefix: { en: "Desktop discovery could not run: ", zh: "桌面发现无法运行：" },
  likelyFound: { en: "likely found", zh: "个可能匹配" },
  totalAgents: { en: "agents total", zh: "个代理总计" },

  levelDetected: { en: "Detected", zh: "已检测到" },
  levelProbable: { en: "Probable", zh: "可能匹配" },
  levelPossible: { en: "Possible", zh: "待确认" },
  levelNotFound: { en: "Not found", zh: "未检测到" },
  signalsMatched: { en: "signals matched", zh: "个信号匹配" },
  noPath: { en: "No Skill path resolved", zh: "未解析到 Skill 路径" },

  skillPath: { en: "Skill Path", zh: "Skill 路径" },
  executable: { en: "Executable", zh: "可执行文件" },
  writable: { en: "Writable", zh: "可写入" },
  writableYes: { en: "Yes", zh: "是" },
  writableNo: { en: "No", zh: "否" },
  writableUnknown: { en: "Unknown", zh: "未知" },
  notResolved: { en: "Not resolved", zh: "未解析" },

  signalType: { en: "Signal", zh: "信号" },
  signalWeight: { en: "Weight", zh: "权重" },
  signalDetail: { en: "Detail", zh: "详情" },
  matched: { en: "Matched", zh: "匹配" },
  notMatched: { en: "Not matched", zh: "未匹配" },

  showSignals: { en: "Show signals", zh: "展开信号" },
  hideSignals: { en: "Hide signals", zh: "收起信号" },

  roadmapHeading: { en: "v0.1 delivery progress", zh: "v0.1 交付进度" },
  sidebarNote: { en: "Your managed skills stay on this device.", zh: "受管的 Skill 仅保留在本机。" },
  langHint: { en: "Click to switch", zh: "点击切换语言" },
  comingSoon: { en: "Coming soon", zh: "即将上线" },

  customAgent: { en: "Custom Agent", zh: "自定义代理" },
  addCustom: { en: "Add Custom Agent", zh: "添加自定义代理" },
  customName: { en: "Display Name", zh: "显示名称" },
  customType: { en: "Agent Type (slug)", zh: "Agent 类型（标识）" },
  customCli: { en: "CLI Name (optional)", zh: "命令行名称（可选）" },
  customConfigDir: { en: "Config Directory (optional)", zh: "配置目录（可选）" },
  customSkillDir: { en: "Skill Directory (optional)", zh: "Skill 目录（可选）" },
  customSkillOverride: { en: "Skill Path Override (optional)", zh: "Skill 路径覆盖（可选）" },
  save: { en: "Save", zh: "保存" },
  deleteBtn: { en: "Delete", zh: "删除" },
  savedAgents: { en: "Saved Custom Agents", zh: "已保存的自定义代理" },
  noCustom: { en: "No custom agents configured.", zh: "未配置自定义代理。" },
  saved: { en: "Saved!", zh: "已保存！" },
  nameRequired: { en: "Display name is required", zh: "显示名称为必填项" },
  typeRequired: { en: "Agent type is required", zh: "Agent 类型为必填项" },

  // v0.1 Library / Deploy / Operations
  libraryTitle: { en: "Skill Library", zh: "技能库" },
  librarySub: {
    en: "Import a skill once, then deploy it to any agent. Skills are stored as content-addressed copies.",
    zh: "导入一次技能，即可分发到任意代理。技能以内容寻址副本形式存储。",
  },
  importDir: { en: "Import directory", zh: "导入目录" },
  importZip: { en: "Import ZIP", zh: "导入 ZIP" },
  pathLabel: { en: "Path", zh: "路径" },
  pathPlaceholderDir: { en: "C:\\path\\to\\skill (contains SKILL.md)", zh: "C:\\path\\to\\skill（包含 SKILL.md）" },
  pathPlaceholderZip: { en: "C:\\path\\to\\skill.zip", zh: "C:\\path\\to\\skill.zip" },
  importing: { en: "Importing…", zh: "导入中…" },
  libraryEmpty: { en: "No skills yet. Import a directory or ZIP to begin.", zh: "暂无技能。导入目录或 ZIP 开始。" },
  hashShort: { en: "Hash", zh: "哈希" },
  versionLabel: { en: "Version", zh: "版本" },
  deploy: { en: "Deploy", zh: "分发" },
  confirmDelete: { en: "Delete this skill and its snapshots?", zh: "删除该技能及其快照？" },

  deployTitle: { en: "Deploy", zh: "分发" },
  deploySub: {
    en: "Choose a skill, pick target agents, and install a managed copy into each agent's skill directory.",
    zh: "选择一个技能，勾选目标代理，将受管副本安装到每个代理的技能目录。",
  },
  pickSkill: { en: "1 · Pick a skill", zh: "1 · 选择技能" },
  pickAgents: { en: "2 · Pick target agents", zh: "2 · 选择目标代理" },
  scanAgents: { en: "Scan agents", zh: "扫描代理" },
  chooseMode: { en: "3 · Mode", zh: "3 · 安装模式" },
  chooseScope: { en: "2b · Scope", zh: "2b · 分发范围" },
  modeCopy: { en: "Copy (default)", zh: "复制（默认）" },
  modeJunction: { en: "Junction (advanced)", zh: "Junction（高级）" },
  buildPlan: { en: "4 · Build plan", zh: "4 · 生成计划" },
  executePlan: { en: "Execute plan", zh: "执行计划" },
  recomputePlan: { en: "Recompute plan", zh: "重新计算计划" },
  requiresConfirmation: {
    en: "This plan overwrites existing targets. Review the conflicts before executing.",
    zh: "该计划将覆盖已存在的目标，执行前请确认冲突。",
  },
  noWritableAgents: { en: "No writable agent skill directories found.", zh: "未发现可写的代理技能目录。" },
  selectSkillFirst: { en: "Pick a skill first.", zh: "请先选择技能。" },
  resultsHeading: { en: "Results", zh: "结果" },
  succeeded: { en: "succeeded", zh: "成功" },
  failed: { en: "failed", zh: "失败" },
  ok: { en: "OK", zh: "成功" },
  junctionFallbackNotice: {
    en: "Junction failed for one or more targets. Security software or local policy may block links. You can review a Copy retry plan for only the failed targets.",
    zh: "一个或多个目标的 Junction 安装失败，可能被安全软件或本机策略拦截。可仅针对失败目标生成 Copy 重试计划并确认后执行。",
  },
  buildCopyRetryPlan: {
    en: "Review Copy retry plan",
    zh: "生成 Copy 重试计划",
  },
  targetPath: { en: "Target", zh: "目标路径" },
  conflictCol: { en: "Conflict", zh: "冲突" },

  operationsTitle: { en: "Operations", zh: "操作记录" },
  operationsSub: {
    en: "Every write action (import, install, uninstall) is audited here.",
    zh: "每次写操作（导入、安装、卸载）都会在此留痕。",
  },
  operationsEmpty: { en: "No operations yet.", zh: "暂无操作记录。" },
  opType: { en: "Type", zh: "类型" },
  opStatus: { en: "Status", zh: "状态" },
  opStarted: { en: "Started", zh: "开始时间" },
  opError: { en: "Error", zh: "错误" },

  refresh: { en: "Refresh", zh: "刷新" },
  loading: { en: "Loading…", zh: "加载中…" },
  verifyAll: { en: "Verify all", zh: "全部验证" },
  view: { en: "View", zh: "查看" },
  filesHeading: { en: "Files", zh: "文件" },
  skillMdHeading: { en: "SKILL.md", zh: "SKILL.md" },
  enable: { en: "Enable", zh: "启用" },
  disable: { en: "Disable", zh: "禁用" },
  disabledTag: { en: "disabled", zh: "已禁用" },
  missing: { en: "missing", zh: "丢失" },

  workspacesTitle: { en: "Workspaces", zh: "工作区" },
  workspacesSub: {
    en: "Global deploys to each agent's user-level skill directory; project workspaces scope skills to a project folder.",
    zh: "全局工作区分发到每个代理的用户级技能目录；项目工作区把技能限定到某个项目目录。",
  },
  newProject: { en: "New project workspace", zh: "新建项目工作区" },
  projectName: { en: "Project name", zh: "项目名称" },
  projectRoot: { en: "Project root path", zh: "项目根目录" },
  projectRootPh: { en: "D:\\code\\my-project", zh: "D:\\code\\my-project" },
  create: { en: "Create", zh: "创建" },
  noWorkspaces: { en: "No project workspaces yet.", zh: "暂无项目工作区。" },
  globalWs: { en: "Global (user-level)", zh: "全局（用户级）" },
} as const;

// Signal type display names
export const SIGNAL_LABELS: Record<string, BiStr> = {
  path_executable: { en: "CLI / PATH executable", zh: "命令行可执行文件" },
  config_directory: { en: "Config directory", zh: "配置目录" },
  skill_directory: { en: "Skill directory", zh: "Skill 目录" },
  running_process: { en: "Running process", zh: "运行中进程" },
  user_override: { en: "User override", zh: "用户手动指定" },
};

export function pick(lang: Lang, pair: BiStr): string {
  return pair[lang];
}

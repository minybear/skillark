# Agent 自动探测 POC 设计

## 1. POC 目标

验证 SkillArk 能够在 Windows 上可靠发现 Agent，并得到可供分发的 Skill 目录。POC 不追求一次覆盖所有安装方式，必须提供用户校正机制。

## 2. 不应依赖单一信号

每个 Agent 使用多信号评分：

- PATH 中存在 CLI
- 已知可执行文件候选路径存在
- 已知配置目录存在
- 已知 Skill 目录存在
- 正在运行的进程存在
- 用户手动确认

示例评分：

| 信号 | 分数 |
|---|---:|
| CLI 可执行文件可调用 | 40 |
| 配置目录存在 | 25 |
| Skill 目录存在 | 25 |
| 正在运行 | 10 |

- 70 以上：自动标记 detected
- 40–69：标记 probable，要求用户确认
- 40 以下：不默认展示，放入“可能存在”

## 3. Adapter 合约

```rust
pub trait AgentAdapter {
    fn kind(&self) -> AgentKind;
    fn display_name(&self) -> &'static str;
    fn detect(&self, ctx: &DetectionContext) -> Vec<AgentCandidate>;
    fn validate_configuration(&self, candidate: &AgentCandidate) -> ValidationResult;
    fn resolve_global_skill_path(&self, candidate: &AgentCandidate) -> Option<PathBuf>;
    fn resolve_project_skill_path(
        &self,
        candidate: &AgentCandidate,
        project_root: &Path,
    ) -> Option<PathBuf>;
}
```

## 4. DetectionContext

```text
home_dir
app_data
local_app_data
program_files
program_files_x86
path_entries
running_processes
registry_snapshot
wsl_distributions（后续）
```

## 5. 路径策略

不要把路径写死在 UI 或业务层。每个 Adapter 返回候选目录，并附带：

- 来源：default / config / detected / user
- 置信度
- 是否存在
- 是否可写
- 是否需要创建

Agent Skills 规范定义 Skill 内容格式，但不强制所有客户端使用同一个安装路径。因此 SkillArk 必须把路径识别视为 Agent Adapter 的职责，并保留手动配置。

## 6. 首期验证矩阵

每个 Agent 至少验证：

1. 默认安装
2. 自定义安装目录
3. CLI 在 PATH 中但 Skill 目录不存在
4. Skill 目录存在但应用未运行
5. 目录只读
6. 用户填写错误目录
7. 中文 Windows 用户名
8. 多个安装实例

## 7. POC 输出

命令或测试页面展示：

```json
{
  "kind": "codex",
  "displayName": "Codex",
  "confidence": 85,
  "executablePath": "...",
  "globalSkillPath": "...",
  "signals": [
    {"type": "path_executable", "matched": true},
    {"type": "skill_directory", "matched": true}
  ],
  "writable": true
}
```

## 8. 验收标准

- 四个目标 Agent 中至少三个可自动或半自动配置
- 不因未安装某 Agent 而报错
- 扫描总耗时小于 3 秒
- 扫描过程可取消
- 所有路径在写入数据库前规范化
- 用户手动设置优先于自动检测结果

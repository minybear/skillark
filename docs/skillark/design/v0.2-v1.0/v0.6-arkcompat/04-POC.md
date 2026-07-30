# SkillArk v0.6 POC 与证伪计划

## 1. POC 目标

先建立 100×Agent 的兼容实测矩阵和 Lockfile 最小 Schema；不先设计抽象评分算法。

POC 的目标不是证明方案正确，而是尽早找到会迫使架构或范围改变的证据。

## 2. POC 不做

- 不追求完整 UI。
- 不接入所有来源/Agent/平台。
- 不把手工样本测试替代自动化基准。
- 不因为已有代码投入而降低通过阈值。

## 3. 可证伪假设

| 编号 | 结论 | 样本 | 指标 | 通过阈值 | 失败决策 |
|---|---|---|---|---|---|
| H-06-01 | Capability Profile 对测试矩阵中的失败原因预测 precision ≥90%。 | 至少 100 个 Skill × 4 Agent × 2 平台组合，记录真实结果。 | 预测 precision/recall、unknown 比例。 | precision ≥90%，unknown ≤20%。 | 未达标则只显示确定性结构差异，不给百分比分数。 |
| H-06-02 | 声明式转换能解决至少 60% 的高频可修复不兼容。 | 从失败矩阵选前 50 个可修复案例。 | 转换后通过率、语义回归失败。 | 通过率 ≥60%，严重语义回归 0。 | 否则限制为建议 Patch，不自动生成派生版本。 |
| H-06-03 | Lockfile 在干净环境恢复内容 Hash 一致率 100%。 | Windows/WSL/macOS 测试机各 20 个项目快照。 | 恢复 Hash 一致率和失败可解释率。 | Hash 100%；失败均有明确缺失依赖。 | 任一静默不一致阻断发布。 |

## 4. 测试夹具

- `fixtures/valid/`：正常、最小、完整、多语言、大目录。
- `fixtures/invalid/`：缺字段、路径异常、损坏包、版本漂移。
- `fixtures/adversarial/`：越界、混淆、提示注入、资源耗尽、恶意扩展。
- `fixtures/failure-injection/`：断网、超时、数据库失败、文件替换失败、进程崩溃。
- `fixtures/golden/`：人工标注的期望输出和 Hash。

## 5. 实验输出

每个实验保存：

```text
experiment-id/
├── README.md
├── dataset-manifest.json
├── environment.json
├── raw-results.jsonl
├── summary.md
└── decision.md
```

`decision.md` 必须写明：通过、失败、部分通过，以及对范围/架构的实际修改。

## 6. 退出条件

- 所有关键 H 已得到可重复结果。
- 失败样本已转化为限制、错误码或明确不支持项。
- POC 代码中可复用部分有测试，试验性捷径未进入 Domain Core。
- 没有“后续再验证但先按当前方案开发”的高风险假设。

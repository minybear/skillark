# SkillArk v1.0 POC 与证伪计划

## 1. POC 目标

不是新功能 POC，而是 v1.0 Release Rehearsal：密钥恢复、供应链最小权限、分阶段发布、回滚、Hotfix 和弃用演练。

POC 的目标不是证明方案正确，而是尽早找到会迫使架构或范围改变的证据。

## 2. POC 不做

- 不追求完整 UI。
- 不接入所有来源/Agent/平台。
- 不把手工样本测试替代自动化基准。
- 不因为已有代码投入而降低通过阈值。

## 3. 可证伪假设

| 编号 | 结论 | 样本 | 指标 | 通过阈值 | 失败决策 |
|---|---|---|---|---|---|
| H-10-01 | v1.0 RC 在四周内满足 crash-free sessions ≥99.5%，核心操作成功率 ≥99%。 | 分阶段 RC，至少 500 活跃设备周。 | 崩溃率、导入/部署/更新/恢复成功率。 | ≥99.5%/≥99%，无数据损坏。 | 不达标继续 RC，不发布 Stable。 |
| H-10-02 | 12 个月支持周期可在预计维护容量内完成。 | 基于 Beta 缺陷、安全事件和发布工时建模。 | 每月维护工时、未处理高优缺陷。 | 维护占用 ≤总开发时间 40%，P0 按 SLA 清零。 | 超过则缩短矩阵或调整支持周期，不增加承诺。 |
| H-10-03 | Extension API v1 可实现三个不同类型扩展且无需私有逃生接口。 | 官方+第三方 Connector、Agent、Scanner 各一。 | 私有接口调用数、兼容测试。 | 0 个未声明主程序接口依赖。 | 否则继续标 experimental，不冻结 v1。 |

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

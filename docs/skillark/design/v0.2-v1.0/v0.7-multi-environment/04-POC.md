# SkillArk v0.7 POC 与证伪计划

## 1. POC 目标

先实现 EnvironmentAdapter + WSL helper，在测试目录完成发现/复制/验证；macOS 单独做构建、签名、Keychain 和 Symlink Spike。

POC 的目标不是证明方案正确，而是尽早找到会迫使架构或范围改变的证据。

## 2. POC 不做

- 不追求完整 UI。
- 不接入所有来源/Agent/平台。
- 不把手工样本测试替代自动化基准。
- 不因为已有代码投入而降低通过阈值。

## 3. 可证伪假设

| 编号 | 结论 | 样本 | 指标 | 通过阈值 | 失败决策 |
|---|---|---|---|---|---|
| H-07-01 | EnvironmentAdapter 可让 ≥80% 的部署服务代码保持平台无关。 | 对 v0.1-v0.6 部署代码进行依赖扫描。 | 平台条件分支比例、重复代码。 | 核心服务平台条件分支 <20%。 | 否则缩小抽象，仅统一 Plan，不强求统一底层操作。 |
| H-07-02 | WSL 受控 helper 在支持矩阵中部署成功率 ≥98%。 | WSL1/2、Ubuntu/Debian、运行/停止、中文 Windows 路径。 | 安装/验证/卸载成功率。 | ≥98%，且 0 次写错发行版。 | 低于阈值则仅支持 WSL2 + 指定发行版。 |
| H-07-03 | macOS Beta 获得至少 50 名有效测试者且核心周活 ≥30%。 | 四周封闭测试。 | 激活、首次分发、周活和缺陷密度。 | ≥50 激活，周活 ≥30%。 | 不足则 v1.0 仅标注 macOS Preview。 |

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

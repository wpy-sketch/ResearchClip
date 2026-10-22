# ResearchClip v4.0.0 发布验收

本版范围、源码审查、功能对照、数据与权限、实际命令、失败记录和Windows人工清单见[高级功能独立验收报告](ADVANCED_FEATURE_REPORT.md)。本版不新增其他业务功能。

## 发布准备

版本元数据统一4.0.0；SQLite schema11，001–011保持原字节，无新增依赖。指南18章114示例31图，随包提供完整验收报告、模型许可证及固定清单。WebClipper1.0.2、MathOCR1.0.4及Qwen既有资源保留原版本与摘要。

安装器、最终载荷摘要、公开发布和全部附件匿名校验将在实际完成后记录。未完成的发布步骤不计PASS。人工验收、偶发保存I/O与WebView退出告警等未解决项不会因发布而改写为通过。

## 实际最终检查（2026-10-22）

前端428测试、Rust212通过/7忽略、类型/Lint/Clippy/格式/构建均退出0；受影响Windows GUI最终53组分别通过，首次失败与修正过程保留在独立报告。真实NSIS隔离安装/忙碌保护/卸载4组退出0，14份既有用户资料摘要不变；签名NotSigned。

原始EXE：27878400字节，SHA256 `b09249e72b7b246570b3419873968b35445af8b61c33b251ee4a7574eda67591`。最终安装器SHA256 `f4234e72d46f183b88c88ad9230b799c51ea8497fb47c6fc1ec5aaddacef3001`。见[机器结果](evidence/v400-checks.json)及[安装器结果](evidence/v400-installer.json)。


## 公开发布回执（2026-10-22）

[v4.0.0正式Release](https://github.com/wpy-sketch/ResearchClip/releases/tag/v4.0.0)已公开，固定源码标签为`532e7f39aeb8aeda0e7c9899b5ba35ea3c960eb4`。独立GitHub读取确认latest=v4.0.0、draft=false、prerelease=false、10个附件齐全；全部匿名下载并核对字节数和SHA256通过：[发布证据](evidence/v400-publication.json)。发布前后28个旧Release、206个旧附件ID、标签/源提交、附件名/大小/摘要逐项一致；不是重新下载所有历史大模型。

大EXE由[一次性校验运输任务](https://github.com/wpy-sketch/ResearchClip/actions/runs/39731475974)还原已测试字节，不重新编译；固定源码、草稿ID、压缩和原始摘要、解压长度均受检查。原始EXE、服务端与匿名下载摘要一致。运输分支只在精确SHA租约匹配时删除，临时工作流已移除，运行记录仍保留：[运输证据](evidence/v400-transfer.json)。

本版离线指南51个文件，18章114项示例31图，含完整高级功能验收报告、模型清单和许可证；ZIP CRC、路径和内置链接检查通过。WebClipper1.0.2、MathOCR1.0.4与旧附件逐字节相同，原有Qwen大模型资源入口和全部历史附件保持。独立验收报告中的历史偶发I/O、退出告警及人工/硬件待验项继续保留。

发布前一次最终检查因自动审批服务账户用量限制未执行；用户要求继续后，经正常审批重试完成，未绕过审批。此基础设施中断不作为应用检查PASS，也未影响此前已核验的安装载荷。

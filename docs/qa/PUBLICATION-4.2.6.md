# v4.2.6 正式发布回执

[正式Release](https://github.com/wpy-sketch/ResearchClip/releases/tag/v4.2.6)，ID424393052；源码与标签冻结在e396b2d6b402ccaa9534688b69ea79a8c0c8a77d。正式非预发布，附件321项。

GitHub Actions [继承任务](https://github.com/wpy-sketch/ResearchClip/actions/runs/40429889500)（job121944730671）成功，继承上一版313项无冲突附件，当前8项新附件单独上传。发布前逐项比较名称、大小和SHA256。

[匿名公开下载验证](https://github.com/wpy-sketch/ResearchClip/actions/runs/40430551977)（run40430551977，job121946874162）成功。321附件均从公开下载URL实际读取字节并核对SHA256与大小，下载不携带Authorization。验证工作流提交e396b2d6b402ccaa9534688b69ea79a8c0c8a77d。完整机器回执见[公开校验](evidence/v426-public-verification.json)。

此前45个Release、4090项原附件的ID、名称、大小及摘要前后保持一致，历史核对PASS。旧标签和附件未覆盖；重名SOURCE/SHA按原名称和字节置于对应版本metadata ZIP，历史清单保留归档成员定位，不以旧链接代替附件内容。当前便携包、NSIS、指南以及历代功能组件均在本版附件中。

安装器SHA256：91896da7a44d0b6e23da40e8a0745fcf8f20d5fd938287db4064ba156466e70c，17,000,692字节，未签名。实际隔离安装、运行中保护、卸载及载荷核对5组通过，结束无残留安装。前端730、Rust301（8默认忽略，另显式执行真实Zotero集成1项）、Windows原生133、指南14、发布基础设施23项通过。指南32章241练习65图；完整修改和测试边界见[开发验收](RELEASE-4.2.6.md)、[修改文件](CHANGED-FILES-4.2.6.md)和[机器证据](evidence/v426-editor-checks.json)。

独立Zotero10.0.6资料库已验证三类型真实CRUD、401/428/412、复读和原PDF摘要不变；官方授权GUI及真实只读群组/非作者权限仍需人工验收。测试key仅在专用profile注入，不能冒称真实用户授权通过；原生UI的模拟HTTP夹具另行标记。物理多屏DPI仍为人工项。首次隔离Zotero的Word集成安装日志及事前hash缺失的边界保留在验收报告，未作猜测恢复。

默认ResearchClip资料只读审计27文件0变化。SQLite19无新增迁移或依赖；无直接Zotero数据库写入、私有笔迹转换、原PDF修改、Markdown导出PDF或独立备份/恢复产品。v4.2.6草稿和待核实意图仅保留于会话内，关闭前需处理或明确放弃，不将其说成持久双向同步。

本回执在源码标签之后以独立文档提交保存；源码标签及发布附件保持不变。按用户新授权，完成本版后开始Step16/v4.2.7，持久同步队列与冲突解决另行实现和验收。
最终历史审计首轮遇到TLS连接中断，原日志保留；使用相同冻结字段逐项只读重试后45个Release/4090附件全部通过，未修改任何旧版本资产。

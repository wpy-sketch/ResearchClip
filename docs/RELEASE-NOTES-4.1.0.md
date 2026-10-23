# ResearchClip v4.1.0

本版完成 Research IDE Step00 架构审查与 Step01 统一科研资源模型，在现有工程增量开发。

- 保留现有截图、Markdown、项目和文献ID；新增PDF附件/位置、标准批注、私有手写/笔迹、代码工程、证据及Agent会话的类型化元数据和双向关联查询。
- SQLite升级到schema12，沿用迁移前校验与一致快照；失败事务回滚、未知未来版本拒写，不直接写Zotero SQLite。
- Markdown沿用唯一文件服务，增加内容指纹和经明确目标选择的Rust重连接口；原图和文档正文不在迁移期间重写。
- 完成隔离旧库升级与Windows真实打开/引用/重启六组验证。前端429测试、Rust230通过/7忽略、构建/Clippy/文档检查通过。

已知限制：额外advanced-native编辑器回归在窗口关闭环节超时，整套未通过；多DPI、真实用户库和安装升级须按验收报告验证。新增元数据不代表PDF阅读器、Zotero插件、HarmonyOS手写、代码终端或Agent执行已实现；新资源暂不加入WebDAV同步。

现有v4.0.0可选WebClipper/MathOCR/模型资源继续从其对应发布附件获取，本版没有升级或重新分发这些运行时。没有Markdown导出PDF或独立备份/恢复功能。

详细设计：[资源模型](architecture/RESOURCE_MODEL.md)。实际测试、未通过项与人工步骤：[Step01验收](qa/RESOURCE_MODEL_STEP01.md)、[发布记录](qa/RELEASE-4.1.0.md)。

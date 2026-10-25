# v4.1.6 正式发布回执

日期2026-10-25。已发布[ResearchClip v4.1.6](https://github.com/wpy-sketch/ResearchClip/releases/tag/v4.1.6)，非草稿、非预发布，Release ID423096328。应用源码与标签固定b5216dee01e7516632279ec2b16905aa6d34d6fa；[45文件完整Diff](https://github.com/wpy-sketch/ResearchClip/commit/b5216dee01e7516632279ec2b16905aa6d34d6fa)。后续回执提交仅更新文档，不重写应用标签。

261附件全部实际匿名下载，逐项大小和SHA-256与冻结本地清单匹配；不是仅检查GitHub元数据。只读[匿名验证40153471942](https://github.com/wpy-sketch/ResearchClip/actions/runs/40153471942)，job121049709920，下载不携带Authorization；本地接收器验证工作流身份、源提交、成功状态及每项摘要。[完整回执](evidence/v416-public-verification.json)。

[继承40153139643](https://github.com/wpy-sketch/ResearchClip/actions/runs/40153139643)，job121048696347，实际核对并填入253个无冲突旧附件，只修改当前授权草稿。历史35版1210个原附件前后审计通过，ID、名称、大小、摘要及标签未变。全部原文件字节在本版261附件中表示：223个原历史运输卷不变，旧便携包/安装器/指南等直接保留，重名SOURCE/SHA按原名保留于各版本Metadata.zip。V416-HISTORY-INVENTORY.json逐项定位，未用旧链接替代内容，未重复嵌套数GB归档。见[下载说明](../RELEASE-COMPONENTS.md)。

Step07完成：离线公式实时预览与模板、八种定理环境、可配置公式/定理/图表编号、稳定标签和交叉引用。原LaTeX和未知源码不因模式切换重写；旧图文格式版本1、截图关联和SQLite12保持兼容。指南24章、149练习、36幅离线图示、12条阅读路线。

最终检查：612项前端/73文件、237项Rust（7项既有忽略）、53组本轮Windows原生、45组浏览器、2组真实记事本、4组最终实际安装通过。论文与原生指南焦点修复后8组复测，其他45组为本轮受影响回归。见[验收与文件设计](MARKDOWN_STEP07.md)、[构建安装记录](RELEASE-4.1.6.md)、[实际测试证据](evidence/v416-editor-checks.json)。

最终NSIS SHA-256：825f557ca52537b3cdadb047d5f7a67b511e5633fbbdc36fc6c9df966e3d13c5。三份许可证实际安装且摘要匹配；16个既有AppData文件只读哈希不变，隔离安装已移除。未启动正式程序访问用户真实资料；测试使用隔离数据库与练习PNG。

未通过/未验证边界：首次直接cargo启动没有嵌入页面，改Tauri构建后通过；指南焦点回归修复并重跑通过。物理中文IME、多显示器DPI/系统放大镜、真实资料仍需人工验收；先前锁屏OS OLE拖放保护失败未冒充通过，受控WebView导入真实Rust路径已通过。没有原生PDF位置选区、Zotero写回或新Agent业务。安装器未签名，既有Vite大chunk提示保留。

无数据库迁移、新应用权限、遥测、Markdown导出PDF或独立备份恢复系统。两个新增直接声明是已安装GFM包的相同锁定版本，所有既有依赖版本未升级。Step07开发、测试、v4.1.6发布及附件验证完成后停止。下一步仅需解锁Windows环境完成上述人工验收；其他阶段需用户另行请求。

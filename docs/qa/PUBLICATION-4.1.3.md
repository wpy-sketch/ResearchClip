# v4.1.3 正式发布凭据

[正式Release](https://github.com/wpy-sketch/ResearchClip/releases/tag/v4.1.3)，ID422602338，正式版、非预发布。最终应用源码和v4.1.3标签：d89ae4ad452d9c9df8c66d1436ad950b99d47333。完成日期2026-10-24（Asia/Hong_Kong）。

## 交付与真实验证

- Markdown固定/浮动工具栏、选区操作、独立撤销、快捷键隔离、字面查找替换、源码/安全预览；原有文件、图文块和工作区继续使用。超限替换保留原文与历史，不虚报成功。SQLite保持12，无新迁移。
- npm run check退出0，69文件525测试，类型/lint/生产构建通过。npm ci退出0；新增remark-gfm4.0.1及18个新包的完整许可，既有锁定版本无升级。
- Rust fmt/test/Clippy均退出0，236通过、7忽略。
- 浏览器工具栏11组、指南14组通过；Windows原生工具栏10组、命令8组、工作区7组、旧资料6组均退出0，共31组。真实文件保存/进程重启、图片与注释/原图哈希、独立撤销及失败保护通过。
- 隔离正式构建与NSIS实际安装/卸载4组退出0。运行中覆盖保护、全部新增许可、16个既有AppData文件只读哈希保持、测试安装清理通过。最终包使用src-tauri/target/v413-distribution独立目录，未关闭用户运行中的旧程序。
- 指南21章127例33图、9条路线，离线指南ZIP371成员和便携ZIP3成员CRC检查通过。

安装包SHA-256：5474151b0c6daf77d81517a6c6c9ebed35c95d83a0ae88fdf8951fd74276daa6；未签名，需要Windows WebView2。

## 243个附件及历史完整性

243个附件全部上传并核对GitHub大小/摘要。包含此前32版463个原附件的精确内容：通过v4.1.1的226附件在相同223卷中；v4.1.2的235个无冲突附件直接附上，SOURCE/SHA两份原文件按原名/原字节存于Metadata.zip。V413-HISTORY-INVENTORY.json逐项定位，参见[下载与重组](../RELEASE-COMPONENTS.md)。旧运行时按原说明配置，不冒充已安装。

[继承工作流40077565672](https://github.com/wpy-sketch/ResearchClip/actions/runs/40077565672)完成235附件复制/校验。[公开校验40079518967](https://github.com/wpy-sketch/ResearchClip/actions/runs/40079518967)实际匿名下载全部243附件，逐一匹配SHA、来源提交与清单；环境GitHub-hosted Ubuntu24.04，下载不发送Authorization，不能称为本机完整下载。接收器与本地冻结清单逐项匹配退出0；[逐附件证据](evidence/v413-public-verification.json)包含摘要、环境、运行和日志校验。

最终本地发布器退出0，发布前后32版463旧附件的ID/大小/SHA、标签名和发布状态保持。v4.1.2已先完成正式发布和237项匿名校验，见[前版回执](PUBLICATION-4.1.2.md)。

草稿阶段额外长度边界检查发现RangeError，修复后重新验证并构建；仅本任务未发布候选af70b6c在期望SHA保护下更新到d89ae4a。旧候选全部产物保留，238个相同附件不动，只替换5个候选文件。网络中断和GitHub临时草稿标签均已恢复；未改任何历史正式标签或发布附件。

## 人工验收与已知限制

物理Windows中文输入法、多显示器DPI/跨屏恢复仍未人工验收；自动组合事件/CSS缩放不能替代。复杂嵌套格式状态保守识别；图文模式查找限当前字段，结构修改重新开始文字历史。额外旧workflow-smoke因前台窗口保护失败、历史advanced-native退出超时均保留为未通过，不以新工具栏测试替代。

用户真实资料、macOS、Zotero写回、HarmonyOS压感及未接入PDF/代码/Agent模块没有本轮成功结论。详见[工具栏验收和Windows步骤](MARKDOWN_TOOLS_4.1.3.md)。本轮完成后停止，不自动进入后续开发。

# v4.1.8 正式发布回执

2026-10-25，[ResearchClip v4.1.8](https://github.com/wpy-sketch/ResearchClip/releases/tag/v4.1.8)已正式发布，非草稿、非预发布，Release ID423204020。应用标签固定01721e7da184e549dc4ba42145b58af891bce0ed；[74文件完整变更](https://github.com/wpy-sketch/ResearchClip/commit/01721e7da184e549dc4ba42145b58af891bce0ed)。本回执仅追加文档，不改写应用标签。

Step09目标已完成：独立本地CitationService，@标题/作者/DOI/键/BibTeX字段补全，文献管理与键冲突提示，截图/公式来源，稳定ID详情，显式文末参考列表，.bib导入导出，以及限定实例/library的BBT键映射接口。Zotero未连接，无自动DOI获取或伪造核实信息。旧[@key]保留，需要明确替换才能参加稳定ID列表；新引用身份不依赖标题或显示键。

技术与修改文件见[Step09报告](CITATIONS_STEP09.md)和[引用契约](../architecture/CITATIONS_STEP09.md)。复用既有文献表、解析器、文件服务和编辑会话；新增schema14仅存外部键映射，001–013不改，13→14隔离迁移验证一致快照、旧记录及失败回滚。新5个受限主窗口IPC，无通用fs/shell、无新增第三方包。

273附件全部实际匿名下载，逐项大小/SHA-256与冻结清单匹配；不是仅依赖GitHub元数据。[只读验证40171926809](https://github.com/wpy-sketch/ResearchClip/actions/runs/40171926809)，job121104383459，不携带下载Authorization；本地接收器核对工作流身份、源提交及全部摘要，见[完整证据](evidence/v418-public-verification.json)。

[继承40171553786](https://github.com/wpy-sketch/ResearchClip/actions/runs/40171553786)，job121103259616，核对并填入265个旧附件，只写当前授权草稿。此前37版1738个原附件前后审计通过，ID/名称/大小/摘要/标签未变；223个历史运输卷原字节保留，旧安装器/便携包/指南仍附带，同名SOURCE/SHA在对应版本Metadata.zip中按原名保存。V418-HISTORY-INVENTORY.json逐项定位，不用旧链接替代文件内容，不重复嵌套数GB归档。见[下载说明](../RELEASE-COMPONENTS.md)。

实际检查均退出0：npm干净安装；前端644项/76文件，类型/lint/构建；Rust251通过、7忽略，格式与Clippy通过；76组本轮Windows原生、56组浏览器、2组真实记事本读取、16项发布脚本测试。生产NSIS安装/占用保护/卸载/清理4组通过，三份许可摘要匹配，16个既有AppData文件只读摘要不变，隔离安装已移除。见[机器证据](evidence/v418-editor-checks.json)及[发布检查](RELEASE-4.1.8.md)。

NSIS SHA-256：d86206d21b78aa86d5ca2ebd1c4d45698d618dd3912b4a35d17c251e3934ca22。安装包未签名，Vite原有大块提示保留。指南26章/167练习/41图，目录与路线自然排序；新增3幅实际Windows引用流程图。原创SVG科研烧瓶重配色并生成桌面图标，侧栏/安装器同源，无Google商标或新增图标包。

已解决：原生补全菜单曾被裁切，改为面板内浮层并复测；原生拖动测试的目标在滚动裁切区外，滚入可见区后保留实际鼠标拖动、顺序及完整撤销断言，回归通过，没有改动拖动业务。夹具同名选择改用稳定ID，API/导入/Clippy问题均修复后重跑。

未通过/未验证：没有遗留失败的自动化用例，7个Rust忽略项不计通过。物理中文IME、多屏DPI/系统放大镜与既有锁屏OS OLE问题仍需人工；系统.bib保存/打开选择器已实际测试，旧Markdown重定位选择器仍是手工边界。真实用户库未升级，测试只用隔离资料与练习文献，不冒称真实DOI元数据已核实。

Windows人工验收按指南26.8：录入练习条目并搜索，正文/可视片段/旧图注补全与撤销，改名后点击旧标记，截图与带唯一标签公式记录来源，导出到新.bib后重新预览，检查文末列表Diff并撤销，保存退出重开。安装前保存退出旧实例，先用独立练习资料核对schema14；旧版拒写新库，不重置或覆盖真实资料。

范围限制：BBT仅预留映射接口；引用列表不是完整CSL排版器，来源状态仍待人工核对；本地文献信息不宣称加入旧WebDAV修订协议。没有Markdown导出PDF、独立备份恢复或新Zotero业务。

Step09开发、测试、v4.1.8发布及附件验证完成。按用户本轮明确顺序，接着实施Step10原生PDF阅读器并发布v4.1.9；其他路线图阶段没有自动实施授权。

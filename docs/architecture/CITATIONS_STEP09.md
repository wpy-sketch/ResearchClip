# Step09：本地 CitationService 契约

2026-10-25，基点c4facc4。先实现本地引用，发布v4.1.8后才进入用户指定Step10；本阶段无Zotero拉取。复用literature_references、ReferenceInput、原BibTeX解析/导入预览、截图与文档关联，不创建第二文献表或ID系统。

新引用为普通Markdown链接 `[@key](researchclip://literature/32位稳定ID)`。展示键是插入时快照，真正身份来自ID；标题/作者/键修改不重写正文，点击始终读取该ID最新详情。未知ID显式报错。旧`[@key]`保留，不静默迁移；生成参考列表发现旧键标记时提示先用补全替换，避免猜错同名键。代码、数学和HTML内的标记不计引用。@补全绑定当前编辑会话及源码/可视片段/旧caption，不在代码/公式/邮箱/IME组合中触发；异步结果失效不改原文。

CitationService独立编排现有本地文献API：标题/作者/DOI/引用键/BibTeX字段搜索、稳定ID解析、导入导出与未来BBT映射。不会因@输入而联网或伪造DOI。既有显式Crossref查询保留，结果仍标待人工核对；手工、导入、修订状态均不冒称核实。

参考文献列表由正文中的有效稳定引用按首次出现去重，显式生成/更新到文末受管区；修改受管区前显示差异并确认，未知ID或过量引用拒绝生成，外部内容不执行。普通编辑器可读，保留原文与一键撤销。公式来源使用稳定论文标签的可读注释`<!-- rc-citation-source: eq:label -->`和普通引用链接，公式LaTeX不改；重复或缺失公式标签拒绝记录。截图来源继续使用原screenshot_references。

schema14只新增citation_key_mappings，键为provider+instanceId+libraryId+externalKey，值为既有稳定文献ID。预留BetterBibTeX适配接口，关联冲突拒绝覆盖；不把Zotero item key/citation key当全局ID，不连接或写Zotero数据库。沿用升级前一致快照与事务迁移，隔离13→14演练、失败回滚，001–013不改。

BibTeX仍由原有界解析器处理，未知字段保留，宏/拼接等不支持语法明确拒绝，不执行LaTeX；本地.bib导出只在系统选择目标后create_new，已有文件拒绝覆盖，文本/条数有界。新增IPC仅main窗口，输入类型和大小校验，不增加通用fs/shell。修复旧全局文献插入入口的会话归属，避免写到另一工作区。

测试本地CRUD/键冲突/来源状态、稳定标题改名、文本作用域与撤销、代码/公式排除、受管文末区、公式来源、映射库隔离、BibTeX往返/未知字段/错误语法、迁移/重启、真实Windows导入导出/选择器边界。图标采用原创几何科研符号和彩色分区，不使用Google商标；指南目录按章节自然顺序，新增引用练习与真实运行截图。无Markdown→PDF或独立备份恢复功能。

颜色设计参考[Material3色彩角色](https://m3.material.io/styles/color/the-color-system)：前景与容器成对，低噪声背景配合少量强调色。实现为原有SVG科研符号的原创重配色；不引入Google图标包、商标或字体。Tauri已安装CLI从同一SVG生成桌面图标，侧栏同源引用。

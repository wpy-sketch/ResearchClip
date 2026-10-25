# Step09 本地科研引用与文献补全

目标仅Step09与v4.1.8；发布完成并验证后才进入用户已指定的Step10。基点c4facc4，先写[引用契约](../architecture/CITATIONS_STEP09.md)，再复用现有literature_references、BibTeX解析器、截图/文档来源关联、Markdown文件服务及编辑会话。

## 已实现与设计

独立CitationService编排本地搜索、ID解析、BibTeX预览/导出和预留BBT映射。@补全绑定源码、近所见即所得片段或旧图片注释的当前选区，邮箱/代码/数学/HTML/组合输入排除；取消不改文稿，原目标变化拒绝写入。新标记[@key](researchclip://literature/稳定ID)，键为显示快照，身份不依赖标题/键；点击预览可看最新详情，缺失ID报错。旧[@key]原样保留，不自动猜配。

公式来源引用既有唯一公式标签，追加可读说明而不改LaTeX；截图沿用原关联表。参考文献按正文首次稳定引用去重，受管区有版本注释，先显示Diff再确认，旧键/未知ID/破损区域拒绝生成，一步撤销。原图文块ID与相对图片路径保留。

schema14只新增citation_key_mappings，限定provider/instance/library/externalKey，值指向既有文献ID；映射冲突不覆盖。001–013不改，隔离13→14演练升级前快照、保留条目与失败回滚，未来版本拒写。预留接口不代表连接Zotero，不读写其数据库。新增5个受限main IPC，无通用fs/shell。

复用原手工管理和BibTeX导入预览，增加粘贴预览及系统目标选择后的create_new导出；原始未知字段保留，不支持宏/拼接时明确拒绝。所有来源仍待人工核对，未触发DOI自动联网。历史/文献等新增信息不宣称纳入旧WebDAV协议。

新增文件：references/CitationService、citation-model与测试、citation-insertion、CitationCompletion/Chooser/Tools/Link及样式；Ruststorage/citations、commands/citations、014迁移。增量调整原References会话归属、References.store、搜索、MarkdownPanel/Preview、schema状态、受限权限、图标与指南。无新增第三方包，既有依赖不升级。

## 本轮实际结果

Windows11、Edge与WebView2实际执行，命令退出码均为0；见[机器证据](evidence/v418-editor-checks.json)。npm ci --ignore-scripts干净安装324包，既有锁定依赖未变；npm run check类型/lint/644项测试（76文件）/前端生产构建通过。npm run format:rust、test:rust、check:rust通过：251项、7忽略，未用忽略项冒充通过。

node scripts/with-rust.mjs tauri build --debug --no-bundle通过。引用原生11组包含系统.bib保存/打开选择器、未知字段/冲突、标题键改名稳定ID、公式与截图来源、可视片段/旧图注作用域、真实进程重启；旧原生回归65组通过，总76组。七组浏览器脚本共56组通过，其中引用7组明确为受控本地IPC，不代替持久性；三类弹窗共90个主题/窗口/CSS缩放组合通过，异步旧稿拒绝和原会话绑定也通过。

pwsh plain-markdown-reader.ps1 -Citations两组真实记事本RichEdit读取通过，源UTF-8字节不变。16项发布基础设施测试通过；npm run check:docs和git diff --check通过，本地链接、权限和已发布迁移完整性均检查。测试只用新隔离数据库、可读练习文献/PNG，没有升级真实用户资料。

npm run desktop:bundle生产NSIS构建退出0，使用src-tauri/target/v413-distribution；installer-smoke.ps1 -RunId v418-distribution -TargetDir src-tauri/target/v413-distribution实际安装/占用保护/卸载/清理4组通过，三份许可匹配，16个既有AppData只读摘要不变，隔离安装已移除。NSIS SHA-256：d86206d21b78aa86d5ca2ebd1c4d45698d618dd3912b4a35d17c251e3934ca22，未签名。

已解决的失败：夹具DirEntry API、未使用导入和Clippy转换修正；同名文档测试改为稳定ID目录选择；原生补全菜单被裁切，改为面板内浮层并复测。原生图文拖动测试的目标矩形在滚动裁切区外，命中另一摘要栏；滚入可见区域后保留真实鼠标拖动、顺序和完整撤销断言，回归通过，未改拖动业务功能。Vite原有大块提示保留。

指南第26章新增10练习、3张隔离Windows实测图；26章/167练习/41图，目录分类与路线自然排序。原创SVG烧瓶重配色，侧栏/桌面/安装器同源，已有5种图标文件由安装的Tauri CLI生成；无Google商标或新图标包。图示使用练习文献，不冒充真实DOI元数据。

273附件正式发布并全部匿名下载校验通过，旧37版1738附件前后终审通过。见[正式回执](PUBLICATION-4.1.8.md)。

## 待人工验收与限制

自动化组合事件不替代物理中文IME；多屏DPI/系统放大镜和既有锁屏OS OLE拖放仍需解锁桌面。系统.bib导入/导出选择器在隔离目录实际测试，不代表真实用户资料已升级。安装包未签名；本地文献详情在另一台机器需要相应本机条目，.md可保留可读标记。旧[@key]需明确替换；不是完整CSL期刊排版器。单列表200篇、单BibTeX1MiB/200条，正文沿用2MiB，键映射仅接口。

按指南26.8在隔离练习资料手工核对录入/搜索/补全、改名后详情、截图与公式来源、文件交换、列表Diff/撤销和重启。没有PDF导出、独立备份恢复或Zotero拉取。下一前置条件为v4.1.8发布与附件验证完成，随后才能执行用户指定的Step10。

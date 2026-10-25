# Step10 PDF阅读器验收记录

本轮目标：在既有可停靠工作区增量接入本地只读PDF阅读，并发布v4.1.9。复用React/Tauri2/FlexLayout、原资源目录、SQLite、Markdown文件与会话服务；不改原PDF页结构、不增加PDF导出、Zotero写回或独立备份恢复。开发、构建及安装验收已完成，正式发布及279附件实际匿名下载核验已完成，见[正式回执](PUBLICATION-4.1.9.md)。

## 实现与设计

选型记录先于业务代码：[PDF_READER_STEP10](../architecture/PDF_READER_STEP10.md)。固定新增pdfjs-dist6.4.299，实际核对Node要求、官方稳定版本、安装类型与许可证。Worker同版本本地打包；不导入QuickJS、脚本管理器、表单动作或外部附件处理器。CSP只允许可信本地WASM，不开放JavaScript unsafe-eval。

原生PdfDocumentService只接受系统文件框或已登记稳定ID。128MiB/文件、3会话、256MiB输入总量；校验内容哈希、禁止覆盖、按块读取、进度、取消与授权释放。受管副本命名用内容哈希，资源身份继续使用原SQL随机稳定ID。旧受管PDF路径逐级检查并验证摘要；相同内容导入复用既有Document/PdfAttachment，已有关系不改。

PdfViewerSession与Markdown状态分开，EditorRegistry注册真实PDF组件；WorkspaceManager复用现有标签、拆分、隐藏和恢复。连续单双列、适宽适页/自定义缩放、视图旋转、目录、缩略图、选区复制、逐页全文搜索和页码跳转均已实现。没有Markdown就没有空白Markdown栏。图片、代码和Agent业务未提前扩展。

页面按视口观察器渲染，远处/隐藏时清零Canvas并释放文字层。串行绘制队列、单页100万像素/缩略图2万、每会话800万/全局2400万像素形成应用缓存预算；PDF.js内部字体/解析/解码堆不能承诺绝对硬内存隔离。最多2000页/边长20000PDF点、图像3200万像素、文字层单页2万项/1Mi字符、搜索8Mi字符/200结果。单次引擎等待120秒，等待输入密码除外；超预算明确提示不完整/失败。

schema15仅加pdf_reading_positions，存页索引、旋转、缩放策略、单双页、归一化滚动偏移和乐观版本/摘要。001–014不改。隔离14→15验证旧目录JSON/稳定ID、SQLite一致快照，模拟失败验证回滚和旧版本保留。前端状态契约同步15，并新增与Rust当前schema一致性测试。旧数据库没有被删除或重置，真实用户库未用于迁移演练。

## 修改范围

主要文件为src/features/pdf下PdfDocumentService、PdfViewerSession、PdfPageLocation、PdfCanvasCache、PdfViewer、PdfOpenButton及测试/样式；src-tauri/src/services/pdf.rs、commands/pdf.rs、storage/pdf.rs和015迁移。资源创建抽出内部共用函数，仍在同一事务中生成原有ID和依赖边；主窗口新增6个受限IPC，其他窗口没有权限。

工作区manager/registry宿主接入，布局恢复保留映射后的活动标签；desktop退出前刷新PDF阅读位置。通用document.open命令接入PDF。library.ts同步新schema；指南新增27章/10练习/3实测图，合计27章177练习44图。沿用v4.1.8原创科研烧瓶图标，PDF工具条复用应用明暗主题与圆角/焦点样式。

scripts/prepare-pdf-assets.mjs仅复制阅读所需静态资源及许可，排除QuickJS与Liberation字体；Helvetica使用系统字体。原PDF.js/Adobe CMaps/Foxit/OpenJPEG/JBIG2/QCMS/ICC相关许可证随安装器和便携包分发，见[许可](../licenses/pdfjs-NOTICES.txt)。用于QA的ReportLab4.4.10/pypdf6.4.0安装在artifacts隔离目录，不是应用依赖；练习PDF由测试生成器创建，未作为产品导出功能。

## 实际检查与已解决问题

Windows WebView2本地Worker/Canvas/文字层平台实验通过，真实PDF图像和文字实际渲染，destroy后Worker释放。实际原生验收已验证中文CMap、嵌入字体、OpenJPEG本地WASM、文字鼠标选区和原生剪贴板、四向旋转、目录缩略图/搜索、多PDF、重启恢复、错误密码/受限/损坏文件、长文加载与搜索取消、画布释放、Markdown并排和缩放。本轮实际npm run check退出0：657项/79文件；Rust259通过/7忽略，格式与Clippy退出0，干净安装通过。45组Windows原生、14组指南浏览器、20项发布基础设施测试通过；NSIS4组安装/占用保护/卸载/清理通过，四份许可匹配，16个真实AppData文件只读摘要不变。见[机器证据](evidence/v419-editor-checks.json)。[正式回执](PUBLICATION-4.1.9.md)已追加。

发现并修复：Tauri实验绝对中文路径被当成URL，改项目相对路径；Node22 Windows递归cp异常退出，改固定文件逐个复制；6.4权限为Set、旧isEvalSupported/文档destroy接口已移除，按实装类型修正；未渲染Canvas默认300×150占用，改初始0；原布局恢复重置选中标签，改稳定映射后的选中身份；前端仍拒schema15导致旧素材列表不可用，补契约并回归。指南测试1437端口被Windows排除，改可配置的27437并重跑，不改系统规则。保留失败记录，不以删除断言获得通过。

## 已知边界与Windows人工验收

安装器未签名；Vite大块提示保留。PDF为只读，PDF自有脚本/附件/表单交互不执行；没有新增PDF截图、标准批注写回或手写业务。扫描PDF无文字时不会自动OCR。超大图像与稀有字体/编码可能不完整，已有PDF内容保留；目录显示最多1000项/16层。阅读位置按视图相对比例恢复，旋转后不保证同一文字行固定在顶端。

人工使用独立练习资料按指南27.8演练，核对实际显示内容、中文复制、复杂字体/扫描/加密PDF、关闭后重开及原件摘要；再核对物理多屏DPI、系统放大镜、输入法和长时间内存。程序化CSS缩放不是物理DPI验收，练习PNG不是实际屏幕截图。原有7项Rust忽略项不计通过。

## 下一步前置条件

先完成v4.1.9源码/标签、安装器、279附件正式发布与逐项匿名下载验证，再进入已明确授权的Step11业务提交。Step11只读接口预审已在隔离工作材料中开始，不混入本版业务。

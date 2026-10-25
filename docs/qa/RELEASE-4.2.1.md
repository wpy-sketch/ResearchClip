# v4.2.1 发布与验收

本轮仅PDF显示清晰度、Zotero配置与使用、自然缩放布局、指南和图标；见[先行诊断/设计](../architecture/PDF_QUALITY_ZOTERO_421.md)。SQLite保持16，无新增应用依赖，001–016迁移不改；不改用户文献或PDF原件，不进入Step12。

修改集中在features/pdf/PdfPage、PdfRenderGeometry、observePdfDensity、PdfCanvasCache、PdfViewerSession、engine、ZoteroSetup及现有来源面板；Rust只增主窗口限定配置说明/连接检测IPC，复用客户端和取消会话。设置/指南/原UI状态增加章节定位，样式使用容器/rem断点，原创SVG及五份原生图标重生成，发布脚本只追加4.2.1版本映射。

## 实际结果

以下命令本轮实际执行并退出0，见[机器证据](evidence/v421-editor-checks.json)：

- npm ci --ignore-scripts：327包干净安装；npm run check：类型/ESLint/构建及84文件671项前端测试通过。
- npm run format:rust、npm run test:rust、npm run check:rust：格式/Clippy通过，Rust272通过、7忽略；忽略项不是通过。
- Windows内嵌资源调试构建、正式NSIS构建通过；最终63组原生：工作区7、命令8、资源6、引用11、PDF13、清晰度5、Zotero配置/来源13。
- node scripts/guide-reader-smoke.mjs v421-final：14组浏览器检查通过；28章、191练习、51图实际加载，章节顺序及小节/步骤编号由单元测试检查，明暗主题、12/16/20px字体、480–1440宽度、CSS缩放75–200%与撤销保留通过。
- 发布基础设施20项通过；001–016与v4.2.0逐文件git diff退出0，无数据库迁移。
- npm run desktop:bundle 与 pwsh -NoProfile -File scripts/installer-smoke.ps1 -RunId v421-delivery -TargetDir src-tauri/target/v413-distribution：5组安装、占用保护、PDF打开候选/默认项保留、卸载与清理通过，四份许可摘要一致；最终回归及安装前后记录的23个默认AppData文件摘要保持。

隔离Windows原生清晰度练习：250%与模拟DPR2下独立源像素MAE=0，26576采样，画布4194304像素且每块不超过1048576；DPR动态变化、旋转/离屏释放和窄屏导航均通过。模拟DPR/CSS缩放不是物理屏幕验收。

NSIS SHA-256：621149919c5d35dd2618ff6a34dab08c58eb425cab335902fb9c9e0cf89d6c4b。测试期间先发现DPR变化没有触发重绘，已补检测并重跑；修复了API配置解析器的lint问题，保留旧失败截图/日志，不删断言或伪造结果。

## 人工边界

真实安装Zotero10.0.3，独立资料配置的健康GET返回200/API3/schema44/实例标识。本轮可读取真实Zotero空库的可访问性界面，但激活输入返回GetCursorPos拒绝访问0x80070005，用户明确选择保留此项人工验收，本轮不再尝试输入；未把HTTP练习或读取界面当作Zotero设置/双击链路通过。仅本机读取，不申请写入key，不直接访问Zotero SQLite。

物理多屏DPI/Windows放大镜、旧WebView2、长期大文件内存和原始低清扫描需人工。本阶段不支持Zotero标准批注读写、传入批注页码、任意API端口/远程URL；普通PDF阅读不依赖API成功。安装器未签名。

## 发布目标

此前40版2575项原附件已冻结为286伴侣文件；本版目标291附件。正式发布和所有匿名下载验证完成前不声称交付成功。

## 修改文件与设计对应

- PDF绘制：PdfViewer/PdfPage/PdfRenderGeometry/observePdfDensity/PdfCanvasCache/PdfViewerSession/engine及相关单元、Windows像素对照脚本。
- Zotero：Rust commands/zotero、lib/build/capabilities新增两项受限IPC；ZoteroSetup、zoteroApi、现有来源面板与设置入口，复用取消/只读客户端。
- 界面与指南：styles.css、pdf.css、zotero-setup.css、ui-store章节目标/Sidebar/GuideView，USER_GUIDE、4幅原生图、SVG和五份原生图标。
- 发布：包版本/锁文件仅应用版本，既有发布脚本/工作流追加421映射、历史清单和验证证据。

数据库、文件存储、旧文献ID/引用、截图原件、Markdown正文与撤销服务保持原实现。后续阶段需要新授权；本版本发布完成后停止。

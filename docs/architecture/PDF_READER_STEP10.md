# Step10：原生工作区PDF阅读器选型与边界

2026-10-25，Step09发布已完成，基点26fe113。只实施用户明确要求的Step10并发布v4.1.9；不新增PDF导出、Zotero写回或独立备份恢复。

## 选型先行

优先Mozilla PDF.js，采用固定pdfjs-dist6.4.299：[官方稳定发布](https://github.com/mozilla/pdf.js/releases/tag/v6.4.299)、[同版本源码](https://github.com/mozilla/pdf.js/tree/v6.4.299)、[官方API](https://mozilla.github.io/pdf.js/api/draft/module-pdfjsLib.html)。npm registry实际核对version6.4.299、Apache-2.0、Node>=22.13或>=24；本机22.22.2满足。先安装固定版本，检查随包类型/实现/许可并做Windows WebView2本地Worker、Canvas、文字层实验，实际WebView2实验通过；记录见Step10验收报告。不开远程CDN，不混用主库与worker版本，不为最新批量升级既有依赖。

浏览器iframe/embed依赖内置PDF插件和不同WebView行为，难以保证Tauri权限、文字层和生命周期；本轮不采用。PDFium等原生引擎引入额外DLL/封装/平台维护，本轮不增加。沿用React、Tauri2、FlexLayout、SQLite与现有资源标识，不重写PDF解析器、不引入完整IDE。

## 进程、会话与资源

PdfDocumentService负责受限系统文件选择、受管只读副本、哈希/大小校验、可取消加载、有限二进制分块授权。PDF原件只读，不改页结构，不接受WebView任意路径。明确告知导入工作区副本，使用pdfs内唯一/内容寻址名称且禁止覆盖；同内容复用既有稳定资源，拷贝与数据库错误明确报告，不删除唯一资料。

复用research_entities的Document(PDF)与PdfAttachment及既有依赖边。内容仍是PDF文件，不放SQLite，不另建PDF全局ID系统。schema15只增加文档阅读位置元数据，以文档ID/内容哈希/格式版本/乐观版本校验，0基页索引、旋转视图、缩放策略和页内滚动偏移；未知未来版本拒绝写。001–014不改，先隔离14→15迁移、升级前一致快照与回滚。

PdfViewerSession每文档独立，拥有PDF.js加载任务、worker、页面/搜索任务和阅读位置；与Markdown/图片业务状态隔离。EditorRegistry注册真实PDF类型，WorkspaceManager只处理标签、拆分、隐藏/关闭/恢复及生命周期；布局只保存稳定ID，不存路径、密码或文件授权。不打开Markdown就不生成Markdown面板。

连续单列/双列阅读、适宽/适页/自定义缩放、90度旋转、目录、缩略图、文本选择/复制、全文搜索和页码导航均在现有停靠面板内；PDF与Markdown可左右或上下拆分。隐藏暂停屏外绘制，关闭取消任务、清零Canvas、释放文字层/观察器/文件授权并销毁worker；退出前刷新阅读位置，失败可见。

## 不可信PDF与性能预算

仅解析渲染，不执行PDF JavaScript，不创建脚本/表单动作执行器，不自动打开外部链接或内嵌附件。6.4.299已移除旧isEvalSupported参数，不传递虚构选项；不导入QuickJS/sandbox/脚本管理器，通过CSP禁止JavaScript动态求值。同源本地worker与资源。若字体/解码需要可信本地WASM，仅允许wasm-unsafe-eval，不开放JavaScript unsafe-eval或任意网络。复制尊重PDF权限；加密需用户输入，密码仅会话内存，未提供/错误/取消均明确处理；损坏及不支持内容显示错误，不冒充成功。

输入上限128MiB，活跃PDF最多3且输入合计最多256MiB；IPC每块最多1MiB，读入显示实际进度并可取消。页面按视口懒渲染，远处Canvas清零，不为全文构建高分辨率缓存；全局画布预算、缩略图与文字缓存有界，超限降低栅格密度或明确失败。PDF.js内部解析/字体/解码堆无法由应用精确硬限，不能宣称绝对内存隔离；页数、图像尺寸、任务超时与worker销毁形成附加限制。实装每页最多100万像素、缩略图2万像素、每会话800万像素、全局2400万像素；最多2000页、边长20000PDF点。文字层单页最多2万项/1Mi字符，搜索最多8Mi字符/200结果。单次引擎等待120秒，密码等待除外。

全文搜索逐页可取消，显示扫描进度；结果/文字预算超限时明确显示不完整，不静默遗漏。页面尺寸、原有旋转与CropBox由PDF.js的getViewport处理；阅读位置保存视图内归一化偏移，旋转后保留页码与相对位置，不承诺锁定同一文字行，也不输出批注坐标。

## 验收与失败回退

依次验证依赖/worker平台实验、Rust授权/路径/字节预算/迁移、TypeScript页面状态/坐标/缓存/取消、真实浏览器及Windows WebView2多PDF/目录/文字/搜索/旋转/缩放/重启。使用自建可读练习PDF，不伪造渲染图，不碰真实资料。PDF解析失败保留资源与原件，关闭后可重试；无法验证的限制与物理DPI项明确记录。安装器/离线指南附新增依赖许可与实际截图，v4.1.9继续完整继承所有此前发布附件。

# Step06 高效Markdown内容编辑验收

最终状态：v4.1.5已正式发布，255附件实际匿名校验及旧34版955附件终审通过；583项前端、237项Rust/7忽略、同轮45组原生、38组浏览器、2组记事本和4组安装通过。OLE锁屏、物理IME/多DPI及PDF原生接口的限制仍保留。见[正式回执](PUBLICATION-4.1.5.md)。

目标：在v4.1.4正式完成后增量开发表格、图片和代码操作，发布v4.1.5后停止。基点e9654db；[设计](../architecture/MARKDOWN_STEP06_DESIGN.md)。当前SQLite12，无迁移或新权限，无PDF导出/独立资料备份恢复。

## 完成与设计边界

表格：8×8网格选择，最多101行含表头/20列，单元格、行列增删、对齐、Tab/Shift+Tab及末格新行；只显式确认时局部生成标准GFM。转义管道，多行/NUL粘贴拒绝，复杂不规则/超预算原文保留。快照验证文档ID、字段和原文，过时确认可见失败，不能覆盖并发新稿。取消/未改确认不写入。原撤销/CRLF和未知扩展保持。

图片：原拖入/素材导入链、快捷插图、标准source alt/title，整个图片与连续说明复制/移动/删除；旧ImageBlock复制使用既有newId及独立受管assetName，截图关联不变，原图不改。旧私有格式仍version1，图题保存在现有caption可读首行，无可选元数据升级。属性、复制和拖动沿用撤销；旧结构删除仍清理资源/旧历史，不声称可撤销删除。尺寸策略：仅视图自适应/弹窗比例，不写非标准宽高；路径仍为相对assets。未知引用式图片不自动改写。

代码：只改围栏语言token，保留info后缀及正文；lowlight3.3.0(MIT)与highlight.js11.11.2(BSD-3-Clause)，common+MATLAB/LaTeX注册，HAST只渲染文本/span，不使用innerHTML，不执行代码。32Ki字符高亮预算，未知/超预算明示纯文本。复制复用受限copy_ocr_text，1MiB限制/错误可见；行号与超过12行折叠为视图状态，不写进.md。同步滚动观察内容高度，折叠后重新测量。

最近格式限6个封闭动作名，仅保存偏好；Ctrl+Shift+T/I仅正文，排除IME、重复及截图配置/实际注册组合，保留既有全局命令。空笔记插图自动“注释：”；已有PDF截图从素材库插入也沿用共同路径。原生PDF阅读器、PDF定位截图接口仍未实现，明确返回MODULE_UNAVAILABLE，不冒充通过。

## 主要修改文件

src/features/document新增content-model、TableDialog、ContentTools、CodeBlock、code-highlight、recent-formats、样式与核心单测；修改MarkdownPanel/Preview、FormattingToolbar、DocumentView、VisualEditor、store、滚动测量及store测试。沿用原API/图文块/工作区；没有第二正文或保存服务。guide及导航测试、混合稿、原生图、版本/锁、两份许可/NSIS资源、发布运输/继承/验收脚本同步更新。

依赖实际安装只新增lowlight/highlight.js两包，已有包版本未变；npm ci --ignore-scripts实际通过。已核对安装API、MIT/BSD全文，安装包/便携包携带lowlight-NOTICES.txt（含两许可），原FlexLayout/GFM许可继续保留。

## 本轮实际测试（阶段记录）

早期npm run check实际581项/71文件通过，类型/lint/build退出0；Rust fmt/test/clippy退出0，237通过、7忽略。最后截图组合保护补充实际注册值后，针对浏览器用例另行重跑，最终构建结果见发布检查。

浏览器markdown-content：6组通过，网格/Tab/增删/对齐/转义、多行粘贴拒绝、并发过时确认、图片组操作、语言/高亮/折叠、最近格式与配置/注册冲突、IME及30个主题/宽度/CSS缩放组合。无桌面IPC时复制显式失败，未冒充复制成功。Step05浏览器7组回归通过。

Windows markdown-content-native v415-final：7组通过，实际复杂稿表格编辑/网格插入及保存字节核对；image alt/title、复制/移动/删除及撤销；安全高亮、语言、行号/折叠、系统剪贴板；旧图文稿独立复制/截图关联/PNG哈希、退出重启重读。受控WebView File drop通过实际Rust解码/SQLite/插图/保存，生成持久相对PNG和空笔记注释区；明确不等同于系统OLE拖放。另实跑Step05原生7组、工具栏10组、命令8组、工作区7组、资源兼容6组，共45组。最终小范围修改后的针对性重跑、安装和指南结果待发布检查补充。

普通编辑器检查：plain-markdown-reader实际启动隐藏的独立Windows记事本，读取隔离副本。首次UIAutomation隐藏控件无法取文，未记通过；随后在同一已证明属于本次测试的窗口，用实际RichEdit WM_GETTEXT读到中文、公式、表格、相对图路径和代码，UTF-8文件SHA不变，关闭仅此测试窗口。v415-win32两组通过；不以浏览器预览冒充外部编辑器。

前期失败保留：代码折叠后fixture_text不再可见导致测试定位器失效，改为先验证目标内容、再固定同一代码块定位，未改业务断言。Windows剪贴板把14个LF转为CRLF；实际比对逻辑字符的SHA仍完全一致，.md文件本身的原换行另作字节核对。首轮全套lint发现测试脚本未使用import，修正后重跑。

## 未通过/未验证与人工验收

真实Windows WinForms/OLE文件拖放本轮失败：image-drop-input前台归属检查返回LockScreenBackstopFrame（屏幕锁定），未向遮挡窗口继续发输入。该脚本前两组实际剪贴板/锁占用检查通过，整体仍FAIL。保留artifacts/import-v415-import/report.json；不能用通过的DOM File路径代替真实OLE验收。需要解锁桌面后，在隔离资料重新拖PNG/JPEG/WebP、保存、重开并核对原文件哈希。

物理IME、多DPI/跨屏、真实科研资料、macOS、Zotero/HarmonyOS未执行。已有额外物理截图的前台保护失败不改写为通过。PDF选区模块未安装/未接入；可用既有区域截图把PDF画面存入素材库，再插入获得注释区，不能声称PDF坐标捕获已验证。

人工步骤：复制练习稿及assets；表格网格/Tab/对齐并取消/确认/撤销；图片alt/title和两段说明整体操作，旧图文稿核对稳定ID/原PNG；15行代码换语言/高亮/折叠/复制，普通编辑器读取完整代码；测试截图组合冲突、物理IME、字号/窗口和保存关闭重开。已发布后不自动继续其他阶段；下一条件仅为上述人工环境与未来单独授权。

构建、安装/发布实际结果见[发布检查](RELEASE-4.1.5.md)。

发布前增加相邻GFM表格边界回归：旧CommonMark投影把图片说明后的表格误当连续说明段，先由失败单测复现，再识别独立table边界，避免误移动/删除；保留表格原片段。相关单测、浏览器和原生整体组操作会重跑，最终数量/安装包另记发布检查。修复前安装候选尚未发布，已在artifacts/v415-candidate-before-table-boundary保留字节和旧验收记录。

另一个发布前失败单测复现了原导入协调器通过动态活动代理释放busy导致的串会话问题：导入期间切换标签后，原会话会停留adding。现捕获既有DocumentStore实例作为异步任务归属，插入/失败/清理都使用原实例；不改原导入或存储服务。新增单测通过，原生受控File drop期间会真实切换另一个已保存面板，验证目标仍在原稿、另一稿不变。最终全套预期583项，以实跑结果为准。

最终全套583项/71文件退出0；图文边界修复后24项投影单测通过，导入协调器6项通过。原生owner-fixed7组和工作区owner-fixed7组均退出0，实际在原生File读取期间切换另一个已保存面板，原稿收到图片/注释且释放busy，另一稿及其磁盘字节不变。测试曾因getByRole默认排除隐藏原面板而超时，改为读取已证明归属的隐藏控件后重新验证，未改业务断言。[完整实测证据](evidence/v415-editor-checks.json)。最终安装已完成，详见发布检查；发布前最后修复的成功结果不依赖早期候选安装器。

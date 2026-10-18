# ResearchClip QA 盘点与复验报告

执行日期：2026-10-15；整理与发布准备续验：2026-10-16，Asia/Hong_Kong；规范：[TEST_PLAN](TEST_PLAN.md)；应用版本：0.1.0。

**结论：本轮规范建立、源码盘点和已知测试设施修正完成；不建议将此构建认定为正式发布验收通过。** Windows release EXE 和 NSIS 已构建，安装/默认卸载验收 PASS；素材库原生套件仍 FAIL（关闭阶段窗口句柄错误），托盘此前超时本次 PASS 但尚未稳定归因，硬件及人工矩阵未完成。未发现本轮已证实的 P0 数据丢失；这不是“没有任何数据风险”的保证。

本报告严格区分：A＝本会话较早的发布审查；B＝用户要求建立 QA 规范后的本轮；C＝2026-10-16用户授权新建公开仓库及发布GitHub预发布版本。A 的证据保留，B 未重跑的项目写 NOT_RUN。B结束时没有远程发布；C另行准备CHANGELOG、发布说明与附件，不改业务、迁移或依赖，不把预发布当作正式质量验收通过。

## 1. 环境与可追溯性

| 项目 | 本次记录 |
| --- | --- |
| 工作区 | `D:\文档\ChatGPT\ResearchClip`，存在大量前序未提交工作；不是干净发布标签 |
| 系统 | Windows 11 家庭中文版 10.0.26200，x64；Windows 10/macOS 没有本轮执行环境 |
| 工具 | Node 22.22.2、npm 10.9.7、Rust MSVC 1.99；Tauri 2.12.3 / CLI 2.12.2；锁文件沿用 |
| 数据 | SQLite schema v2；AnnotationDocument、项目与偏好 schemaVersion 均为1，分别校验 |
| 原生测试 | debug EXE + WebView2/CDP；独立 settings/workspace/Tauri目录；多数集成使用明确标记的320×200 PNG |
| 屏幕证据（A） | 主屏 DISPLAY2 2560×1440、原点(0,0)、100%；副屏 DISPLAY1 2560×1600、原点(-2560,0)，仅枚举不能算副屏像素验收 |
| 证据保护 | 新建唯一 artifacts 目录；旧失败日志未删除；业务文件基线见 `artifacts/release-audit-20261015/qa-business-baseline.json` |

测试图像只用于插件、编辑器与持久化用例；真实截图另有 A 阶段原生屏幕色块证据。本轮没有以生成图替代真实屏幕。原生自动化串行，未操作用户阅读器或删除用户资料。

## 2. 实际修改及修复边界

本轮 B 修改文件：

| 文件 | 修改与验证 |
| --- | --- |
| [TEST_PLAN.md](TEST_PLAN.md) | 新增五态结果、优先级、七步模块流程、隔离规则、用例目录和发布门槛 |
| 本报告 | 新增实现盘点、复现、命令和人工清单 |
| [editor-native-smoke.mjs](../../scripts/editor-native-smoke.mjs) | 保存请求是 `test-export.png`，原断言却寻找默认 `ResearchClip-export-*`；现在严格读取用户指定的同一目标，保留尺寸、全像素、重开和原图字节断言。B-05 PASS |
| [workflow_input.rs](../../src-tauri/examples/workflow_input.rs) | 合并可折叠if，修复全部目标Clippy失败；路径/PID校验保持。B-03复验 PASS |
| [check-docs.mjs](../../scripts/check-docs.mjs) | 递归覆盖 docs/qa，并核对npm锁文件与Cargo版本一致性；没有降低capability或迁移检查 |

A 阶段已有修复在 B 中保留，不冒充本轮新增：编辑器未保存标签关闭需确认；删除卡片时阻止与脏笔记/进行中写入冲突；debug测试目录隔离；NSIS遇运行中应用拒绝安装/卸载而不强退；覆盖层测试选择器、原生对话框可见控件输入/读回、带HWND的真实剪贴板锁、安装测试PE资源释放及Tauri bundle标记校验。前两项已加入单测；B的177项回归覆盖它们。新增业务缺陷本轮只记录，等待单独修复指令。

## 3. 源码盘点及模块结果

“已实现”仅说明找到完整入口和业务分支。每一项测试只覆盖所写范围；不能据此将整模块或完整MVP标为PASS。命令编号见第4节，A证据见第5节；人工项见第8节。

| 模块 / 实现状态 | 实际文件与关键函数 / 入口 | 本轮 B 实测结果、风险与下一阶段 |
| --- | --- | --- |
| 全屏截图：已实现Windows路径 | [CaptureControls](../../src/features/capture/CaptureControls.tsx)；[capture.rs](../../src-tauri/src/commands/capture.rs) `list_displays/capture_fullscreen/run_fullscreen_capture`；[windows.rs](../../src-tauri/src/capture/windows.rs)；`ScreenCaptureService::try_begin` | B-02尺寸/门控单测 PASS；B真实全屏像素 NOT_RUN，A-01 PASS仅主屏。GDI不能强行取消，350ms隐藏等待不是普遍保证；下一步硬件验收 |
| 区域截图：已实现单屏流程 | [CaptureOverlay](../../src/features/capture/CaptureOverlay.tsx)；[region.rs](../../src-tauri/src/commands/region.rs) `capture_region/ready_region_overlay/commit_region_selection/cancel_region_selection`；[geometry.rs](../../src-tauri/src/capture/geometry.rs) | B-01/B-02坐标/裁切 PASS；B-07仅进入并Esc取消 PASS，成功裁切本轮 NOT_RUN；A-01真实冻帧裁切 PASS。高DPI/旋转/插拔仍需硬件 |
| 双全局快捷键：已实现 | [SettingsView](../../src/features/settings/SettingsView.tsx)；[settings.rs](../../src-tauri/src/services/settings.rs) `initialize/update/action_for/prepare_shutdown`；[workflow.rs](../../src-tauri/src/commands/workflow.rs) `shortcut_capture` | B-04/B-07实际OS注册、外部占用回滚、重启释放 PASS；B物理按键触发 NOT_RUN。A SendInput曾因非测试进程前台被安全拒绝；保留人工项 |
| 剪贴板/原始PNG：已实现 | [CapturePanel](../../src/features/capture/CapturePanel.tsx)；`copy_capture_image/save_capture_png` | B-04 PASS：独立Edge读取320×200真实image/png全部RGBA；占用报错/保留预览；系统另存为/取消。Paint/Word及真实截图粘贴本轮 NOT_RUN |
| 五类标注/画布：已实现 | [EditorView](../../src/features/editor/EditorView.tsx)、[EditorCanvas](../../src/features/editor/EditorCanvas.tsx)、[AnnotationNode](../../src/features/editor/AnnotationNode.tsx)；[model](../../src/features/editor/model.ts) `createAnnotation/resizeAnnotation/moveArrowEndpoint`；[geometry](../../src/features/editor/geometry.ts) | B-01模型/坐标 PASS；B-05五类型创建、保存重开和删除/撤销 PASS。缩放后拖拽/手柄完整像素证据为A-02，本轮 NOT_RUN；物理IME/字体矩阵待人工 |
| 历史/脏状态：已实现 | [store](../../src/features/editor/store.ts) `close/update/undo/redo`；[history](../../src/features/editor/history.ts) `recordEdit/boundHistory` | B-01 PASS含关闭保护、单手势、redo失效、预算边界；B-05真实Delete/Ctrl+Z PASS。100步、8MiB/会话；与旧PRD32MiB预算有差异，不能宣称达到旧全部容量目标 |
| 无损项目/PNG合成：已实现 | [serialization](../../src/features/editor/serialization.ts) `parseDocument/parseProject/packImage/unpackProject`；[export](../../src/features/editor/export.ts) `renderComposite`；[editor命令](../../src-tauri/src/commands/editor.rs) | B-01/B-02校验 PASS；B-05实际对话框、原图字节不变、320×200合成、跨应用全像素一致、重启恢复 PASS。透明/箭头/文字/层级专门渲染验证A-02；B未重复该完整矩阵 |
| SQLite/文件管理：已实现v2 | [Storage](../../src-tauri/src/storage/mod.rs)；[migrations](../../src-tauri/src/storage/migrations.rs) `migrate/apply_migration/validate_schema`；[files](../../src-tauri/src/storage/files.rs)；[library](../../src-tauri/src/storage/library.rs) `create_screenshot/delete_screenshot` | B-02真实临时DB/文件单测 PASS；B-07实际保存失败阻止退出 PASS；B完整专门重启套件 NOT_RUN，A-03 PASS。多节点崩溃/真实磁盘满恢复尚未验收 |
| 素材库：已实现，当前回归失败 | [LibraryView](../../src/features/library/LibraryView.tsx)、[LibraryCard](../../src/features/library/LibraryCard.tsx)；[store](../../src/features/library/store.ts) `refresh`；[library-actions](../../src/app/library-actions.ts) `deleteLibraryScreenshot`；`screenshot_page` | B-06十组功能断言 PASS，但套件整体 FAIL：退出两次出现0x80070578；QA-001/P2。49卡/24分页、收藏/改名/删除/损坏缩略图和重启一致性已实测；不能忽略日志后算通过。建议先修复 |
| 知识卡片/标签：当前范围已实现；早期PRD部分未实现 | [KnowledgeCardView](../../src/features/notes/KnowledgeCardView.tsx)；[notes/store](../../src/features/notes/store.ts) `flush/flushAll/beginDelete/finishDelete`；[knowledge.rs](../../src-tauri/src/storage/knowledge.rs) `save_knowledge_card`；[TagsView](../../src/features/tags/TagsView.tsx) | B-01/B-02保存/关联/重载/冲突 PASS；B-07真实SQLite失败保留草稿、重启后正文存在 PASS；完整知识卡片套件B NOT_RUN/A-04 PASS。标签改名/NFKC未实现，见契约表 |
| 搜索筛选：已实现标题/正文及标签AND筛选 | [QuickSearch](../../src/features/library/QuickSearch.tsx)、[use-search](../../src/features/library/use-search.ts)；[search.rs](../../src-tauri/src/storage/search.rs) `pattern/search_screenshot_page` | B-01/B-02中文、ASCII、特殊字符、空查询、组合筛选 PASS；B完整UI矩阵 NOT_RUN/A-04 PASS。LIKE不对标签名作关键词匹配；1000卡p95 NOT_RUN |
| Markdown：已实现原图模式 | [MarkdownExport](../../src/features/export/MarkdownExport.tsx)；[markdown命令](../../src-tauri/src/commands/markdown.rs)；[markdown仓储](../../src-tauri/src/storage/markdown.rs) `export_markdown/reserve_directory/publish_file` | B-01/B-02目录/相对引用/部分失败 PASS；B真实目录对话框全套 NOT_RUN/A-05 PASS。仅原图+主笔记，合成模式未实现；Obsidian/VS Code移动目录人工验收 NOT_RUN |
| 设置/托盘/退出：已实现，存在间歇失败记录 | [desktop.rs](../../src-tauri/src/services/desktop.rs) `install_tray/show_main/request_exit/finish_exit`；[use-desktop-experience](../../src/features/desktop/use-desktop-experience.ts)；[desktop/store](../../src/features/desktop/store.ts) | B-07十组原生断言 PASS；A托盘恢复超时 FAIL，QA-002/P2待归因，未改业务不能称已修复。B系统色彩为CDP媒体模拟，真实Windows切换及Explorer重启 NOT_RUN |
| 权限/资源：已实现限制，有测试边界 | [main capability](../../src-tauri/capabilities/main.json)、[overlay capability](../../src-tauri/capabilities/capture-overlay.json)、[config](../../src-tauri/tauri.conf.json)、[lib](../../src-tauri/src/lib.rs) | 静态审查与B文档/命令权限一致性检查 PASS不等于渗透测试；A-01 overlay越权拒绝 PASS。生产无调试fixture；全面生产安全测试 NOT_RUN |
| Windows安装：已实现NSIS候选 | [installer hook](../../src-tauri/windows/installer-hooks.nsh)、[installer-smoke](../../scripts/installer-smoke.ps1) | B-08构建、B-09安装/默认卸载 PASS；未签名；安装后的生产程序端到端、升级、Windows10、交互向导 NOT_RUN/BLOCKED。暂不进入正式发布 |

### 审阅重点结论

- TypeScript严格类型与React Hooks lint有可执行检查；Zustand采用按图/卡片状态、请求generation、防抖串行写入、revision冲突处理。库列表只保留当前页，编辑器8图/128MiB、卡片20会话，限制历史/画笔点数。
- 异步listen返回后卸载会调用unlisten；ResizeObserver、Blob URL、ImageBitmap、Konva Stage和Windows GDI有清理代码/部分测试。QA-001说明“存在清理代码”不等于所有退出时序已通过。
- main只授予枚举业务命令，overlay仅5个会话命令；无通用fs/SQL/前端插件注册权限。CSP脚本本地；来源链接只允许已验证http(s)。不能把静态白名单核对说成完整安全审计通过。
- SQL参数化、固定排序枚举；图片引用绑定ID与受管目录，原图不变；同目录临时文件、sync、禁止覆盖提交；删除有事务意图与失败重试，不递归删除用户目录。
- v1→v2升级前校验并VACUUM INTO备份、事务迁移；未来/坏库拒绝写入。未通过删除数据库解决问题；完整崩溃/恢复矩阵仍待执行。
- npm七个直接运行依赖均在源码使用，未找到可直接移除的无用依赖。Cargo重复版本为上游依赖图，不盲目强制统一。没有因审查新增框架或改锁文件。

### 文档与实现差异（不自动扩展功能）

| 项目 | 当前事实 / 对应状态 | 处置 |
| --- | --- | --- |
| PRD顶部“仍为M1”及架构早期段落 | 与当前实现不符，文档一致性P2；架构后部已记录更新契约 | 以本报告源码映射为盘点；本轮不重写需求基线 |
| 回收站/恢复 | NOT_IMPLEMENTED；当前用户确认后永久删记录及受管文件 | 最新删除请求与ARCH第10节已明确永久删除；不把它作为回收站通过，不擅自增功能 |
| 标签改名/NFKC | NOT_IMPLEMENTED；现有创建/关联/删除、trim+ASCII规范化 | 早期PRD目标，待用户决定保留或后续实施 |
| 关键词匹配标签名 | NOT_IMPLEMENTED；实际关键词匹配标题/笔记，另有标签AND筛选 | 符合后续搜索请求；早期PRD未同步 |
| 快捷键鼠标所在屏/固定屏策略 | NOT_IMPLEMENTED；热键用主屏，按钮可指定枚举屏 | 本轮不添加设置；人工说明目标显示器 |
| Markdown合成图/notes+images结构 | NOT_IMPLEMENTED；现有notes.md+assets和原图，符合后续导出请求 | 旧目标不能当成已完成；导出标注用编辑器PNG |
| 32MiB/2000图元/100000点等旧容量 | 实际历史8MiB、500图元、5000点/笔、总50000点 | 部分实现且规格差异；资源保护存在不等于旧容量验收 |
| macOS原生截图 | NOT_IMPLEMENTED，非Windows分支返回UNSUPPORTED | Windows首发后单独适配，不能做跨平台通过承诺 |

## 4. 本轮 B 实际执行的命令与结果

日志根目录：`artifacts/release-audit-20261015`（仅本地）。独立命令未运行的，不因为组合脚本包含名字而重复计数；测试组数不等于单元测试数。

| ID | 实际命令 | 状态 / 退出码 / 结果与证据 |
| --- | --- | --- |
| B-01a | `npm run check`（受限环境，日志qa-frontend.log） | FAIL / 1；类型与lint通过，Vitest14文件初始化ENOENT（沙箱Temp变换缓存）；10文件79项执行通过；build被前序失败阻止，NOT_RUN |
| B-01b | `npm run check`（正常授权进程，qa-frontend-retry.log） | PASS / 0；相同全部24文件177测试；TypeScript、ESLint、Vite生产构建通过，无跳过失败用例 |
| B-02 | `npm run test:rust` | PASS / 0；87通过、0失败、0忽略；main/doc tests各0；qa-rust-test.log |
| B-03a | `npm run format:rust`；`npm run check:rust` | 前者PASS/0；后者FAIL/101，`workflow_input.rs`可折叠if，-D warnings拒绝；qa-rust-clippy.log |
| B-03b | 同上，修正测试辅助程序后原命令复验 | 两者PASS/0；qa-rust-fmt-retry.log、qa-rust-clippy-retry.log |
| B-04 | `node scripts/workflow-smoke.mjs qa-20261015-workflow --fixture --native-save-controls` | PASS / 0；9组；[报告](evidence/b-workflow.json)，真实插件/文件对话框/注册，非真实截图 |
| B-05 | `node scripts/editor-native-smoke.mjs qa-20261015-editor` | PASS / 0；6组；[报告](evidence/b-editor.json)，原图字节/五类对象/PNG/剪贴板全像素/重启 |
| B-06 | `node scripts/library-native-smoke.mjs qa-20261015-library` | FAIL / 1；10组功能断言通过后stderr断言失败；[报告](evidence/b-library-failed.json) |
| B-07 | `node scripts/desktop-native-smoke.mjs qa-20261015-desktop` | PASS / 0；10组；[报告](evidence/b-desktop.json)，实际托盘回调/菜单、冲突、单实例、退出保护；区域返回CANCELLED不算截图成功 |
| B-08 | `$env:CARGO_TARGET_DIR = Join-Path (Get-Location) 'src-tauri/target/release-audit'`；`npm run desktop:bundle` | PASS / 0；前端构建、Rust优化构建38.72s、makensis；[日志](evidence/b-release-build.txt) |
| B-09 | `pwsh -NoProfile -File scripts/installer-smoke.ps1 -RunId qa-20261015-final` | PASS / 0；4组；[报告](evidence/b-installer.json)；正常安装/卸载0，运行中安装/卸载分别2，受控进程存活 |
| B-10 | `npm run check:docs`、脚本语法/业务文件哈希复核 | 初次FAIL/1为本报告误写证据目录名，修正后PASS/0；当时24文档142链接；Node/PowerShell语法PASS/0；123份src/迁移文件hash不变 |

重试时不修改测试断言和业务源码，不清理真实Temp/AppData。B-01a保留为FAIL及环境原因，B-01b的PASS不抹除它。B-03a为测试设施缺陷，已按Clippy提示修复，无allow属性。

## 5. A 阶段证据（本轮B未重复的明确标记NOT_RUN）

| ID | A实际命令 / 证据 | A结果；B状态 |
| --- | --- | --- |
| A-01 | `npm run test:smoke:desktop -- --run-id release-review-20261015-3`；[报告](evidence/a-screen.json) | PASS/0；真实主屏九点匹配、冻帧裁切、20轮区域（10提交/10Esc）、资源/重启；B NOT_RUN |
| A-02 | `npm run test:smoke:editor -- release-review-20261015`；[报告](evidence/a-editor-renderer.json) | PASS/0，16组；透明、文字、箭头、图层、缩放拖拽/手柄、关闭保护；浏览器夹具；B NOT_RUN |
| A-03 | `node scripts/storage-native-smoke.mjs release-final-20261015-storage`；[报告](evidence/a-storage.json) | PASS/0，8组；真实SQLite/文件/向量重开与两次重启；B专门套件NOT_RUN |
| A-04 | `node scripts/knowledge-native-smoke.mjs release-final-20261015-knowledge`；[报告](evidence/a-knowledge.json) | PASS/0，9组；知识卡片/标签/搜索；B专门套件NOT_RUN |
| A-05 | `node scripts/markdown-native-smoke.mjs release-final-20261015-markdown`；[报告](evidence/a-markdown.json) | PASS/0，7组；真实导出目录对话框/相对链接；B专门套件NOT_RUN |
| A-06 | `npm audit --json`，再`npm audit --json --registry=https://registry.npmjs.org` | 镜像audit接口404 FAIL/1；官方源PASS/0、当时npm公告0项；B NOT_RUN，不代表Rust安全扫描 |
| A-07 | `node scripts/with-rust.mjs cargo tree --manifest-path src-tauri/Cargo.toml --locked --duplicates` | PASS/0；上游png/windows-sys等重复，未改依赖；B NOT_RUN |

A真实截图前两次被阅读器窗口遮挡，九点像素断言FAIL；用户明确回复最小化/关闭后第3次PASS。4次全屏延迟2010、1756、1749、1754ms，仅2560×1440样本，不是4K预热后50次p95。A的原生批量结果还保留了库/托盘/编辑器三项FAIL，见 [native-results](evidence/a-native-exit-codes.json)。

## 6. 问题、复现与处置

### QA-001：素材库退出出现无效窗口句柄，P2，未修复

- 实现路径：LibraryView/缩略图异步IPC，`DesktopService::finish_exit`关闭WebView；根因仍待确认，不能仅凭日志归责某一函数。
- 复现：从根目录执行 `node scripts/library-native-smoke.mjs <新的ID>`。脚本创建49条隔离记录，翻页/改名/收藏/缺图恢复/删除，关闭再启动，最终关闭。
- 预期：十组数据断言通过且stderr无异常。实际：数据断言全部通过，但两次 `PostMessage failed ... 0x80070578 - 无效的窗口句柄`，最终 `2 !== 0`，退出码1。
- A与B均复现同样2条；[B报告](evidence/b-library-failed.json)。没有发现本次丢行、原图改变或崩溃；不能据此忽略生命周期错误。
- 状态FAIL；建议下一阶段优先在隔离环境记录IPC完成/窗口关闭时序并修复，再重跑本套件和桌面退出套件。本轮遵守不改新发现业务Bug的约束。

### QA-002：托盘回调恢复偶发超时，P2待归因

- A复现命令：`node scripts/desktop-native-smoke.mjs release-final-20261015-desktop`，启用关闭到托盘→关闭主窗→向本测试PID的原生托盘窗口发送左键回调，等待主窗可见，超时20秒。原始[FAIL报告](evidence/a-desktop-failed.json)保留。
- B-07同一业务代码复验PASS，真实菜单、三轮最小化/恢复及退出保护也通过；没有证据可以称已修复。
- 安装的tray-icon0.25.1在回调中调用GetCursorPos与Shell_NotifyIconGetRect，失败时提前返回；这只是排查方向，未证明本次超时原因。
- 保留间歇性风险；用户物理点击可见/隐藏托盘及Explorer重启后恢复，仍NOT_RUN。若稳定无法恢复，按P1重新分级。不要以直接IPC恢复窗口替换失败步骤制造PASS。

### QA-003～006：测试设施问题，已修正并复验

| ID / 优先级 | 原FAIL与原因 | 当前结果 |
| --- | --- | --- |
| QA-003 / P2 | 原生编辑器保存指定test-export.png后仍寻找默认文件名 | B-05 PASS，严格验证实际指定文件，没有放宽像素断言 |
| QA-004 / P2 | 对话框辅助程序选择隐藏字段/SetWindowText未触发真实变更；工作流停留保存对话框 | A已改为可见字段、WM_CHAR、读回一致再确认；B-04/B-05 PASS |
| QA-005 / P2 | 剪贴板占用夹具未真正独占，导致错误路径未触发 | A改为自有HWND持有并独立探针验证BUSY；B-04真实错误路径PASS |
| QA-006 / P3 | 验收Rust辅助程序的嵌套if使-D warnings失败 | B-03b PASS，不关闭警告 |

A安装测试也曾因Tauri独有UNK→NSS bundle标记导致“整个文件hash不同”的误判，以及同进程PE元数据读取占用EXE导致正常卸载被安全拒绝。已使用短生命周期元数据检查进程，且只规范化经安装源码确认的唯一标记，再比对整个文件hash；B-09 PASS。不是跳过payload验证。旧清理记录中319曾包含易变WebView缓存，不能引用为319份科研资料不变；本轮准确数字为8份核心文件。

### QA-007：需求/现有文档契约冲突，P2，待确认

复现：对照PRD F05–F08、ARCH早期草案与ARCH第9–13节及上述源码。差异已逐项列在第3节。本轮以实际能力盘点，不把未实现功能标PASS，不擅自删减旧发布目标。

## 7. release 产物与安装验收

这两个是**0.1.0验收候选**。C阶段用户授权以`v0.1.0-rc.1`公开预发布，不代表正式版质量门槛通过：

| 产物 | 路径 / 尺寸 / SHA-256 |
| --- | --- |
| 可直接启动程序 | `src-tauri/target/release-audit/release/researchclip.exe`；15,311,360字节；`0560C796AE12FE0877CB63AC7EE75A2732359DFE691A0BD81C0011D844D9DBDF` |
| Windows NSIS安装包 | `src-tauri/target/release-audit/release/bundle/nsis/ResearchClip_0.1.0_x64-setup.exe`；3,704,395字节；`4F0032C89E21F40FBCAB742FB77AE6A0C07F5BBA211B90E95093E56FD6BE0C1E` |

配置为当前用户安装、中英文、ResearchClip图标/产品/发布者和版本0.1.0；签名状态NotSigned。需要Windows x64和WebView2；默认安装器在缺少WebView2时使用下载引导，不能承诺完全离线安装。运行前保存并退出旧实例，否则单实例会恢复旧窗口。

B-09实际安装到专有中文/空格路径，核对版本/图标/注册及安装payload；用只驻留等待的受控进程夹具验证“运行中不强退”保护，安装/卸载均返回2且进程存活；随后真实默认卸载成功，移除程序/安装注册，保留无关哨兵文件。测试安装已卸载，`installationStillPresent=false`。

真实AppData只读前后hash：8份配置/数据库/科研图片等核心文件不变。仅`Local/EBWebView`运行缓存排除不可变断言，未把缓存当核心资料；未启动正式安装版去写用户数据。安装后的生产应用全流程因此NOT_RUN，不能由debug原生测试代替。

NSIS默认卸载的“删除应用数据”未勾选；本次证明的是默认行为保留资料。用户明确勾选该项会删除资料，不能声称任何卸载选项都保留。本轮没有在真实用户资料上测试该危险选项。签名、干净系统、交互安装/升级与Windows10仍未验收。

## 8. Windows 人工验收清单（全部未标通过）

每项完成后记录系统、分辨率/缩放、构建hash、实际步骤与结果。当前状态只反映本轮执行，不用自动化PASS替代人工签收。

| ID | 操作与预期 | 当前状态 |
| --- | --- | --- |
| M01 全屏 | 指定每个显示器，捕获论文/普通桌面；核对边角/尺寸、不包含主窗；失败后重试可用 | NOT_RUN |
| M02 区域 | 四向/跨边界/0面积拖动、Esc、失焦、连续20次；冻结画面、宽高和落点一致，无残留层 | NOT_RUN |
| M03 高DPI/多屏 | 100/125/150/200%、左右/上负原点、混合缩放/旋转/拔插；裁切误差≤1像素或安全取消 | BLOCKED：本轮没有完成对应物理配置矩阵 |
| M04 快捷键 | 在其他应用前台及关闭到托盘时，真实按下两种可配置快捷键；冲突回滚、长按不重复 | NOT_RUN |
| M05 剪贴板 | 原图和透明合成图Ctrl+V到Paint/Word，核对图像而非路径、色彩/透明度；被占用时提示 | NOT_RUN |
| M06 标注 | 五类工具、文字IME/多行、选中/移动/手柄/Delete/Esc；切工具不误建；缩放平移后原位置不变 | NOT_RUN |
| M07 历史 | 创建/改属性/移动/缩放/删→Ctrl+Z/Y/Shift+Z；新编辑清redo；脏标签关闭确认保留/放弃正确 | NOT_RUN |
| M08 PNG/项目 | 中文空格路径保存/取消/同名拒绝；未知项目/缺图提示且保留草稿；外部打开PNG无手柄、原分辨率 | NOT_RUN |
| M09 SQLite | 在新测试副本保存截图/标注/笔记后退出重启；原图hash和关联一致；迁移备份/失败恢复不重置 | NOT_RUN |
| M10 素材库 | 翻页、升降序、收藏/取消、重命名、确认/取消删除、坏图占位；快速退出无异常 | NOT_RUN；QA-001自动化FAIL未解决 |
| M11 笔记/标签 | 快速切换卡片保留未保存输入，失败可重试；移除/删除标签不删图；打开http(s)来源 | NOT_RUN |
| M12 搜索 | 中文IME、英文、%、_、引号/反斜杠、多标签AND+收藏、清空和无结果；只读当前页 | NOT_RUN |
| M13 Markdown | 单/多卡导出，复制整个目录到另一位置，用Obsidian/VS Code查看相对图片；缺图部分失败可理解 | NOT_RUN |
| M14 设置/托盘 | 实际系统主题切换、默认标注/目录重启恢复；托盘所有菜单、关闭/最小化/再打开、Explorer重启 | NOT_RUN；QA-002需重点关注 |
| M15 发布程序完整流程 | 用第7节release程序在独立Windows测试账号串起截图→五类标注→保存重启→笔记/标签/搜索→PNG/Markdown | NOT_RUN |
| M16 安装/升级/卸载 | 干净Windows10/11，交互向导、缺WebView2/离线提示、升级保留旧资料、默认卸载保留数据 | BLOCKED：缺干净系统/旧发布升级矩阵；当前机静默安装另有PASS |
| M17 性能/长稳 | 4K/1000卡、预热5次后50次原始样本和nearest-rank p95、50次截图资源趋势；达ROADMAP目标 | NOT_RUN |
| M18 Rust公告/生产安全 | 配置cargo-audit或等效公告扫描、生产包调试端口/联网及权限边界实测 | NOT_RUN；未安装scanner不能宣称Rust零漏洞 |

## 9. 是否进入下一阶段

建议进入“QA-001独立修复、QA-002稳定性归因及人工矩阵”，不进入正式稳定版发布。C阶段仅按用户授权发布含已知问题说明的预发布；业务修复需单独指令，不自动补回收站、macOS、云端、OCR或AI功能。

## 10. 最终复核与C阶段发布准备

- `node --check scripts/editor-native-smoke.mjs`、`node --check scripts/check-docs.mjs`、PowerShell Parser对安装测试脚本的语法检查均PASS/0；`git diff --check` PASS/0（只含既有CRLF转换提示）。
- 123份前端/Rust业务源码及迁移文件hash核对PASS，B基线以来0变更；测试辅助程序、报告和发布文档另行列入修改范围。
- 公开源码只包含Git未忽略的工程文件。`artifacts/`、`.tools/`、`target/`、用户数据库、环境配置与图片测试工作区均排除；对拟发布文本做常见令牌/私钥模式扫描，没有发现匹配，不能据此保证所有形式秘密均被检测。
- `evidence/`收录12份原始JSON报告的去本地路径副本和1份构建日志；结论、断言和FAIL保留。原始文件及导出前hash见`evidence/MANIFEST.json`，原始证据仍留本机，不上传截图/数据库。副本使GitHub上的报告链接可读，不以生成报告冒充运行证据。
- 2026-10-16用户明确授权公开仓库`wpy-sketch/ResearchClip`和GitHub Release；发布说明将其标为预发布，并保留QA-001/002及人工缺口。发布完成与下载校验以实际远程结果为准，失败不得写成功。
- C阶段`npm run check:docs` PASS/0：26份Markdown文档、143个本地链接，权限与schema检查一致；`npm run lint`及两份改动Node脚本语法复核PASS/0。报告中的错误证据路径已修正，未删除失败记录。
- C阶段使用既有Git Credential Manager登录创建公开仓库PASS：`https://github.com/wpy-sketch/ResearchClip`；创建前确认不存在同名仓库，未覆盖旧内容。登录凭据只在进程内存使用，不写入代码或日志。
- 首次暂存完整源码后`git diff --cached --check` FAIL/2，发现既有对话框脚本和新构建日志副本各一处尾部空行；仅规范结尾换行，不改脚本逻辑或原始构建日志，复验后再提交。此前未暂存检查不覆盖新文件，因此保留这次失败记录。

## 11. C阶段远程发布实际结果（2026-10-16）

| 操作 / 实际命令 | 结果 |
| --- | --- |
| 既有凭据账号核验、GitHub REST `POST /user/repos` | PASS/0；新建公开仓库 [wpy-sketch/ResearchClip](https://github.com/wpy-sketch/ResearchClip)，创建前不存在同名仓库；无LICENSE自动添加，无远程覆盖 |
| `git commit -m 'Prepare ResearchClip Windows preview and systematic QA baseline'` | PASS/0；`fcba7c1275e7d60de8fa38c45071392dca43e74a`，含已有工程、锁文件和QA基线；首次暂存检查空行失败已先修正再提交 |
| `git tag -a v0.1.0-rc.1 -m 'ResearchClip 0.1.0-rc.1 Windows preview; known QA limitations documented'` | PASS/0，新标签；未替换已有标签 |
| `git push --atomic origin HEAD:refs/heads/main refs/tags/v0.1.0-rc.1:refs/tags/v0.1.0-rc.1` | PASS/0，main和标签均新增；无force；保留历史 |
| GitHub REST创建draft/prerelease→上传三个附件→核对size/digest→公开 | PASS/0；[v0.1.0-rc.1](https://github.com/wpy-sketch/ResearchClip/releases/tag/v0.1.0-rc.1)，`draft=false`、`prerelease=true`、不设置为稳定latest；源码标签仍指向上述提交 |
| `git ls-remote origin refs/heads/main refs/tags/v0.1.0-rc.1 'refs/tags/v0.1.0-rc.1^{}'` | PASS/0；当时main与解引用标签均为fcba7c1；后续仅补充本节发布验证文档，不移动发布标签 |
| 无凭据GET公开仓库/Release、逐个HTTP下载三个附件并Get-FileHash | PASS/0；默认分支main；3份附件字节数和SHA-256均与上传前一致；[验证记录](evidence/c-github-public.json) |
| 拟发布Markdown链接的目标是否Git收录 | PASS/0；当时211个受版本管理文件，0个Markdown链接依赖仅本地ignored文件；未上传屏幕截图/数据库/工具凭据 |

发布使用本机Git Credential Manager的既有登录，通过本地临时发布脚本调用官方REST API；脚本与凭据均未提交。GitHub CLI未安装且连接器最初未返回可访问仓库，未将这些能力缺失伪装成CLI执行成功。上传阶段返回过临时untagged下载地址，公开后重新GET得到规范`v0.1.0-rc.1`地址并完成实际下载核验，最终链接以验证记录为准。

发布说明保留本报告FAIL、未实现项和人工缺口；**公开预发布成功不改变QA-001的FAIL，也不等于正式稳定版验收通过。**

## 2026-10-17 桌面贴图增量验收

范围仅用户本轮桌面贴图请求，非整套 MVP 再认证。已实现截图完成页/编辑器按钮、真正独立无边框置顶窗口、多贴图、拖动/缩放/透明度、复制/另存/关闭、恢复位置和退出清理；涉及文件、关键函数/模块、全部修改文件及原生证据详见[贴图报告](../PINS-VALIDATION.md)。

PASS（实际命令退出 0）：`npm run check`（220 测试）、`npm run format:rust`、`npm run check:rust`、`npm run test:rust`（102 测试）、Windows debug 构建与启动、`node scripts/pins-native-smoke.mjs final-20261017`（13 组）、`node scripts/editor-native-smoke.mjs pins-regression-20261017`（6 组）、`npm run desktop:build`（生产 EXE）、`npm run check:docs`。托盘隐藏通过实际 HWND 可见性确认；剪贴板失败通过真正锁定 Windows 剪贴板复现，释放后重试成功；不是 Mock 结果。

初期 FAIL 与修正记录保留在贴图报告及本地 artifacts；最终本模块无尚未修复的自动检查 FAIL。没有已确认新增 P0/P1。历史 QA-001 / QA-002 不在本轮关闭，不能据成功的贴图专项验收宣称全项目已可正式发布。

NOT_RUN：真实屏幕内容的全屏/区域到贴图、高/混合 DPI 和多屏硬件矩阵、画图等人工粘贴、长期堆/显存、超时与原生窗口创建/销毁失败注入、Windows 10/macOS、最终生产 EXE 交互和 NSIS 安装卸载。人工步骤详见贴图报告。未触碰真实用户工作区或删除资料；仅隔离测试目录有写入。

建议：当前请求实现及本轮自动/原生专项检查完成，按用户要求停止；用户可依据人工矩阵验收。完整发布判定仍遵守 TEST_PLAN，不自动推进其他功能或远程发布。


## 2026-10-17 v0.1.3 OCR 增量验收

新增离线 OCR 的实际入口、实现、命令、原生证据、首次失败与修复、准确性 FAIL、语言包 BLOCKED 和人工 NOT_RUN 矩阵见 [v0.1.3 验收](RELEASE-0.1.3.md)。本次核心资料及迁移保持不变，历史 QA-001/QA-002 未据此关闭。用户要求本次新 Release 不保留预发布标识，不将该发布属性当作未执行项目通过。


## v0.1.4 高级标注增量验收

马赛克、局部模糊、编号及前后端 JSON 扩展的实现文件、234 项前端/107 项 Rust 检查、真实像素和 Windows 原生保存重开/持久化证据、原有工具回归及非预发布公开回执见 [v0.1.4 验收](RELEASE-0.1.4.md)。硬件/长稳/大图性能/生产完整人工流程仍 NOT_RUN，QA-001/QA-002 不据此关闭。


## 2026-10-17 v0.1.5 模板与大纲

新增模板、独立模板持久化与 H1–H6 大纲，本轮实现、实际检查、首轮失败修复、原生与人工清单见 [0.1.5 验收](RELEASE-0.1.5.md)。保留历史问题状态，不以本次新增功能检查覆盖旧硬件/长稳矩阵。

发布续验：v0.1.5已公开为非预发布Latest，源码339a3eb、Windows EXE及2个校验说明附件均已上传。匿名REST速率403后改走公开下载，三份大小和SHA256均匹配；[发布回执](evidence/v015-publication.json)。完整修改文件、首轮失败修复、已知边界及NOT_RUN人工矩阵保留于[0.1.5报告](RELEASE-0.1.5.md)。


## v0.1.8 增量验收（2026-10-17）

导入、独立双图对比、列表/网格及完整指南详见[本轮报告](RELEASE-0.1.8.md)。实际261项前端/116项Rust、9组导入/8组对比、11组分栏/8组图文/10组模板/6组编辑原生回归、8组浏览器界面检查及Windows生产构建均PASS。历史失败及修复有独立证据；完整人工、高DPI/多屏、长稳、生产用户资料及安装卸载本轮NOT_RUN；QA-001/002继续跟踪。本次无新迁移或用户资料重置。


## v0.2.0 增量验收（2026-10-17）

[科研项目与批量报告](RELEASE-0.2.0.md)：SQLite v3备份迁移、项目关系、文档索引和批量事务；280项前端/123项Rust、57组隔离Windows原生及8组浏览器界面检查和Windows生产构建实际PASS。项目筛选P1、改名规范路径与短窗P2已修复并复验，初始失败均留证据。真实AppData未重置，完整生产/硬件/长稳/安装卸载NOT_RUN；历史QA-001/002继续跟踪。最终源码与发布回执另记。


## v0.2.1 增量验收（2026-10-18）

[项目分类/离线数学公式报告](RELEASE-0.2.1.md)：侧栏、公式安全/实时预览/源码保存重开、实际Windows打开方式选择器、带图指南；43文件299项前端、125项Rust、67组隔离原生和8组浏览器布局以及Windows生产构建最终PASS。初期失败与拖放不稳定均保留；第三方Markdown应用逐个打开、完整硬件/真实生产资料/安装矩阵NOT_RUN，历史QA-001/002继续跟踪。schema v3及既有001/002/003迁移不变，无资料重置。源码与发布结果以随后回执单独记录。


## 2026-10-18 v1.0.0 界面与图文指南

[完整报告](RELEASE-1.0.0.md)：43文件300项前端、125项Rust、11套88组隔离原生专项、8组浏览器布局、主屏实际3帧及最终NSIS隔离安装/卸载4组PASS，7份既有资料摘要保留。素材库专项仍FAIL：10组资料断言完成，但退出两条0x80070578触发最终零stderr断言；QA-001/P2保留，QA-002本轮完整10组托盘复验通过但不自动关闭历史间歇问题。初期夹具schema/定位问题修正后复验、系统打开方式首次超时及重测均留证据。无新依赖/权限/schema变更，不把正式Release属性视为全套稳定性认证。完整生产UI/多DPI多屏/Windows10/macOS/第三方应用/长稳矩阵NOT_RUN。远程发布以回执为准。

## 2026-10-18 v1.0.1 浏览器滚动长截图

[本轮完整报告](RELEASE-1.0.1.md)：44文件306项前端/132项Rust及Windows生产构建PASS；实际Edge手动3屏、自动10屏抽样像素无差异，但十屏所属首轮整套脚本因后续旧按钮定位仍FAIL；最终3屏完整滚动专项9组PASS。区域21、指南5、分栏11、编辑6、数学8组原生回归及8组浏览器布局PASS。最终两新目录NSIS默认卸载4组各PASS，7份既有资料摘要保留；先前多轮间歇卸载FAIL列QA-008/P2 OPEN，根因未确认，不以时序前置检查宣称修复。QA-001/002继续保留。Chrome/实屏125%与150%/副屏长截图/长稳/完整生产UI NOT_RUN。schema v3不变、无新依赖、未重置用户资料。远程发布以回执为准。

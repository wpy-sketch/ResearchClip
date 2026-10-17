# v0.2.0 科研项目与批量素材验收

日期：2026-10-17，Asia/Hong_Kong。环境：Windows 11 NT10.0.26200 x64、WebView2；Node22.22.2/npm10.9.7、Rust MSVC1.99、Tauri2.12.3/CLI2.12.2。按[TEST_PLAN](TEST_PLAN.md)区分PASS/FAIL/BLOCKED/NOT_IMPLEMENTED/NOT_RUN。只修改本轮项目/批量/界面指南及必要契约，不重建应用；无新增依赖。两个锁文件只同步应用版本0.2.0。

## 数据库变化与数据安全

SQLite **schema v2→v3**，新增不可变迁移003_projects.sql：

| 表 | 作用与约束 |
| --- | --- |
| projects | 项目ID、名称/规范化唯一名称、创建/更新时间；名称1–80字符，最多500个项目 |
| screenshot_projects | screenshot_id主键与截图CASCADE外键、project_id RESTRICT外键；每图最多一项目 |
| markdown_documents | 稳定ID、唯一文件路径、显示名、可空项目、登记时间；项目RESTRICT，不保存Markdown正文 |

新增项目/文档查询索引。001/002原有迁移不改；打开旧库先验证、生成并校验一致备份，再事务升级。未知未来版本拒绝写入；失败回滚，不删除资料。旧图片ID、路径、标注JSON、笔记、来源和标签保留，旧资料默认未分类。不改Annotation schemaVersion1。旧版不能写v3，请保留备份，不直接降级重存。

项目只做逻辑分组，移动不搬磁盘文件；标签跨项目共享。第一版仅允许删除空项目，UI和事务均检查图片/文档数量，不提供“删除项目内数据”的隐式动作。

批量移动/添加标签/收藏/删除使用BEGIN IMMEDIATE，整批校验id/revision；任一失效即整批回滚。标签追加合并而非替换，每卡100/全局1000上限。删除须明确确认数量，事务同时写入受管图片清理队列，提交后再清理，失败/超过每次200任务预算保留供重试和重启。不会删除外部源图、Markdown、assets或项目。已打开标注草稿保留并解除旧库绑定；笔记先flush，失败阻止批量操作。批量期间阻止正常退出。

Markdown继续用原DocumentFiles/文件保存管线和标准.md。系统选择授权token登记SQLite路径，WebView不传任意路径；应用内重命名更新同一索引ID/项目，另存为建立独立索引。索引失败单独提示、保留已保存文件并可重试。项目按ID重开仍经过原脏草稿确认；不扫描磁盘或复制正文。外部移动/改名须重新打开登记，旧缺失引用保留；没有自动跟踪外部文件身份。

## 功能入口与实现状态

| 功能 | 关键文件/入口 | 状态 |
| --- | --- | --- |
| 项目新建/重命名/空项目删除/计数 | storage/projects、commands/projects、ProjectBar/save_project/delete_empty_project | 已实现；PASS |
| 全部/未分类/项目浏览 | search::LibraryFilter、LibraryStore/ProjectBar，统一SQL谓词 | 已实现；PASS |
| 文档登记/关联/重开/改名 | DocumentFiles::index_paths/mark_indexed、document/api、sync_project_document/open_project_document、DocumentProject | 已实现；PASS |
| Ctrl/Shift/筛选全选 | projects/selection、BulkActions、LibraryCard；select_library_screenshots | 已实现；PASS；Shift本页范围，Ctrl/勾选可跨页 |
| 批量标签/移动/收藏 | projects/batch、batch_library_screenshots，固定动作枚举 | 已实现；PASS |
| 数量确认删除/取消/恢复 | BulkActions快照、BatchAction::Delete与pending_file_deletions | 已实现；PASS |
| 批量导出 | 复用原MarkdownExport/系统目录对话框/原图assets | 已实现；PASS，沿用每次100张资源预算 |
| 布局/指南 | styles、GuideView和USER_GUIDE.md | 已实现；PASS，14节任务分组/章节搜索 |
| 非空项目连同内容删除、外部文件自动跟踪 | 第一版不提供，默认保留资料 | NOT_IMPLEMENTED，明确范围边界 |

所有新增9个命令仅main授权；overlay/pin不增权限，无通用fs或SQL。实际全选返回ID/revision，最多5000，超限整次拒绝不截断；分页继续24项/当前页缩略图。选择快照剥离图片元数据，避免向严格IPC发送多余字段。改变筛选清空选择；失败保留选择和草稿。

## 实际执行的命令与结果

以下最终执行退出码均0，无跳过断言。Mock仅证明相应边界逻辑，不充当Windows或真实数据库验收。

- PASS：`npm run check`，TypeScript、ESLint、**42文件/280项Vitest**、前端生产构建。包括范围/跨页/超限选择、参数转发、严格DTO、取消不调用删除、事务失败选择保留、笔记保存失败、文档索引重试、过期查询、原文档脏草稿保护、批量退出保护，以及已打开编辑器的标注/历史保存点不被重置。
- PASS：`npm run format:rust`、`npm run test:rust`（**123项**，0失败/忽略）、`npm run check:rust`（all-targets、-D warnings）。新增7个真实SQLite测试：项目与笔记/原图重启、批量标签与冲突回滚、删除确认与中途失败回滚/清理重启、文档改名ID/归属、v2备份升级、失败升级保留原库、5001条全选拒绝无截断。
- PASS：`node scripts/with-rust.mjs tauri build --debug --no-bundle`；`node scripts/with-rust.mjs cargo build --manifest-path src-tauri/Cargo.toml --locked --example workflow_input --example markdown_dialog`。原生辅助器只操作匹配PID/标题的测试窗口和artifacts路径，焦点不符拒绝发送输入，临时置顶/线程输入连接有恢复。
- PASS：`node scripts/projects-native-smoke.mjs v020-release`，[11组完整原生项目证据](evidence/v020-projects-native.json)。实际预建32卡v2库→v3及可读备份；UI项目增改/空项目确认、Ctrl/Shift手动选择实写、28张跨页筛选整理、标签/收藏/取消收藏、批量删除取消、真实文件夹导出28份相对PNG、标准Markdown登记关联/按ID重开/右键改名、14节指南和重启、4张未分类确认删除及已有单张删除实时计数均执行。所有图片为受控2×2 PNG，不证明实时截图。
- PASS：`node scripts/split-native-smoke.mjs projects-v020-fixed`，[11组分栏回归](evidence/v020-split-native.json)，含480×400真实编辑高度≥80px、Konva坐标、拖动比例/重启、合成PNG、SQLite保存、外部修改拒绝覆盖。
- PASS：`node scripts/document-native-smoke.mjs projects-v020`，[8组图文保存回归](evidence/v020-document-native.json)，图片注释配对、原分辨率标注像素和系统保存/重开/取消。
- PASS：`node scripts/templates-native-smoke.mjs projects-v020`，[10组模板/大纲回归](evidence/v020-templates-native.json)。
- PASS：`node scripts/import-native-smoke.mjs projects-v020-diagnostic`，[9组真实剪贴板/OLE/SQLite/重启回归](evidence/v020-import-native.json)。文件放入前仍校验目标归属，没有绕过遮挡保护。
- PASS：`node scripts/compare-native-smoke.mjs projects-v020`，[8组对比/列表/指南原生回归](evidence/v020-compare-native.json)，实际红蓝像素、透明度和同步变换。
- PASS：`node scripts/material-ui-smoke.mjs projects-v020-final`，[8组浏览器主题/布局检查](evidence/v020-material-ui.json)，深浅主题对比度及8种480–1440宽度；浏览器不能替代原生截图/数据库验收。
- PASS：`npm run desktop:build`，生产Windows x64 EXE **17,945,600字节**，PE FileVersion/ProductVersion=0.2.0，SHA256 `b76afddd222eb2ea62c2958b77e8872c80be83750a2928a6fad95a5234fcff2e`。

本轮原生共57组，浏览器8组。生产文件位于 `src-tauri/target/release/researchclip.exe`；未让生产EXE访问真实用户资料，原生运行使用同源debug隔离工作区。没有新增依赖，本轮未重复npm ci；不把上个版本的安装结果算成本轮执行。文档/权限/迁移检查与最终发布结果在后续补记。

## 失败记录、严重程度与复现

| 问题 | 原始结果/复现 | 修复与复验 |
| --- | --- | --- |
| P1 项目筛选漏传分页请求 | [首轮FAIL](evidence/v020-projects-filter-initial-fail.json)：移动28/32图后点未分类，分页仍包含全部，等待4卡超时 | 请求实际携带projectId/unclassified；新增严格转发单元测试；完整原生11组PASS，清空关键词后项目28/未分类4实际检验 |
| P1 文档改名规范路径不一致 | Rust真实文件改名测试ID相等断言FAIL；Windows规范路径和原路径不同 | 当前与历史路径统一父目录规范化，保留同一ID/项目；单元和真实右键改名均PASS |
| P2 小窗口编辑区被项目控件挤压 | [分栏FAIL](evidence/v020-split-compact-initial-fail.json)：480×400右侧编辑高度不足80px | 短窗压缩提示留白、隐藏重复底部状态栏；保留80px断言，11组复验PASS |
| 审查发现手动选择快照混入列表字段 | 严格Rust SelectedScreenshot不接受路径/缩略图等多余字段；未把单纯审查说成已执行FAIL | 选择与IPC均只保留id/revision，新增单元测试；原生Ctrl/Shift选择后的实际收藏及取消收藏PASS |
| 测试初始收藏状态不匹配 | [FAIL](evidence/v020-projects-fixture-fail.json)：手动选择测试已收藏外部范围3图，后续总数31与假设28不同 | 真实执行取消所选收藏并验证0，再测项目批量；不改业务收藏逻辑 |
| 测试在模态框完成前输入 | [FAIL](evidence/v020-projects-modal-timing-fail.json)：选择已清零但事务UI尚在关闭，后台输入未生效 | 等待模态销毁及工具重新可用；完整后续步骤PASS |
| 原生输入焦点/目标归属 | [文件夹焦点FAIL](evidence/v020-projects-focus-fail.json)；[拖拽FAIL1](evidence/v020-import-target-initial-fail.json)、[FAIL2](evidence/v020-import-target-retry-fail.json) | 文件夹辅助器仅提升已验证测试窗口，操作后恢复；拖拽仍保留归属检查，只增加进程/窗口类诊断。后续完整套件PASS；遮挡/焦点依赖仍需人工，未断言根因已完全解决 |
| 浏览器在开发热重载期间读取空DOM | [FAIL](evidence/v020-material-reload-initial-fail.json)，无页内脚本异常 | 停止改动源文件/文档后稳定执行原断言，最终8组PASS；不作为产品崩溃证据 |

开发中还有真实编译/检查失败：扩展filter后的旧测试缺默认字段、测试Rust语法、schema断言仍为2、Clippy嵌套if、React组件文件导出普通helper及新增EditorSession测试误用历史字段。均修正后完整重跑，未移除检查或降低断言。

## 未执行项与Windows人工清单

以下均NOT_RUN，不能按代码存在或上述受控图验收当作通过：

1. 运行生产EXE升级自己的备份副本，核对项目/笔记/标注/来源、原图及备份；生产完整用户流程、5000图批处理延迟/内存和长稳未验收。
2. 真实科研项目跨页Ctrl/Shift、组合关键词/标签/收藏筛选、批量失败重选；删除取消与明确数量确认，包含打开的脏卡片/脏标注及退出保护。
3. 多个实际.md/大assets关联、中文/英文/LaTeX、改名/另存、外部移动后的缺失提示、权限不足/磁盘满/被占用时索引与正文保存分别报错。
4. 原生生产界面深浅主题、中文IME、125/150/200%与混合DPI/多屏；剪贴板/资源管理器拖拽被其它置顶窗口遮挡时的表现；指南任务目录/章节搜索及长文。
5. 实时截图/快捷键、OCR/贴图、托盘恢复和真正退出的完整硬件矩阵；历史QA-001退出句柄及QA-002托盘间歇问题继续跟踪。
6. Windows10、macOS、签名、新NSIS安装/卸载复验均NOT_RUN。本轮仅发布未签名EXE，WebView2为前提，不宣称完成安装器验收。

阶段结论：本轮已执行的最终检查无未解决FAIL；核心项目/批量/迁移均有真实SQLite与Windows正向证据，无已证实的新P0/P1。建议交付本阶段与用户授权的普通GitHub Release；全部人工/硬件矩阵尚未通过。非空项目连同内容删除、外部文件自动跟踪为明确NOT_IMPLEMENTED，不新增到本轮。


## 新增和修改文件

```text
CHANGELOG.md
README.md
docs/ARCHITECTURE.md
docs/PRD.md
docs/RELEASE-NOTES-0.2.0.md
docs/ROADMAP.md
docs/USER_GUIDE.md
docs/qa/RELEASE-0.2.0.md
docs/qa/TEST_REPORT.md
docs/qa/evidence/v020-compare-native.json
docs/qa/evidence/v020-document-native.json
docs/qa/evidence/v020-import-native.json
docs/qa/evidence/v020-import-target-initial-fail.json
docs/qa/evidence/v020-import-target-retry-fail.json
docs/qa/evidence/v020-material-reload-initial-fail.json
docs/qa/evidence/v020-material-ui.json
docs/qa/evidence/v020-projects-filter-initial-fail.json
docs/qa/evidence/v020-projects-fixture-fail.json
docs/qa/evidence/v020-projects-focus-fail.json
docs/qa/evidence/v020-projects-modal-timing-fail.json
docs/qa/evidence/v020-projects-native.json
docs/qa/evidence/v020-split-compact-initial-fail.json
docs/qa/evidence/v020-split-native.json
docs/qa/evidence/v020-templates-native.json
package-lock.json
package.json
scripts/check-docs.mjs
scripts/compare-native-smoke.mjs
scripts/image-drop-input.ps1
scripts/knowledge-native-smoke.mjs
scripts/library-native-smoke.mjs
scripts/projects-native-smoke.mjs
scripts/storage-native-smoke.mjs
src-tauri/Cargo.lock
src-tauri/Cargo.toml
src-tauri/build.rs
src-tauri/capabilities/main.json
src-tauri/examples/markdown_dialog.rs
src-tauri/migrations/003_projects.sql
src-tauri/src/commands/library.rs
src-tauri/src/commands/mod.rs
src-tauri/src/commands/projects.rs
src-tauri/src/lib.rs
src-tauri/src/services/markdown_document.rs
src-tauri/src/storage/library.rs
src-tauri/src/storage/migrations.rs
src-tauri/src/storage/mod.rs
src-tauri/src/storage/projects.rs
src-tauri/src/storage/search.rs
src-tauri/tauri.conf.json
src/features/desktop/store.test.ts
src/features/desktop/store.ts
src/features/document/MarkdownPanel.tsx
src/features/document/api.ts
src/features/document/store.test.ts
src/features/document/store.ts
src/features/guide/GuideView.tsx
src/features/library/LibraryCard.tsx
src/features/library/LibraryView.tsx
src/features/library/store.test.ts
src/features/library/store.ts
src/features/library/use-search.ts
src/features/projects/BulkActions.tsx
src/features/projects/ProjectBar.tsx
src/features/projects/api.test.ts
src/features/projects/api.ts
src/features/projects/batch.test.ts
src/features/projects/batch.ts
src/features/projects/selection.test.ts
src/features/projects/selection.ts
src/features/projects/store.test.ts
src/features/projects/store.ts
src/shared/api/library.test.ts
src/shared/api/library.ts
src/styles.css
```


## 最终文档检查

PASS / 0：`npm run check:docs`（48文档/298本地链接，9个项目命令与最小权限一致，v1/v2已发布迁移不变）；`npm run lint`、`git diff --check`。源码与公开下载结果另以发布回执记录。

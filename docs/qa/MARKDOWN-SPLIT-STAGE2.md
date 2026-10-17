# Markdown 动态分栏工作区验收（第二阶段）

日期：2026-10-17，Windows11 x64 / Node22.22.2 / npm10.9.7 / Rust1.99 MSVC / Tauri2.12.3。范围：收尾第一阶段图文文档，增加同一主窗口内的单活动Markdown动态分栏；没有新增数据库迁移、截图实现、云服务、多文档标签页或发布新版。

## 实现与操作

功能状态：已实现。代码存在不等于验收，本报告下表分别列实际自动化、受控原生测试和未执行项。第一阶段记录见 [图文文档报告](MARKDOWN-DOCUMENT-STAGE1.md)，当前接口见 [架构第15节](../ARCHITECTURE.md)。

- `App → WorkspaceSplit → original-workspace(main)`左侧容器始终存在；只有document.id存在才创建`separator + MarkdownPanel`。关闭时右侧实际卸载，原左侧DOM恢复完整宽度。`MarkdownPanel`按文档ID重建，`DocumentConfirmation`负责统一三选一确认。
- 顶部“新建空白 Markdown”：创建只在内存中的untitled.md并聚焦；直接输入源码。标题、粗斜体、列表、代码、链接、图片、中文/英文、LaTeX源码均为普通文本；预览采用既有CommonMark组件，不执行HTML，不加载远程资源，公式排版未实现。
- “打开 Markdown”：系统对话框选择.md；无标记文件按原文编辑，不自动加H1或私有元数据。第一阶段带版本标记的文档按原图文块恢复；“新建 Markdown”继续创建可排序的图文编排文档，可显式转普通源码，当前图片/注释顺序保留。
- 右侧Ctrl+S/保存首次弹系统对话框，之后更新已授权文件；Ctrl+Shift+S/另存选择新文件。首次/另存拒绝覆盖，继续保存检查外部改动，临时文件sync后原子提交。图片只保存到同目录assets，受控相对引用，原图不改。
- 左侧素材卡“插入到 Markdown”将已保存标注合成图和已有笔记快照插在右侧记住的光标/选区；未保存标注会提示先保存。左侧编辑器同名按钮则合成当前向量，允许未保存编辑，不改原图或原编辑器保存点。源码插入可撤销/重做；图文模式追加整体内容块。
- 默认50/50；拖动分隔线，或聚焦后方向键/Home调整。偏好通过既有本地设置服务保存，重启恢复；小窗口只调整实际显示宽度。没有永久Markdown侧栏页面或独立系统窗口。
- 右上关闭：干净文档直接关闭；脏文档保存/放弃/取消，保存失败不关闭。切换左侧页面不丢右侧内容；只在右侧处理Markdown保存和历史快捷键，左侧标注历史独立。

## 实际命令与结果

所有原生脚本使用独立artifacts工作区、debug程序和明确生成的320×200测试PNG，SQLite、系统对话框、合成渲染与文件系统是真实执行。受控PNG不能证明本轮全屏/区域真实屏幕捕获。生产EXE本轮完成编译，完整原生功能套件运行的是debug程序。

| 检查/命令 | 状态、退出码及结果 |
| --- | --- |
| `npm run check` | PASS/0，最终28文件209项；TypeScript、ESLint、Vitest、Vite生产构建。最终日志artifacts/document-stage2-delivery-check.log |
| `npm run format:rust` | PASS/0，cargo fmt --check |
| `npm run check:rust` | PASS/0，Clippy --all-targets -- -D warnings |
| `npm run test:rust` | PASS/0，94项，0失败/忽略；含原93项和分栏配置兼容/持久化/非法值测试 |
| `node scripts/with-rust.mjs tauri build --debug --no-bundle` | PASS/0，实际启动Windows WebView2主窗口 |
| `node scripts/split-native-smoke.mjs stage2-20261017-delivery` | PASS/0，11组，含15项核心需求映射；[结果副本](evidence/markdown-split-stage2.json) |
| `node scripts/document-native-smoke.mjs stage2-20261017-preview-errors` | PASS/0，原有图文编排8组；[结果副本](evidence/markdown-blocks-stage2.json) |
| `node scripts/editor-native-smoke.mjs mdstage2-20261017-editor` | PASS/0，原标注/撤销/项目重开/导出与逐像素剪贴板6组；[结果副本](evidence/markdown-editor-stage2.json) |
| `node scripts/storage-native-smoke.mjs mdstage2-20261017-storage` | PASS/0，真实SQLite/文件/重启/删除8组；[结果副本](evidence/markdown-storage-stage2.json) |
| `node scripts/workflow-smoke.mjs mdstage2-20261017-workflow --fixture --native-save-controls` | PASS/0，9组；[结果副本](evidence/markdown-workflow-stage2.json) |
| `$env:CARGO_TARGET_DIR=.../src-tauri/target/document-stage2; npm run desktop:build` | PASS/0，首次1分42秒；最终交付重建40.24秒，无安装包 |
| `npm run check:docs`、`git diff --check` | 最终文档完成后执行，结果见文末 |

### 需求对应的实际证据

| 测试 | 状态与证据 |
| --- | --- |
| 1 初始不显示分栏 | PASS，真实DOM无panel/separator，左侧完整宽度 |
| 2 新建空白后显示 | PASS，untitled.md空文本、聚焦、磁盘无文件 |
| 3 打开已有文件 | PASS，真实系统Open、源码/图片及第一阶段块顺序 |
| 4 关闭干净文档隐藏 | PASS，pane/separator detached |
| 5 关闭脏文档确认 | PASS，真实三选一对话框 |
| 6 取消关闭保留 | PASS，实际源码内容保持 |
| 7 保存失败保持面板 | PASS，真实磁盘外部修改冲突；文件与草稿保留，待关闭对话框继续显示 |
| 8 拖动与最小宽度 | PASS，真实鼠标拖动/键盘Home，极限拖动左侧>=279px；纯函数覆盖窄宽度 |
| 9 比例保存恢复 | PASS，读取真实settings.json，打开/关闭及实际进程重启恢复 |
| 10 左侧切换不丢文档 | PASS，标签→素材库，右侧全文保持 |
| 11 左侧插入 | PASS，真实键盘选择“DELETE”，素材卡插入恰好替换对应选区；当前编辑器也可插入 |
| 12 保存图片路径 | PASS，磁盘.md等于当前源码、相对assets两张PNG；320×200及标注红色像素检查 |
| 13 新建保存关闭重开 | PASS，真实Save/Open/Save As、Save and Close和进程重启 |
| 14 关闭恢复左侧 | PASS，原宽度恢复、同一个canvas DOM、同一图/4对象保持；滚动精确位置人工未测 |
| 15 分栏后标注坐标 | PASS，真实画布拖动产生矩形x110/y10/w45/h30，读回SQLite误差<1.5原图像素；纯函数缩放往返正确 |
| 源码与左侧历史隔离 | PASS，右侧Ctrl+Z恢复源码，左侧仍4个标注 |
| 最小窗口 | PASS，480×400无文档级水平溢出；右侧编辑区>=80px，实际截图已查看 |
| 快速创建关闭 | PASS，连续3次新建/放弃/关闭，无残留panel或WebView错误；不是长期泄漏性能测量 |

## 失败记录与修复

- FAIL：早期类型检查中DocumentConfirmation误用了局部source变量，TypeScript报错；修复后全部检查通过，未屏蔽规则。
- FAIL：`stage2-20261017-a`光标注入测试插图落文末。脚本通过DOM setSelectionRange注入没有触发React选择事件，不能单独证明普通用户键鼠必现。增加onBlur读取真实选区，改用Home/方向键/Shift进行真实选择；b、final和responsive三轮完整原生测试通过。
- FAIL/P2：`stage2-20261017-compact`保留编辑区至少80px断言失败，实际400px高窗口被工具栏/长提示/路径挤压。已在窄窗口使用有可访问名称的图标工具按钮、单行成功提示和折叠路径；未降低断言，responsive通过并查看截图。
- 初次构建的动态/静态重复导入警告已修复，确认对话框拆为独立组件，MarkdownPanel按需加载。Vite的插件耗时提示不代表构建失败。
- 审查发现P2：图文模式的全页预览会卸载块编辑器，原错误提示若跟随子编辑器会隐藏。改为面板公共提示区域；preview-errors原生套件在预览中触发真实外部文件冲突，错误可见且原文件保持，PASS。
- 第一阶段旧“科研卡片原图Markdown目录导出”辅助程序仍因Shell前台/目录选择限制为BLOCKED，详见第一阶段报告；本轮未把它当作已通过，也没有修改该业务导出实现。

## 限制与人工验收（全部NOT_RUN）

1. 退出旧的普通ResearchClip实例后运行本阶段EXE：先确认没有分栏；新建空白输入中文输入法组词、英文、数学公式源码、代码/列表，切换深浅主题，保存后用Obsidian或VS Code打开检查。自动化直接填入中文，不等同IME组合输入人工验收。
2. 左侧执行真实全屏、区域截图及全局快捷键；右侧文档保留。打开图片标注，放大/平移后多次拖动分隔线，手动确认鼠标、手柄、标注位置和左侧滚动恢复。
3. 分别用素材卡与编辑器插入截图；检查文字/箭头/透明背景；保存整个.md+assets文件夹后移动到另一个目录，用外部Markdown编辑器查看。
4. 编辑后关闭，分别选择取消、放弃、保存；首次保存再取消系统对话框；对同名另存、缺图、只读目录、磁盘满/文件占用进行人工失败检查。已自动验证的外部修改冲突不能代替这些全部故障矩阵。
5. Windows100%/125%/150%/200%缩放、不同分辨率/多显示器、长期编辑与内存分析、macOS均未本轮验收。
6. 历史QA-001素材库关闭句柄报错、QA-002托盘恢复超时仍未关闭；本轮小规模退出通过不代表已修复。不建议据此宣布完整正式版发布验收完成。

其他限制：单活动文档；源码历史最多100快照且每方向8MiB；最多100图文块、32张受控图片（含撤销保留资源）、总128MiB、文本2MiB。图片是插入时快照，不随源图后续修改更新；草稿不承诺断电/强杀恢复。只收集ResearchClip受控PNG，不自动复制普通外部Markdown的任意附件；远程图片/HTML/公式排版不渲染。失败可能留新孤立assets，绝不自动清理用户旧文件。显式转源码后由用户直接维护正文，不自动反向重建图片/注释编排。

## 修改文件

第一阶段新增模型/文件服务/安全预览以及本阶段分栏的当前完整新增代码：

- src/features/document/model.ts、api.ts、store.ts、DocumentView.tsx、LibraryPicker.tsx、MarkdownPreview.tsx
- src/features/document/WorkspaceSplit.tsx、MarkdownPanel.tsx、DocumentConfirmation.tsx、InsertButton.tsx、split.ts
- src/features/document/model.test.ts、store.test.ts、preview.test.tsx、split.test.ts
- src-tauri/src/commands/document.rs、src-tauri/src/services/markdown_document.rs
- scripts/document-native-smoke.mjs、scripts/split-native-smoke.mjs

当前相对原仓库有内容变更的既有代码/配置：

- src/app/App.tsx、layout/Toolbar.tsx
- src/features/editor/EditorView.tsx、src/features/library/LibraryCard.tsx
- src/features/desktop/store.ts、store.test.ts、DesktopExperience.tsx
- src/shared/api/settings.ts、src/styles.css、vite.config.ts
- src-tauri/src/services/settings.rs、services/mod.rs、commands/mod.rs、src/lib.rs、storage/mod.rs
- src-tauri/build.rs、capabilities/main.json、examples/workflow_input.rs、examples/markdown_dialog.rs
- package.json、package-lock.json、scripts/check-docs.mjs、scripts/markdown-native-smoke.mjs
- README.md、docs/ARCHITECTURE.md、本报告、第一阶段报告与evidence结果副本

Sidebar与ui-store在第一阶段曾新增Markdown导航，本阶段撤回该永久入口，最终内容与原仓库一致。没有改Cargo.lock或数据库迁移，没有新建第二套状态框架。第二阶段未新增依赖；第一阶段仅增加react-markdown10.1.0并已干净npm ci。测试辅助代码只控制指定PID与测试目录，保留所有失败artifacts。

## 本阶段程序

src-tauri/target/document-stage2/release/researchclip.exe；15,544,320字节；SHA256=B30AF74B23C6B618C8F4423F843585AB1276A123BE3ADB48C63822E819677BF7。构建成功，无签名，未重新打安装包/发布GitHub Release；旧release-audit与document-stage1产物保留。生产版手动启动先退出旧普通实例，避免单实例机制恢复旧程序。原生功能测试使用隔离debug版，不声称生产版和用户真实资料全流程已验收。

## 最终复核

最终`npm run check`（artifacts/document-stage2-delivery-check.log）PASS/0，28文件209项；类型、ESLint及Vite构建通过。最终debug重建及`stage2-20261017-preview-errors`图文编排8组、`stage2-20261017-delivery`动态分栏11组均PASS/0，errors=[]。预览模式的真实保存冲突仍明确显示错误。

`npm run check:docs`最终PASS/0（28文档156条本地链接）；`git diff --check`与迁移/Cargo.lock无变化检查PASS/0。Windows文件辅助脚本node --check均PASS/0。release交付构建PASS/0，40.24秒；本节产物hash为实际Get-FileHash结果。

没有尚未修复的本次新增功能自动化失败；早期失败及环境BLOCKED均保留，不把人工项标PASS。建议先完成列出的Windows人工验收；历史发布门槛问题未解决，不能宣布完整正式发布验收通过。本阶段结束，等待用户下一条指令。

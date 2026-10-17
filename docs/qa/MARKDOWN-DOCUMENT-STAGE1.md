# Markdown图文文档第一阶段验收

日期：2026-10-16，Asia/Hong_Kong。范围为新建、图片/注释编排、CommonMark预览、标准.md与assets保存/重开、未保存保护；不实施分栏窗口、数据库迁移或云端服务。当前功能为工作区增量修改，未替换此前GitHub预发布附件。

## 使用方法与数据契约

顶部“新建 Markdown”或侧栏“Markdown 文档”进入单文档工作区，填写标题，点击“添加图片”从分页素材库勾选。选择顺序即添加顺序，跨页选择保留；图片必须先将标注保存到素材库。有未保存标注时拒绝静默使用旧版本。导入复用现有Konva合成渲染，注释为笔记/当前卡片草稿的添加时快照，不回写原资料。

图片上方、注释紧跟下方构成同一个ImageBlock；上移/下移整块操作，不分开维护图片与文字数组。在任意块后插入TextBlock，或删除某块。实时预览在同一页面上下显示，支持中英文、标题、粗斜体、列表和代码；LaTeX源码保留、不渲染公式；禁HTML、远程图片和可点击链接。

Ctrl+S/“保存 Markdown”首次打开系统保存对话框，之后更新已授权当前文件；Ctrl+Shift+S/“另存为 Markdown”选择新文件。首次及另存拒绝同名覆盖；继续保存比对磁盘快照，外部修改时保留草稿并提示另存。每个PNG为assets/下稳定唯一名，Markdown引用相对路径。资源先提交、Markdown临时文件sync后最后原子提交；失败可遗留未引用的本次资源，不自动删除任何旧文件。

打开已保存文件恢复图文顺序及关联；修改后可继续保存。关闭/新建/打开替换脏文档与应用退出均提供保存、放弃、取消，取消系统文件对话框或保存失败不清草稿。导航到其他功能保留工作状态。文档不写SQLite，不改原图/标注/笔记，不建立第二套素材库。

标准Markdown包含普通标题、图片和正文，附隐藏HTML注释记录版本、ID、块类型/关联及UTF-16正文长度；普通Markdown软件可直接阅读。长度防止用户正文中类似注释误切块。外部修改若破坏结构，安全拒绝结构化重开；普通无标记.md作为文本导入，不自动复制任意外部附件。移动时保留整个文件夹。更多接口细节见 [架构第14节](../ARCHITECTURE.md)。

## 自动测试执行记录

Windows11 x64 / Rust1.99 MSVC / Node22.22.2 / npm10.9.7；Tauri2.12.3。原生测试使用debug程序、隔离settings/workspace/Tauri目录和明确生成的320×200测试PNG，**不证明本轮真实屏幕读取或硬件矩阵**。原始结果保存在artifacts，失败不删除。

| 命令 | 状态 / 实际结果 |
| --- | --- |
| `npm view react-markdown version license engines --json` | FAIL/1，受限环境npmmirror DNS失败；改为指定官方registry后核对10.1.0、MIT、React>=18，PASS/0 |
| `npm install --save-exact react-markdown@10.1.0 --registry=https://registry.npmjs.org` | PASS/0；仅一个新直接运行依赖；与原锁文件比较，既有package版本0处升级 |
| `npm ci --registry=https://registry.npmjs.org` | PASS/0；294包安装、295包audit，当前npm公告0项；非Rust漏洞扫描 |
| `npm run typecheck` | PASS/0 |
| `npm run lint`（首次） | FAIL/1，picker在effect内同步设置loading触发Hooks规则；改为按页派生异步资源状态，无禁用规则 |
| `npm test -- src/features/document src/features/desktop/store.test.ts`（早期） | PASS/0，26项；当时原include未发现tsx，不能声称已跑预览安全测试 |
| `npm run check`（tsx发现范围修正后及最终修改后） | PASS/0，27文件199项；全部既有测试保留，含两项实际React预览渲染测试；类型/ESLint/Vite生产构建通过 |
| `npm run test:rust`（首次） | FAIL/101，复用files校验模块原为私有；仅改为crate内可见，未授予WebView权限 |
| `npm run test:rust`（修正后和最终核心修改后） | PASS/0，93项、0失败/忽略；原87项加6项真实临时文件/失败用例 |
| `npm run format:rust`；`npm run check:rust` | PASS/0；cargo fmt --check及all-targets Clippy -D warnings；最后辅助程序改动仍须最终复核 |
| `npm run tauri -- build --debug --no-bundle`及workflow_input构建 | PASS/0，已启动实际Windows程序 |
| `node scripts/document-native-smoke.mjs stage1-20261016-a` | PASS/0，8组真实Windows验收，见下表；代码后续仅补窄窗口样式与错误提示，最终复验另记 |
| `node scripts/editor-native-smoke.mjs mdstage1-20261016-editor` | PASS/0，6组；五类标注、项目重开/撤销、原图、PNG/剪贴板像素、重启 |
| `node scripts/storage-native-smoke.mjs mdstage1-20261016-storage` | PASS/0，8组；SQLite/图片/标注/笔记/标签/删除及重启一致性 |
| `node scripts/workflow-smoke.mjs mdstage1-20261016-workflow --fixture --native-save-controls` | PASS/0，9组；真实图片剪贴板及占用、系统保存、快捷键注册冲突和重启；非真实屏幕捕获 |
| `node scripts/markdown-native-smoke.mjs mdstage1-20261016-legacy` | FAIL/1，旧目录辅助程序只改可见Edit，实际导出落在另一个自建测试目录：editor-native-mdstage1-20261016-editor/exports/ResearchClip-Export；未删除该失败证据、未触碰真实科研目录 |
| 同旧导出套件，ID为legacy-retry / legacy-final | FAIL/1；逐字输入被Shell自动补全截断、显式EN_CHANGE仍未改变实际选择。增加实际返回目录断言后拒绝误判通过 |
| 同旧导出套件，ID为legacy-scoped | BLOCKED，命令退出1；改用限制在测试PID前台的Ctrl+L导航，但当前Windows拒绝前台条件，安全中止。未取消PID校验；该套件本轮不能标PASS，需人工复验 |
| `$env:CARGO_TARGET_DIR=.../src-tauri/target/document-stage1; npm run desktop:build` | PASS/0，首次优化构建1分50秒；独立目录保留旧发布程序。窄窗口样式之后的最终构建状态在文末记录 |

一次原生目录辅助程序编译/执行的自动审批超时，未实际执行；按工具允许重试一次后已执行，未作为产品测试失败。最终安全焦点拒绝属于运行环境限制，旧导出业务实现未修改，不能因此证明其功能通过或已损坏。

### 需求用例映射

| 用户要求 | 已执行证据 / 结果 |
| --- | --- |
| 创建空文档 | 模型与store单测、实际主界面未保存标志：PASS |
| 一张/多张图片注释 | 模型单/多图、store调用既有renderer、实际勾选B/A/C默认笔记：PASS |
| 排序/删除中间块 | 100次移动关联校验、实际移动后删B保留A/C及各自注释：PASS |
| 图文间文本与基础Markdown预览 | 特殊注释标记、中文/英文、列表/代码/粗斜体，React安全测试和实际WebView预览：PASS |
| 带标注图片 | 实际PNG320×200，红色矩形像素与原始测试PNG不同；导入箭头/文字复用renderer；已有完整五类标注原生回归PASS |
| Markdown/assets安全保存 | Rust同名拒绝、同名不同资源保护、缺图/坏UTF8/路径/写入失败；实际另存/继续保存、外部编辑冲突、取消无文件：PASS |
| 重新打开及顺序 | 模型无损往返、store重载、实际系统打开/应用重启保留A-文本-C：PASS |
| 中文/LaTeX/特殊字符 | UTF-8文件、中文空格文件名、美元/反斜杠/代码、emoji及相似元数据注释：PASS；公式排版未实现，不冒充渲染通过 |
| 未保存三选一 | store失败/取消保留、实际关闭取消/保存后关闭、真实窗口关闭取消/显式放弃：PASS |
| 原资料不改 | 新文档流程3张原图hash/3条SQLite记录保持，迁移和Cargo锁文件未改：PASS |

## 已知限制、失败恢复与人工清单

- 本阶段单文档会话、100块/32图、Markdown2MiB、PNG总128MiB；导航保留草稿，强制结束/断电恢复未实现。
- 合成图为导入时已保存标注快照，后续编辑源图不会自动更新文档；未保存的标注需先显式保存。图片注释修改不回写Note。
- 文档不是数据库项目；正文派生，工作状态/原生token不入文件。HTML注释元数据合法但不能保证经任意第三方编辑后仍结构化恢复；破坏标记时拒绝写回，不抹除草稿。
- 新目标或另存不覆盖同名；已打开文件只有显式保存及磁盘内容匹配时更新。原子替换不是跨多文件事务；可能遗留本次资源，不清理旧assets；不承诺抵御同等磁盘权限外部进程所有竞争。
- 预览不运行HTML/脚本，不加载远程图片、不打开链接；LaTeX仅保存源码，未加公式渲染器。普通外部Markdown附件不会自动收集。
- 历史QA-001素材库退出窗口句柄错误、QA-002间歇托盘问题未在本轮修复；本轮小规模存储退出通过不能据此关闭历史Bug。

人工项均未标通过：

| 人工操作 | 状态 |
| --- | --- |
| 在Obsidian/VS Code打开新.md；移动整个文件夹后查看附图、中文/代码/标题 | NOT_RUN |
| 在实际显示缩放125/150/200%、浅色系统主题和不同字体下检查图片/按钮/弹窗 | NOT_RUN |
| 新文档重复“添加→排序→删除→保存→重开”，确认选中图片与注释总是同行 | NOT_RUN（对应自动化有PASS，不能代替人工） |
| 手动执行原有卡片单/多Markdown导出目录选择，核对实际目录和图片 | NOT_RUN；原生自动化BLOCKED需重点补验 |
| 磁盘满、目录只读、文件占用、杀进程及备份恢复 | NOT_RUN；仅已列确定性失败/冲突路径自动通过 |
| 修改元数据标记、删除assets后重开，应明确报错且保留当前草稿 | NOT_RUN（规则/失败单测PASS） |
| macOS构建/运行与文件对话框 | NOT_RUN；当前Windows证据不能替代 |

## 文件变更清单

新增：

- src/features/document/model.ts、api.ts、store.ts、DocumentView.tsx、MarkdownPreview.tsx、LibraryPicker.tsx
- src/features/document/model.test.ts、store.test.ts、preview.test.tsx
- src-tauri/src/services/markdown_document.rs、src-tauri/src/commands/document.rs
- scripts/document-native-smoke.mjs、本文档；最终测试JSON副本另记

修改：

- package.json、package-lock.json、vite.config.ts、scripts/check-docs.mjs
- src/app/App.tsx、ui-store.ts、layout/Sidebar.tsx、layout/Toolbar.tsx
- src/features/desktop/store.ts、store.test.ts、DesktopExperience.tsx、src/styles.css
- src-tauri/build.rs、capabilities/main.json、src/lib.rs、src/commands/mod.rs、src/services/mod.rs、src/storage/mod.rs
- src-tauri/examples/workflow_input.rs、markdown_dialog.rs、scripts/markdown-native-smoke.mjs
- README.md、docs/ARCHITECTURE.md

## 第一阶段最终记录（后续第二阶段另记）

`node scripts/document-native-smoke.mjs stage1-20261016-final`：PASS/0，8组；最终480×400界面、真实文件保存及重启。`npm run format:rust`、`npm run check:rust`、`npm run check:docs`：PASS/0；文档27份146条本地链接。旧目录导出自动化仍为BLOCKED，不改记PASS。

最终独立release构建PASS/0（37.08秒）：src-tauri/target/document-stage1/release/researchclip.exe，15,541,248字节，SHA256=3A0CAA6BF6D168E3E555773A78D0855F049B9989B4437C56CEFEC233C6CA7299。未签名，不是新发布的GitHub附件；此处为第一阶段历史产物，不代表第二阶段。此前发布程序未覆盖。

本文保留第一阶段当时的页面/侧栏交互记录；用户随后明确授权第二阶段动态分栏，当前界面和最新结果见 [第二阶段报告](MARKDOWN-SPLIT-STAGE2.md)。第一阶段未自动发布新版。

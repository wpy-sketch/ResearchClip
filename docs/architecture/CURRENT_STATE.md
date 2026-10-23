# Step 00：现有项目事实审查

本文件冻结Step00开始时的schema11源码事实；下述“本轮”仅指Step00审查。随后授权实施的Step01已进入schema12，增量实现见[统一资源模型](RESOURCE_MODEL.md)和[Step01验收](../qa/RESOURCE_MODEL_STEP01.md)。

审查日期：2026-10-23（Asia/Hong_Kong）。源码基点：`53e4bfa994d4c648b5b4d0545cb1b87a164e033f`，主分支 `main`，应用版本 4.0.0。开始时工作区干净。本轮只新增五份规划/基线文档，不修改业务、依赖、权限、数据库或用户资料。

配套：[目标架构](TARGET_ARCHITECTURE.md)、[技术决策](TECHNOLOGY_DECISIONS.md)、[分阶段路线](../ROADMAP_RESEARCH_IDE.md)、[实际基线报告](../qa/BASELINE_REPORT.md)。这里的“已实现”指存在调用链和持久化实现，不等于所有硬件/人工验收通过。

## 1. 审查依据与文档冲突

已阅读 [AGENTS.md](../../AGENTS.md)、[README](../../README.md)、[PRD](../PRD.md)、[既有架构](../ARCHITECTURE.md)、[既有路线图](../ROADMAP.md)、[Pro约束](../PRO-DEVELOPMENT.md)，并交叉检查当前入口、功能模块、IPC、存储、001–011迁移、工具脚本及当前版本验收。旧 DATA-MODEL / DEVELOPMENT-PLAN 仅作为历史设计背景。

发现需要在后续文档维护中处理、但本轮不改写的历史描述：

- PRD 开头仍写“M1骨架”，ARCHITECTURE 的早期表结构仍列 `image_assets/cards`，与当前真实表不一致。
- README 部分段落仍写 schema3、LIKE 搜索、旧安装器文件名；当前为 schema11，截图搜索已有 FTS5 trigram 和短查询回退。
- 原始规划有回收站；当前 `delete_screenshot` 是显式确认后的数据库删除和受管文件清理，没有回收站。
- A06-M1 的 BLOCKED 是历史阶段，不能据此把当前手动 WebDAV 判成未实现；实际代码已接入 schema6 协议。
- 原生分栏旧脚本仍等待旧成功文案；本轮实际失败及补充验证单列在基线报告，不能沿用历史 PASS。

发生冲突时以当前用户约束和源码为准。本文不会把旧设计中的目标表、尚未实现的模块当作可调用接口。

## 2. 实际技术栈

版本取自现有锁文件，而不是 package.json 的版本范围。

| 层 | 实际实现/锁定版本 | 源码证据与判断 |
| --- | --- | --- |
| 桌面容器 | Tauri Rust 2.12.3、JS API 2.12.3、CLI 2.12.2；Windows WebView2 | [lib.rs](../../src-tauri/src/lib.rs)、[配置](../../src-tauri/tauri.conf.json)；不是 Electron |
| 前端 | React 19.3.0、TypeScript 6.0.3、Vite 8.3.4、Tailwind CSS 4.3.3 | [锁文件](../../package-lock.json)、[入口](../../src/main.tsx)、[Vite配置](../../vite.config.ts) |
| 状态 | Zustand 5.0.15，按 feature 分 store；无前端 SQL、科研正文 localStorage 持久化 | [界面状态](../../src/app/ui-store.ts)、各 feature/store.ts |
| Rust | edition 2024，最低 Rust 1.99；windows-rs 0.62.2 | [Cargo.toml](../../src-tauri/Cargo.toml)、[Cargo.lock](../../src-tauri/Cargo.lock) |
| 数据库 | rusqlite 0.37.0，bundled libsqlite3-sys 0.35.0，schema11 | [Storage](../../src-tauri/src/storage/mod.rs)、[迁移器](../../src-tauri/src/storage/migrations.rs)；不是前端 SQL 插件或 ORM |
| 图片编辑 | Konva 10.7.1 / react-konva 19.3.0，自定义矢量文档、几何、历史及 PNG 合成 | [EditorCanvas](../../src/features/editor/EditorCanvas.tsx)、[模型](../../src/features/editor/types.ts)、[导出](../../src/features/editor/export.ts)；没有外购图片编辑器 |
| Markdown 编辑 | 原生 textarea 源文编辑＋自定义图文 block；自定义单文档 Zustand 会话 | [MarkdownPanel](../../src/features/document/MarkdownPanel.tsx)、[store](../../src/features/document/store.ts)、[model](../../src/features/document/model.ts)；目前没有 Monaco、CodeMirror、TipTap 或 Theia |
| Markdown 预览 | react-markdown 10.1.0、remark-math 6.0.0、KaTeX 0.19.0、mdast/CommonMark | [预览](../../src/features/document/MarkdownPreview.tsx)、[公式边界](../../src/features/document/MathFormula.tsx)；禁用户 HTML，图片受控 |
| 原生捕获 | 显示器/区域：Win32 GDI；窗口：WGC/D3D11，同 EXE 短生命 worker | [GDI](../../src-tauri/src/capture/windows.rs)、[窗口捕获](../../src-tauri/src/services/window_capture/mod.rs)、[worker](../../src-tauri/src/services/window_capture/worker.rs) |
| OCR/本地模型 | Windows.Media.Ocr；可选 Python/ONNX MathOCR、共享图文向量；可选 Qwen 本地进程 | [OCR](../../src-tauri/src/services/ocr.rs)、[MathOCR](../../src-tauri/src/services/math_ocr.rs)、[向量](../../src-tauri/src/services/embeddings.rs)；不等于 Codex/Claude Agent |
| 网页剪藏 | 独立 MV3 扩展 1.0.2＋Rust Native Messaging Host＋有界协议 | [扩展包](../../browser-extension/package.json)、[Host](../../browser-extension/native-host/src/main.rs)、[协议](../../browser-extension/protocol/src/lib.rs) |

## 3. 功能盘点

| 模块 | 状态 | 实际能力 | 缺口/边界 |
| --- | --- | --- | --- |
| 全屏、区域、快捷键 | 已实现 | 冻结帧、物理像素裁切、取消、复制/另存、入库 | 完整混合 DPI/多屏/HDR/远程桌面矩阵未验；非 Windows 不承诺 |
| 窗口、滚动截图 | 已实现，有支持范围 | WGC 窗口自身图像；有界滚动拼接/手动或支持窗口自动滚轮 | 受保护/最小化/复杂页面等按当前限制；本轮不把实现视作全部硬件通过 |
| 无损标注/版本/贴图 | 已实现 | 八类绘制对象，视口、撤销重做、保存/历史恢复、合成 PNG、独立置顶贴图 | `pen` 只是平面坐标数组，没有压感/倾斜/时间样本；贴图不跨重启恢复 |
| 素材库/笔记/标签/项目 | 已实现 | 分页、收藏、CAS 写入、卡片自动保存、批量事务、图片/Markdown 项目分类 | 科研 project 是逻辑分类，不是代码根目录；无回收站、独立备份产品 |
| Markdown | 已实现，Research IDE 所需多会话部分未实现 | 标准 `.md + assets`、图文块、源文/预览、公式、模板、大纲、重命名、外部打开 | 当前只有一个活动文档；不能直接宣称具有多标签、多工作区或代码 IDE |
| OCR/公式 | 已实现但识别质量有限 | 本机语言包、有界区域/批量 OCR、纠错索引；可选公式识别后人工校对 | 模型运行时/权重可选；本轮没有重新下载/评估模型准确率 |
| 搜索/相似/语义 | 已实现，语义属实验 | FTS5/短词 LIKE，pHash/dHash/颜色，512维共享向量、本机有界查询 | 未支持 PDF 全文统一检索；模型失败有独立错误；2000项规模性能不由小样本保证 |
| 文献引用 | 部分满足目标 | 手工、Crossref DOI 查询、BibTeX 预览导入、引用键、截图/Markdown 关联 | 不是 Zotero 集成；没有 Zotero item/attachment key、标准批注读写与 Reader 联动 |
| 双向链接/图谱/图片锚点 | 已实现 | `researchclip://document|screenshot/<id>`，稳定 ID、反向索引、原图 SHA 绑定锚点 | link_type 只允许两类，不能直接塞 PDF/代码目标；不是 PDF 位置系统 |
| WebDAV | 已实现，服务商验收部分未完成 | 用户手动启动、资源级协议、强 ETag 条件写、冲突/队列/删除意图 | 不同步整个 SQLite；文献/图谱/新类型不能默认加入；真实坚果云仍有历史阻塞 |
| 命令中心/有限工作流 | 已实现 | 固定类型动作、确认识别结果、取消/重试；设置持久化 | 没有 LLM 规划、通用脚本执行或 Agent 权限代理；不能扩展成通用工作流引擎 |
| PDF 阅读 | 未实现 | 现有图片可引用来源 | 无 PDF renderer/text layer/page anchor/附件管理；浏览器能显示 PDF 不等于项目已接入 |
| Zotero 插件 | 未实现 | 无对应包、manifest 或适配服务 | 插件需运行在 Zotero 进程，通过其 API 写入；不直写 Zotero DB |
| HarmonyOS | 未实现 | 无 ArkTS 工程、设备协议或压感格式 | Windows 画笔不能代替 HarmonyOS 真机输入验证 |
| 代码 IDE | 未实现 | 无项目文件树、语言服务器、终端、调试适配器 | 项目分类/Markdown textarea 不等于代码工作区 |
| Codex / Claude Code | 未实现 | 本地受控模型子进程可提供生命周期经验 | 无官方 Agent 协议适配、权限批准、Diff 审阅、会话恢复或账号集成 |

“已实现但待验证”与“未实现”分开：前者需要回归/硬件证据，后者必须先经过后续授权开发。具体本轮执行结果只见基线报告。

## 4. 动态分栏能否扩展

[WorkspaceSplit](../../src/features/document/WorkspaceSplit.tsx) 在 `document?.id` 存在时才挂载右栏，左侧 children 保持原位置；比例经既有 SettingsService 保存，ResizeObserver、键盘和指针拖动具有清理/取消逻辑。[split.ts](../../src/features/document/split.ts) 显示最小宽度会随窗口缩小，不能把 280px 写成所有窗口下的绝对保证。

可直接保留：无文档不占位、焦点/光标插图、左右撤销互不串扰、保存失败保留草稿、条件 lazy 加载和比例偏好。

扩展障碍：

1. `useDocumentStore` 是单实例；`document/token/path/assets/past/future/phase/pending` 都是单份。`MarkdownPanel key=documentId` 还包含局部预览、大纲和滚动状态。
2. 编辑器虽已有最多8个图片会话/128 MiB 图像预算，`tool/style/gesture` 部分仍是全局；笔记有自己的脏稿队列，不能复制一套全局工作区正文。
3. 多处 OCR、插图、引用、链接、项目登记、工作流、退出和同步直接访问当前 Markdown store；多会话后必须明确目标 session，过期回执不能写到新面板。
4. Rust DocumentFiles 当前最多4个 Grant；会话多开前需要容量与 token 回收实验。文件/模板对话框复用 CapturePermit，后台保存不能无界抢占截图。
5. `activeView` 是一个枚举；科研项目切换只改变筛选。多工作区应先表示不同研究项目的布局/会话集合，不能直接变成多数据库根目录并发写入。

结论：适合**增量扩展**，不适合只换分栏库就宣称完成。先在现有 store 工厂上建立有界 session 集合和集中脏稿守卫，再引入固定区域＋标签；只有经实际用例证明必要才增加更多嵌套分栏。布局持久化只存会话资源引用，不存正文、凭据或可重用授权 token。

## 5. 当前数据库与文件边界

### 5.1 真实迁移序列

| schema | 已发布变更 |
| --- | --- |
| 1 | `app_metadata` |
| 2 | `screenshots/notes/tags/screenshot_tags/pending_screenshots/pending_file_deletions` |
| 3 | `projects/screenshot_projects/markdown_documents`；文档为路径索引，不复制正文 |
| 4 | `ocr_content`、`screenshot_search` FTS5 trigram、视图/触发器 |
| 5 | `annotation_revisions`，已有标注原字节导入历史 |
| 6 | `annotation_heads`，同步 identity/config/state/queue/conflicts/tombstones/document_grants |
| 7 | `image_features` 派生索引 |
| 8 | `literature_references/screenshot_references/document_references` |
| 9 | `link_index/link_index_state/document_tags` |
| 10 | `annotation_anchors`；link_index 增加 anchor_id |
| 11 | `semantic_embeddings`；单图2048字节512维向量 |

[迁移器](../../src-tauri/src/storage/migrations.rs) 先拒绝未来版本、执行 quick_check 和结构校验；已有旧库用 `VACUUM INTO` 生成一致快照并验证后以禁止覆盖方式提交。每个版本独立 `BEGIN IMMEDIATE` 事务，失败回滚当前版本，**不是整个多版本升级共享一个事务**。成功早期版本可能已提交；后续计划必须支持从这个中间版本重试。当前快照是迁移安全措施，不是独立备份/恢复系统。

[Storage::open](../../src-tauri/src/storage/mod.rs) 使用工作区文件锁、rusqlite 连接、foreign_keys=ON、WAL、synchronous=FULL、5秒 busy timeout。主应用 LibraryService 是单写协调入口；扩展不能直接持有 SQLite 连接。

### 5.2 文件与身份

- 受管数据根下有 `researchclip.sqlite3`、`originals/<32位小写hex>.png`、`thumbnails/...`、`exports`、按需迁移快照及同步缓存。原图不变；创建先登记 pending、写文件，再事务激活；显式删除先记清理意图再删受管文件。
- Markdown 正文保存在用户选择的 `.md`，配套 `assets/researchclip-<id>.png`。数据库仅登记规范路径及现有文档 ID。模板在配置目录独立存种子，不是第二套笔记库。
- `DocumentFiles` 的 token 是进程内文件授权，记录上次读取/保存字节，保存前核对外部变更；普通保存原子替换**被授权的同一文件**，首次/另存/改名均禁止覆盖已有目标。
- `MarkdownDocument.id`/block ID 是编排或临时编辑身份，不等于普通 `.md` 在 `markdown_documents.id` 的持久身份；新通用导航必须采用后者，不能创建另一个资源 ID 映射层。
- 现有持久 ID 一般是32位小写hex；现有链接以 `(kind,id)` 判别类型。项目、截图、文档、引用的 ID 均应继续保留。

### 5.3 对新领域的承载能力

| 新领域 | 可复用 | 当前不能直接承载 | 必需扩展 |
| --- | --- | --- | --- |
| Zotero 文献 | literature_references、引用键、项目/文档关联、CAS | DOI 不能作为外部唯一身份；无 library/item/attachment 映射 | 外部键映射、来源版本/摘要、插件握手；同 DOI 不自动合并 |
| PDF/位置 | 文件提交、受管路径、异步任务、内部链接框架 | 图片存储只接受 PNG；截图锚点单位是原图像素 | 独立 PDF 文档类型、不可变内容摘要、页索引/标签/坐标格式 |
| 私有手写 | 矢量校验、历史/取消经验 | Annotation schema1 的 pen 不保留压感/倾斜/时间，也不绑定 PDF 页 | 独立 private ink 格式与 sidecar、页版本关联；不混入 Zotero 标准批注 |
| 代码项目 | projects 的逻辑研究归属、文件授权理念 | project 无文件系统根/读写能力/信任级别 | 项目根授权、文件 CAS、二进制/大小/链接防护、按需 LSP/终端 |
| Agent 会话 | 有界 worker、Job Object、取消/代次、日志脱敏经验 | 没有权限代理、供应商会话、Diff、工具审计；Job Object 不等于文件访问沙箱 | 独立受限会话/进程、提案与批准、补丁基准校验、实际 OS 隔离 |

SQLite 与文件体系可以继续使用，表结构和授权契约必须增量扩展；不能把新对象塞进截图 annotation JSON 或 Markdown 正文元数据来绕过迁移。

## 6. 源码审查地图

以下覆盖主入口和全部主要业务域，采用入口/模型/持久化/测试交叉阅读；不是对所有依赖、生成文件和历史脚本逐行安全审计。

| 域 | 主要源码及审查重点 |
| --- | --- |
| 组装/权限/生命周期 | [App](../../src/app/App.tsx)、[ui-store](../../src/app/ui-store.ts)、[lib](../../src-tauri/src/lib.rs)、[main](../../src-tauri/src/main.rs)、[build](../../src-tauri/build.rs)、[capabilities](../../src-tauri/capabilities/main.json)、[desktop](../../src-tauri/src/services/desktop.rs)、[settings](../../src-tauri/src/services/settings.rs) |
| 捕获/导入/贴图 | capture 下 geometry/windows；services 的 screen_capture/region_capture/scrolling/stitching/window_capture/image_import/pins；前端 capture/import/pins 和各 typed API、store 测试 |
| 编辑器 | editor 下 types/model/geometry/history/serialization/store/EditorCanvas/AnnotationNode/effects/export/RevisionHistory，以及 Rust services/editor、storage/revisions、anchors；原像素坐标与历史边界 |
| 素材/卡片/项目/导出 | [library-actions](../../src/app/library-actions.ts)、notes/library/projects/export 前端；commands/library、storage 的 library/knowledge/projects/markdown/files/search/ocr；事务、revision、失败保留 |
| Markdown/链接 | document 下 model/store/split/WorkspaceSplit/MarkdownPanel/MarkdownPreview/MathFormula/templates/outline/api；Rust commands/document/projects/links、services/markdown_document/templates、storage/links；保存与索引失败分开 |
| 文献/本地检索 | features/references/similarity/semantic，services/references/similarity/similarity_index/similarity_ai/embeddings，storage/references/similarity/embeddings |
| OCR/本地进程 | features/ocr/ocr-index/math-ocr，Rust OCR/MathOCR/EmbeddingService，math-ocr/worker.py、semantic-search/worker.py 与固定模型清单；有界输入、主动下载和取消 |
| WebDAV | features/sync，commands/sync、services/sync 的 runtime/engine/model/transport/credentials/tests、storage/sync；短DB锁与网络分离、凭据不落明文 |
| 剪藏 | browser-extension/src 的 popup/content/types，Native Host、protocol、services/clipper、storage/clips；外部 HTML/URL 校验、临时 token、幂等请求 |
| 命令/指南/QA | features/commands 的 model/runner/runtime/commands，guide/navigation/GuideView；scripts 下 docs/原生/浏览器检查与 package scripts；固定工作流不是 Agent |

## 7. 本轮结论与实施前置

保留 Tauri/React/Rust 模块化单体、同一 SQLite、同一 Markdown 文件服务。建议先做多会话和受限资源契约，再接 Zotero 只读与 PDF；标准批注写回、私有手写、HarmonyOS、代码执行和两种 Agent 分阶段验收。

既有基线并非全绿：旧分栏脚本、硬件矩阵和历史未解决问题必须继续展示。后续第一阶段需用户明确授权，并先解决测试脚本与当前文案/画布布局的失配；本轮不会自动实施。

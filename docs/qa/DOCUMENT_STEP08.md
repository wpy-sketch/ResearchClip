# Step08 科研模板、文档导航与知识链接

目标仅为Step08与v4.1.7正式发布；基点69e268d。设计先写入[文档组织契约](../architecture/DOCUMENT_ORGANIZATION_STEP08.md)。本轮复用原模板、H1–H6大纲、查找替换、稳定ID链接/反向索引、DocumentFiles和EditorSession，不重建文件服务或分栏。

## 模块与兼容性

新增文献综述、研究日志，数学模板加入Step07定理/证明；原自定义模板服务继续使用。DocumentDirectory管理已登记文档的逻辑文件夹、收藏、最近与失效项，外部移动通过既有指纹和原生选择器重连。ScopedLinkCompletion复用原LinkPicker，将补全绑定到当前源码/可视片段/旧caption。

document-navigation提供全文字段查找、原子替换和有界行Diff。DocumentHistory复用原save，再载入校验过的历史，正常保存并强制追加恢复版本。截图和文档共享shared/revisions元数据与Rust校验，修订正文分表。schema13迁移不读取外部文件、不伪造旧历史；001–012发布迁移不改。原图不可变、旧Markdown图文关联和资源ID保留。

有界新IPC仅允许main窗口，使用稳定ID或当前授权token；唯一新路径来自原生文件选择器。没有通用fs/shell/SQL授权。若文件保存后历史/索引失败，错误明确说明部分完成并保留草稿，不伪称已保存全部状态。新增历史/目录尚未加入旧WebDAV协议。

## 本轮实际结果

Windows11、Edge与WebView2实际执行，命令退出码均为0；详细检查项与本机日志位置见[机器证据](evidence/v417-editor-checks.json)。测试只使用新建隔离资料，未升级真实用户数据库。

| 检查 | 实际结果 |
| --- | --- |
| npm ci --ignore-scripts | 干净安装324包，无新增依赖；锁文件仅应用版本变化 |
| npm run check | 类型、lint、629项/74文件、生产前端构建通过 |
| npm run format:rust / test:rust / check:rust | 245通过、7忽略；格式与Clippy通过 |
| node scripts/with-rust.mjs tauri build --debug --no-bundle | Windows嵌入式调试程序构建通过 |
| markdown-organization-native-smoke v417-verified | 12组通过：目录/收藏/最近、Diff/恢复、全文替换/大纲、同名补全/循环链接、重命名、旧图文、自定义模板、外部移动/删除、实际进程重启 |
| 既有7组原生脚本 | 53组通过：论文8、内容7、Markdown工作区7、工具栏10、工作区7、命令8、资源6 |
| 6组浏览器脚本 | 49组通过；目录/历史为明确标注的受控IPC，不能替代原生持久性验证 |
| plain-markdown-reader.ps1 -DocumentNavigation | 2组真实记事本RichEdit读取通过，UTF-8字节不变 |
| 发布基础脚本测试 | 16项通过，包含精确附件继承与归档字节校验 |
| npm run desktop:bundle | 生产Windows NSIS构建通过，缓存v413-distribution |
| installer-smoke.ps1 -RunId v417-distribution | 实际安装/占用保护/卸载/清理4组通过，16个真实AppData只读摘要不变；隔离安装已移除 |
| npm run check:docs / git diff --check | 文档结构、本地链接、协议权限与差异检查通过 |

001–012迁移无改动；隔离12→13迁移验证升级前一致快照、旧内容保留、失败回滚及未来版本拒写。原生从独立v11资料升级13，图像原件与Markdown字节不变。当前打开时间只在打开时更新，保存不伪造最近时间；同内容显式恢复仍追加新版本。

首轮发现前端素材库状态仍校验12，已修正13并重跑。旧迁移夹具与原生退出审计也相应检查13，保留原数据断言。实际WebView2在200%时历史弹窗溢出，改为布局稳定后测量约束；九组原生窗口/CSS缩放及60组浏览器目录/历史组合通过。自定义模板测试遵守原三文档上限并按普通源码模板验证真实图片，不改变业务上限或删除断言制造成功。

生产NSIS SHA-256：430410baa27d3a0ca016fb4ecc07739ff82112a04b8545df040feece8e7b6ed6。未签名；现有Vite大块提示保留。没有失败遗留的自动化用例；7个Rust忽略项、下述物理/用户资料项不计通过。

## 修改文件与设计

新增src-tauri/migrations/013_document_organization.sql、storage/document_organization.rs及测试、commands/document_organization.rs、storage/revision_meta.rs；在原projects/markdown_document服务接入登记和快照，主窗口仅新增8个受限IPC。原截图revision仅复用元数据校验，正文分表。

前端新增DocumentDirectory、DocumentHistory、DocumentNavigationTools、ScopedLinkCompletion、document-navigation、organization-api与样式/测试；增量修改MarkdownPanel、原模板/LinkPicker、文档/项目store、CommandCenter和Modal。没有第二套文件读写、ID或全局状态框架；弹窗监听器在卸载时清理。

指南新增第25章和两幅实际原生图，现25章/157例/38图；更新README、架构契约、发布说明与受控发布工作流。版本统一4.1.7，无新增第三方依赖。完整文件差异可由本版源码标签与基点69e268d比较。

## Windows人工验收与边界

使用隔离练习文件验收五类科研模板与自定义模板、目录树/收藏/最近、H1–H6定位、全文替换/一次撤销、源码/可视块/caption补全、同名ID与A↔B链接、版本Diff、先保存当前再恢复追加、新版本相同内容也追加、重命名/移动/删除与真实重启。

重新定位系统文件选择器须在解锁Windows中人工核对；自动化使用同一Rust指纹服务的隔离探针不能冒充原生选择器验收。物理中文IME、多显示器DPI、系统放大镜及真实资料升级仍需人工；既有锁屏OLE拖放问题保留。外部删除只报告失效，不提供未经确认的文件删除。单正文2MiB、文档历史128MiB/全库512MiB、Diff2000行预算，超限不自动删资料。

没有新增第三方依赖、PDF导出、独立备份恢复或外部Agent/Zotero业务。完成发布后停止，下一阶段需要用户新请求。

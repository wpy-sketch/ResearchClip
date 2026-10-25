> **v4.1.8 · 本地科研引用**：@补全、稳定文献ID、公式/截图来源、BibTeX导入导出和显式参考列表；SQLite14。指南目录自然排序及原创多色科研图标。见[本版说明](docs/RELEASE-NOTES-4.1.8.md)、[实际验收](docs/qa/CITATIONS_STEP09.md)与[下载指南](docs/RELEASE-COMPONENTS.md)。

<!-- Current delivery status -->
> **历史v4.0.0 · 独立验收与界面完善**：修复语义任务状态竞争，改善缩放阅读和六条任务路线；保留A01–A13与完整配套组件。实际结果及未解决问题见 [高级功能验收报告](docs/qa/ADVANCED_FEATURE_REPORT.md)。

# ResearchClip

# 使用Codex进行构建

面向科研人员的本地桌面截图、图片标注与 Markdown 知识管理工具。**当前构建：v4.1.8 Windows；发布与验收状态见本版记录**。

[GitHub发布版本](https://github.com/wpy-sketch/ResearchClip/releases/latest) · [v4.1.8正式回执](docs/qa/PUBLICATION-4.1.8.md) · [v4.1.8发布检查](docs/qa/RELEASE-4.1.8.md) · [v4.1.7正式回执](docs/qa/PUBLICATION-4.1.7.md) · [v4.1.7发布检查](docs/qa/RELEASE-4.1.7.md) · [v4.1.7配套与历史附件](docs/RELEASE-COMPONENTS.md) · [v3.1.2发布传输状态](docs/qa/DELIVERY-3.1.2.md) · [完整图文使用指南](docs/USER_GUIDE.md) · [网页剪藏安装](browser-extension/README.md) · [v1.0.2实际验收](docs/qa/RELEASE-1.0.2.md) · [v1.0.2发布回执](docs/qa/PUBLICATION-1.0.2.md)

v3.1.2 新增 **本地相似图片检索**：主动生成原图特征索引，SHA完全重复组、可调整pHash/颜色距离与双图人工比较，不自动删除。可选Qwen3.5 4B随完整离线分卷包提供；轻量NSIS不含数GB模型。模型可能出错或截断，只作辅助。[算法与离线包](docs/SIMILARITY.md) · [本版变更](docs/RELEASE-NOTES-3.1.2.md) · [实际验收](docs/qa/RELEASE-3.1.2.md)。

v3.1.0 修复 WebDAV 设置交互：保存回执与状态轮询分离、停止不等待配置锁、凭据错误可见且可修复、编辑中的连接信息不被轮询重置。字段就近提示、按钮条件说明与23幅离线指南图片帮助完成保存→测试→同步。[本版变更](docs/RELEASE-NOTES-3.1.0.md) · [实际验收](docs/qa/RELEASE-3.1.0.md) · [正式发布回执](docs/qa/PUBLICATION-3.1.0.md)。真实坚果云认证仍需在你的账号上验证，未放宽安全条件写入门槛。

v3.0.0 新增 Windows 手动 WebDAV 资源级同步：系统安全凭据、项目范围、强ETag安全探针、可重试队列与逐项冲突确认；schema6保留既有资料并建立明确标注head。设置新增分类导航，指南扩展同步步骤、排错和21张离线图。**坚果云认证兼容性尚未验收；不可靠条件写入会被拒绝。** [本版变更](docs/RELEASE-NOTES-3.0.0.md) · [实际验收](docs/qa/RELEASE-3.0.0.md) · [正式发布回执](docs/qa/PUBLICATION-3.0.0.md)。

v3.0.1 新增 **窗口截图**：鼠标高亮选择、Tab/Shift+Tab切换、Esc取消，使用Windows Graphics Capture读取窗口自身图像，复用复制、PNG保存、入库及标注编辑。受控本地捕获worker限制资源与超时；不会用桌面裁剪冒充无遮挡窗口截图。无数据库迁移或新第三方依赖。[使用与支持范围](docs/WINDOW-CAPTURE.md) · [本版变更](docs/RELEASE-NOTES-3.0.1.md) · [实际验收](docs/qa/RELEASE-3.0.1.md) · [正式发布回执](docs/qa/PUBLICATION-3.0.1.md)。

## 从这里开始

1. **获取程序**：在 Release 下载 Windows x64 EXE 或 NSIS 安装包。需要 WebView2；程序未签名，请核对随发布提供的 SHA256。先保存并退出旧实例。
2. **采集与整理**：截图后点击「存入素材库」，或主动从剪贴板/本地图片导入；用项目、标签、收藏和搜索组织资料。
3. **标注与写作**：保存可编辑标注，再插入 Markdown；文档保存为标准 `.md` 与相对 `assets`，可交给其他 Markdown 软件。

核心资料留在本机，不设ResearchClip账号或遥测；WebDAV仅在用户明确配置并手动开始后传输所选资料，没有自动上传。升级不会清空 SQLite 或原图；备份请保留完整资料目录与外部 Markdown 文件夹。

## 主要工作区

- **采集**：Windows 全屏/区域截图、两组可配置全局快捷键、图片剪贴板与文件拖拽导入。
- **编辑**：无损箭头/矩形/椭圆/文字/画笔/马赛克/模糊/编号，撤销重做、原分辨率PNG、贴图和离线OCR。
- **整理**：SQLite素材库、网格/列表、科研知识卡片、笔记自动保存、跨项目标签、组合搜索、批量操作与双图对比。
- **写作**：动态Markdown分栏、标注合成图插入、模板、大纲、离线LaTeX预览和系统其他应用打开。
- **帮助**：18章任务式说明、114项功能示例、分层目录与正文搜索、31幅离线图示和原尺寸放大查看。

v1.0.5 新增非破坏性标注版本历史：显式保存去重、版本说明、只读预览/对比、恢复追加新版本；schema5升级前备份，原图不重复保存。[本轮验收](docs/qa/RELEASE-1.0.5.md) · [发布核验](docs/qa/PUBLICATION-1.0.5.md)。Release继续附带兼容的 v1.0.2 浏览器插件包，不删除既有发布。

v1.0.4 新增需校对的离线公式图片转LaTeX；可选Python运行时与171 MiB模型由用户主动安装/下载。实际样本存在误识别，详见[模型比较](docs/MATH-OCR.md)与[本轮验收](docs/qa/RELEASE-1.0.4.md) · [发布核验](docs/qa/PUBLICATION-1.0.4.md)。v1.0.3 新增独立OCR全文存储、用户控制批量索引、中文短词/英文组合搜索与安全摘要高亮；数据库以备份及事务升级到schema4。详见[本轮验收](docs/qa/RELEASE-1.0.3.md) · [发布核验](docs/qa/PUBLICATION-1.0.3.md)。v1.0.2 新增独立MV3网页剪藏、受限Native Messaging Host、项目保存和标准Markdown联动；完整安装/重试说明见指南第3节。v1.0.1滚动长截图保持。完整高DPI/多屏、浏览器真实工具栏交互、macOS与第三方应用矩阵仍需专项验证；旧退出/托盘问题与本轮复验分别记录在 [QA总表](docs/qa/TEST_REPORT.md)，不把正式Release标识等同于未运行项目通过。

## WebDAV：先验证，再同步

设置中输入HTTPS目录、专属子目录和本机应用授权密码，先测试安全能力，选择项目、授权已登记Markdown目录，再手动同步。两台设备各自保留SQLite，不上传数据库文件。协议与限制见[资源级协议](docs/sync/PROTOCOL.md)、[schema6迁移](docs/sync/MIGRATION.md)和[指南第13章](docs/USER_GUIDE.md#131-webdav手动双向同步)。

指定测试目录为 `https://dav.jianguoyun.com/dav/ResearchClip/`，不访问Zotero目录。当前环境未获得安全配置的用户凭据，**真实坚果云认证/条件并发验收为BLOCKED**。独立本地双客户端测试不代表该服务支持；不支持强ETag与可靠条件写入时应用拒绝提交，不降低到不安全覆盖。首次用小型测试项目，留意限流和磁盘容量。当前Markdown同步支持应用生成的相对PNG assets，其他本地引用会报错，不静默省略。

## 实现与历史记录

当前版本说明：[3.1.5双向链接与图谱](docs/RELEASE-NOTES-3.1.5.md)。此前 [3.1.4导航与指南](docs/RELEASE-NOTES-3.1.4.md)。此前 [3.1.3文献引用](docs/RELEASE-NOTES-3.1.3.md)。此前 [3.1.2本地检索](docs/RELEASE-NOTES-3.1.2.md)。此前 [3.0.0 WebDAV与工作区](docs/RELEASE-NOTES-3.0.0.md)。此前 [1.0.1滚动长截图](docs/RELEASE-NOTES-1.0.1.md)。此前 [1.0.0](docs/RELEASE-NOTES-1.0.0.md)。此前 [0.2.1项目分类与公式](docs/RELEASE-NOTES-0.2.1.md)、[0.2.0项目/批量](docs/RELEASE-NOTES-0.2.0.md)、[0.1.8导入/对比](docs/RELEASE-NOTES-0.1.8.md)、[0.1.5模板](docs/RELEASE-NOTES-0.1.5.md)、[0.1.4高级标注](docs/RELEASE-NOTES-0.1.4.md)、[0.1.3OCR](docs/RELEASE-NOTES-0.1.3.md)、[0.1.1贴图](docs/RELEASE-NOTES-0.1.1.md)。旧发布内容保留。

存储和既有模块证据：[存储](docs/STORAGE-VALIDATION.md)、[素材库](docs/LIBRARY-VALIDATION.md)、[笔记与搜索](docs/KNOWLEDGE-SEARCH-VALIDATION.md)、[标注与导出](docs/EDITOR-COMPLETION-VALIDATION.md)、[桌面体验](docs/DESKTOP-EXPERIENCE-VALIDATION.md)。历史记录不充当本轮测试结果。

## 运行

技术栈：Tauri 2 / Rust、React / TypeScript / Vite、Tailwind CSS、Zustand、Konva / react-konva。npm 是唯一包管理器；两个锁文件保留。需要 Node >=22.12、Rust MSVC >=1.99、Windows SDK / C++ Build Tools、WebView2；参考 [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)。

```powershell
npm ci
npm run desktop:dev

# 浏览器预览（没有原生捕获能力）
npm run dev

# Windows release，无安装包/签名
npm run desktop:build
# src-tauri/target/release/researchclip.exe

# Windows NSIS 安装包（当前用户安装）
npm run desktop:bundle
# src-tauri/target/release/bundle/nsis/ResearchClip_3.1.5_x64-setup.exe

# 旧release窗口正在运行时，可构建到独立Windows target目录
npm run tauri -- build --no-bundle --target x86_64-pc-windows-msvc
# src-tauri/target/x86_64-pc-windows-msvc/release/researchclip.exe
```

scripts/with-rust.mjs 优先使用项目内 .tools 工具链，否则使用 PATH 中的 Rust，不改系统 PATH。macOS 截图与整机运行未验证。

运行新版本前请保存并退出旧应用；启动第二实例会恢复既有窗口，不重复注册托盘和快捷键。Windows也会阻止替换正在运行的EXE。

安装器和卸载器遇到程序仍运行或文件占用无法确认时会中止，不强制结束应用。默认卸载保留科研资料；卸载界面的“删除应用数据”若由用户明确勾选，会删除应用资料，请谨慎选择。缺少WebView2时默认安装器需要下载运行时；不能承诺离线首次安装。

## 截图与设置

截图完成区和图片编辑器均提供“贴图”：创建独立无边框置顶窗口，编辑器使用包含标注的合成 PNG。拖动移动、滚轮等比缩放、右键复制/另存/调整透明度/关闭，支持多张；工具栏图钉按钮可恢复屏幕外贴图。隐藏到托盘保留贴图，真正退出关闭全部贴图；会话不跨重启恢复。实现、资源限制、实际测试与人工矩阵见[桌面贴图验收](docs/PINS-VALIDATION.md)。

- 选择目标显示器后，全屏捕获或区域拖动；区域层显示捕获开始前冻结的原生画面，不以 CSS 截屏代替。Esc 取消，零面积可重选，终态销毁覆盖层。
- 默认区域截图快捷键 Ctrl+Shift+2、全屏截图快捷键 Ctrl+Shift+1，在设置中分别修改，均捕获主显示器。两组不可使用相同组合（含等价写法）；注册冲突保留旧设置及绑定。Rust 只处理 Pressed，复用单一捕获门控，真正退出时注销两组快捷键；隐藏到托盘时继续有效。
- 旧设置中的区域快捷键保留，缺少全屏字段时补默认值而不立即改写文件。若旧区域配置正好占用 Ctrl+Shift+1，保留区域动作并提示全屏冲突，请在设置中为全屏选择另一组合。
- 设置菜单集中管理深色/浅色/跟随系统主题、字体、基础字号、默认素材视图、双快捷键、截图默认保存目录及版本。桌面端写入 app_config_dir/settings.json（独立 schemaVersion=1）；未知版本/损坏文件拒绝覆盖。浏览器外观只在会话内有效。
- 默认标注颜色和线宽（1–32原图像素）点击“保存标注默认值”保存，用于之后创建的对象；不会改写已存在的标注。主题、快捷键及关闭行为等设置直接保存，失败保持原配置并提示。
- 文件夹通过系统目录对话框选择。默认保存目录决定“保存 PNG”的系统另存为起始位置，不移动旧截图、不自动保存。目录不可用时设置页和保存操作明确报错并保留配置，请重新选择；权限/空间问题在实际写入时报错。
- 截图完成后提供复制图片、保存 PNG、打开编辑器、取消。复制使用官方插件的 RGBA 图片 API；保存是原始 PNG，经系统对话框授权，同目录暂存、sync 和禁止覆盖提交。同名文件即使经过系统提示也不会被覆盖，请选择新名称。
- 全屏和区域均只缓存一张有大小上限的 PNG；前端用不透明 captureId 读取。替换/取消释放资源，不开放任意路径读写。以前的临时 PNG 不被删除。
- 不上传屏幕内容，不在普通日志打印像素、图片或完整资料路径。SQLite现为schema v3，保留旧app_metadata；主界面通过受限业务命令读取真实素材。

使用官方 [global-shortcut](https://docs.rs/tauri-plugin-global-shortcut/2.4.1/tauri_plugin_global_shortcut/)、[clipboard-manager](https://docs.rs/tauri-plugin-clipboard-manager/2.4.2/tauri_plugin_clipboard_manager/struct.Clipboard.html) 与 [dialog](https://docs.rs/tauri-plugin-dialog/2.8.2/tauri_plugin_dialog/struct.FileDialogBuilder.html)，均为 MIT/Apache-2.0。插件仅由受限 Rust 用例调用，前端没有通用文件系统、剪贴板读取或快捷键注册权限。

## 离线 OCR 文字识别

截图完成区或图片编辑器点击“识别文字”，可识别整张图片，或在图片内拖动框选局部区域。编辑器复用已标注合成图；原图和素材记录不改动。结果可手动修改、复制到其他应用，或明确点击“插入到 Markdown”放入已打开文档的光标/选择处；图文编排模式在末尾新增文本块。请先打开 Markdown 再识别；关闭面板前复制或插入，结果不会自动写入笔记。

使用 Windows 原生本地引擎，不随应用附带/自动下载模型，不上传图片或识别文本。下拉框只显示实际已安装的 OCR 语言；缺少组件时请在 Windows 语言选项中安装后重开面板。当前真机仅有简体中文包，英文/混合关键词测试通过但仍有错字；独立英文引擎未验证。低分辨率小字、特殊符号和复杂公式误差明显，**不能当作 LaTeX 公式识别器**，务必校对再插入。实际测试、限制与人工清单见 [OCR 验收](docs/qa/RELEASE-0.1.3.md)。

## 托盘与退出

托盘菜单可以开始区域截图、全屏截图、打开主窗口、打开设置或退出。单击图标恢复同一个主窗口；最小化仍是普通系统最小化。采用Tauri内置托盘和官方single-instance插件，应用内只创建一个固定ID托盘。

默认点击窗口关闭按钮退出；在设置中启用“关闭窗口时隐藏到托盘”后，关闭只隐藏窗口、快捷键保留。托盘不可用时不会把窗口隐藏到无法恢复的状态。托盘“退出 ResearchClip”始终请求真正退出。

退出先等待卡片笔记保存；失败会取消退出并保留草稿。未保存标注或绘制中的动作会要求返回保存或明确放弃。PNG导出不等于保存可编辑标注。退出等待中的截图/文件对话框有界等待，忙时提示重试；成功时关闭WebView、移除托盘并释放快捷键和单实例资源。未存入素材库的截图预览仍是临时数据，请先显式保存；强制结束进程/断电不承诺草稿恢复。

## 本地素材库

- 截图后选择“存入素材库”；编辑器选择“保存到素材库”。单击素材缩略图或标题打开科研知识卡片，“编辑标注”仍直接进入图片编辑器。核心资料不写浏览器localStorage；浏览器预览没有持久化能力。
- 网格与列表均显示缩略图、标题、创建时间和原图尺寸。按创建时间最新/最早优先排序；标题、收藏和标注更新不会改变创建顺序。同一时间以ID稳定排序。
- 每页24条，用上一页/下一页切换并显示总数量；切换排序/收藏范围回到第一页，删除最后一页的唯一素材自动回退。翻页替换当前列表并释放旧缩略图URL，不累积加载原图。缩略图读取/解码失败显示独立占位，可重试；缺原图保留记录并在打开时报告错误。
- 使用既有rusqlite/bundled SQLite，不添加通用SQL插件权限。默认目录为Tauri app_data_dir（Windows通常为 `%APPDATA%/io.researchclip.desktop`），内有researchclip.sqlite3、originals、thumbnails、backups和workspace.lock。设置里的保存目录只影响外部另存，不搬迁受管资料。
- 图片保存唯一文件名和相对引用；原图字节不变，缩略图最长边512px、保留透明度且不放大，基于原图而非合成标注。标注仍为schemaVersion1 JSON，重启恢复可编辑对象。
- 可以改标题、收藏与确认删除。删除会移除记录、笔记、标签关联和受管图片，标签本身及外部导出不删除；没有回收站。清理失败保留任务，界面提示并支持重试，重启也会继续。
- 升级前创建并验证一致SQLite备份，再事务迁移；未知未来版本/损坏库拒绝写入，不重置资料。多个应用实例同时访问同一工作区时，后一个实例提示工作区被占用。
- 图片标注仍需手动保存。卡片标题、Markdown正文、来源、收藏和标签在停止输入约700ms后自动保存，也可点击“保存卡片”或Ctrl+S。同一卡片写入串行，快速切换保留独立草稿；失败显示提示和全局待保存入口，不清空文本。退出前请确认待保存内容已保存。
- 其他操作修改同一资料时会报告revision冲突并保留草稿。自己的卡片事务不改变标注JSON，可在已知revision相同时同步编辑器绑定；未知外部变更不自动覆盖。移动资料或备份请关闭应用并保留数据库和图片目录，不能只复制正在运行的主DB文件。

## 科研知识卡片与搜索

- 卡片显示原图、标题、Markdown纯文本编辑区、来源URL、多个标签、收藏和创建/修改时间。已有多条笔记时，默认编辑最早的(createdAt,id)笔记，其他笔记保留且纳入搜索。本轮不渲染Markdown HTML或脚本。
- 正文最多1 MiB，标题200字符，每卡100标签；来源仅有效http/https，长度与编码后长度均不超过2048字节，不接受账号密码。打开来源先保存当前卡片，再通过受限Rust命令调用 [官方Opener](https://v2.tauri.app/plugin/opener/)，前端没有通用打开文件/URL权限。
- 卡片标签输入支持已有标签候选和新建；移除芯片只解除关联。标签页可创建、查看对应素材和确认全局删除标签，截图、笔记和图片文件保留。
- 顶部快捷搜索约300ms防抖，中文按原文子串、ASCII英文不区分大小写；同时匹配标题或任意笔记正文。Ctrl+K打开搜索弹窗，可跳转卡片或查看全部结果。
- 标签筛选采用AND（同时包含全部选中标签），与关键词、收藏状态、创建时间排序组合。收藏页导航也是额外已收藏条件；“清除全部筛选”回到全部素材。特殊字符%、_、反斜杠作为字面文本处理，空/全空白查询表示不限制关键词。
- SQLite参数化LIKE适合当前MVP规模，无独立搜索服务器；仍按24条分页，搜索只读摘要并加载当前页缩略图。关键词最多200字符、筛选标签最多100个。1000卡/大量长笔记的p95尚未验收，后续根据测量考虑版本化FTS迁移。
- 会话缓存最多20张卡片，满时只回收干净会话，未保存草稿不被淘汰；全部为未完成卡片时提示先保存。草稿仅在当前进程中保留，不承诺崩溃/断电或强制退出时恢复。

## 数学公式与外部 Markdown 应用

右侧文档源码支持 `$V(x)=x^TPx$`、独立 `$$...$$`、矩阵和多行公式，默认下方实时预览，可切换完整预览。公式错误显示“公式错误”及原始内容；保存/重开保留 LaTeX 文本，渲染不会把公式改成图片。KaTeX 和字体随应用离线打包；不执行文档 HTML、公式链接或外部资源命令。每式8192字符、每次预览256式/131072公式字符、宏展开500次；超限提示，原文保留。

面板“使用其他应用打开”先保存当前修改，再调用 Windows 系统选择器选择已安装的 Obsidian、Typora、VS Code 等；取消保存不会启动其他应用。外部修改后 ResearchClip 拒绝覆盖，请重新打开或另存。不同编辑器的公式语法扩展可能不同；移动文件请连同 assets 一起移动。本轮没有将第三方应用逐个实测结果标记为通过。

## Markdown 导出

### Markdown 模板与大纲（0.1.5）

顶部“模板”或右侧面板“新建模板文档”选择空白、论文精读、数学定理、科研实验或自定义模板；可设置新文件名。新草稿在内存中编辑，Ctrl+S 仍使用系统保存对话框保存普通 UTF-8 `.md` 和相对 `assets`，原有图文编排与动态分栏保持不变。

右侧“保存为模板”复制当前正文及受管图片到 `app_config_dir/templates-v1/<id>/名称.md` 和 `assets`，不修改或保存当前文件；模板最多100个、名称80字符，沿用每份2 MiB正文/32图/128 MiB资源限制。模板选择框可重命名和两次确认删除；同名不覆盖。删除只将独立目录改为 `deleted-<id>`，不再出现在列表，保留文件以便关闭应用后手动恢复/归档；不会自动清空这些目录。

“显示大纲”识别 CommonMark H1–H6（含 Setext），忽略代码块与不可信 HTML。点击在源文中定位光标并滚动，预览中滚动到对应标题，编辑后自动更新；图文模式跳转到预览，不转换原有内容块。没有文档时不显示 Markdown 面板或大纲。实际结果和人工步骤见 [0.1.5 验收](docs/qa/RELEASE-0.1.5.md)。

### Markdown 图文文档（当前工作区新增）

顶部“新建 Markdown”创建图文编排文档，自动在右侧分栏打开，左侧原页面保留。选择素材图片后，使用编辑器相同的Konva合成逻辑把已保存标注烘焙进独立PNG，按勾选顺序添加，每张图片下方紧跟自己的注释。已有笔记作为添加时快照，之后修改文档不回写素材或笔记；未保存的图片标注须先保存到素材库。

可编辑标题和注释，用上移/下移调整整个图文块，在任意块后插入Markdown文本，或删除内容块。“实时预览”与编辑区在同一页面上下排列；支持CommonMark标题、粗斜体、列表和代码。LaTeX源码保留，使用离线KaTeX渲染行内 `$...$` 和独立 `$$...$$` 公式；预览不执行HTML、不加载远程图像或打开链接。

“保存 Markdown”或Ctrl+S首次选择.md位置，之后更新当前已打开文件；“另存为 Markdown”或Ctrl+Shift+S选择新文件。新目标拒绝同名覆盖；继续保存前检测外部修改，冲突时保留草稿并提示另存。PNG放在同目录assets/，稳定唯一命名、相对引用且不覆盖既有不同内容。移动时请保留整个文档文件夹。

“打开 Markdown”恢复本应用保存的图文块顺序；普通外部.md作为文本块导入，不自动收集外部附件。为无损保留ID/关联，生成的标准Markdown含不可见的版本/内容块注释；第三方软件可直接阅读，但若外部修改破坏结构或长度标记，ResearchClip会拒绝结构化重开并保留现有草稿，请先在外部编辑器检查。关闭/新建/打开替换及退出应用时，未保存文档均提供保存、放弃、取消。

当前只保留一个文档会话，导航切换不丢草稿；不承诺强制结束进程后的草稿恢复。每份最多100块/32张图、2 MiB Markdown及128 MiB资源；从文档移除图片不删除素材库或磁盘旧assets。失败可能留下未引用资源，不自动清理用户文件。本功能不改SQLite schema；第一阶段历史检查见 [图文文档验收](docs/qa/MARKDOWN-DOCUMENT-STAGE1.md)。

```powershell
npm run tauri -- build --debug --no-bundle
npm run test:smoke:document:native -- stage1-new-id
```

### Markdown 动态分栏（第二阶段）

- 顶部“新建空白 Markdown”创建内存中的 untitled.md 并立即编辑；“打开 Markdown”用系统对话框选择已有文件。无文档时没有右侧面板和分隔线；关闭后恢复原工作区宽度。
- 左侧素材库/图片编辑器继续工作，右侧支持标准Markdown源码、中文/英文/LaTeX文本、撤销重做和预览。Ctrl+S在Markdown面板内保存；Ctrl+Shift+S另存。首次选路径，之后安全更新已打开文件；另存拒绝同名覆盖。
- 文件名区域支持右键→“重命名文件”（也可左键打开菜单）。新文档自动命名，可在首次保存前自定义名称；缺少扩展名时补.md。已保存文件仅在当前目录改名，正文标题和assets不变，后续Ctrl+S写入新文件；同名、非法名称、外部改动时明确报错。详见 [文件命名验收](docs/qa/MARKDOWN-RENAME.md)。
- 拖动分隔线调整宽度，也可用方向键或Home恢复50%；比例写入现有本地settings.json，重启恢复。窄窗口动态缩小显示最小宽度，不改变原图坐标。
- 文档打开后，素材卡和图片编辑器出现“插入到 Markdown”。素材卡使用已保存标注及笔记快照；编辑器按钮可合成当前未保存标注，不改原图或标注保存点。插入到右侧上次光标/选区处，保存时提交稳定相对assets引用。
- “新建 Markdown”仍提供第一阶段图文编排；可显式“编辑 Markdown 源文”转为普通全文编辑，图片和注释的当前顺序保留。原有结构化文件继续按原格式重开。
- 关闭修改过的文档提供保存/放弃/取消；保存失败保持面板及草稿。预览不执行HTML/脚本，不下载远程图片；公式仅编辑源码。普通外部Markdown的任意附件不自动收集。

实际结果、完整文件清单、失败记录及Windows人工步骤见 [动态分栏验收](docs/qa/MARKDOWN-SPLIT-STAGE2.md)。原生回归命令（隔离测试资料，非真实屏幕捕获）：

```powershell
npm run test:smoke:split:native
```

### 科研卡片原图导出

在知识卡片中点击“导出 Markdown”，或在素材库勾选多张卡片后点击“导出所选 Markdown”。支持跨页选择，最多100张；离开素材库页面后清空选择。导出前自动保存所选卡片的未保存笔记，保存失败会停止导出并保留草稿。

通过系统文件夹对话框选择位置，每次创建新的ResearchClip-Export文件夹，重名时使用-001、-002等后缀。内含notes.md及assets/；标题、Markdown原文、来源、标签和UTC创建时间一起导出，图片采用相对正斜杠路径。移动到Obsidian仓库或其他位置时，请保留整个文件夹。详情见 [Markdown导出验收](docs/MARKDOWN-EXPORT-VALIDATION.md)。

本次导出卡片显示的原图和主笔记，不合成编辑器标注，不收集用户笔记中另外引用的外部附件；需要带标注图片时可使用上述图文文档、分栏插入或编辑器PNG导出。内部图片绝对路径不会自动写入Markdown；用户自行写入正文的内容按原文保留。

原图缺失、损坏或卡片已删除时跳过对应卡片，成功部分仍可使用；界面及export-report.txt列出问题。全部失败时不生成空导出。目录提交失败时可能保留EXPORT-INCOMPLETE.txt，需检查权限和空间后重新导出；不覆盖既有文件，不修改资料库。每批正文16 MiB、原图合计512 MiB上限，暂存和提交期间需要额外磁盘空间。

原生自动化命令如下。第二条会先构建必要的测试辅助程序；使用独立artifacts工作区，不向真实资料库创建样例。

```powershell
npm run tauri -- build --debug --no-bundle
npm run test:smoke:markdown:native -- export-acceptance-01
```

## 图片编辑器

截图完成后点击“打开编辑器”，进入独立页面。图片有各自的视口、选择和标注；最多 8 个会话、128 MiB 原图 Blob，不自动淘汰草稿。标签页切换保留当前会话状态。退出前选择“另存可编辑项目”，重启后从截图页的“图片编辑器 / 打开项目”入口恢复。

- 选择：单击选中、拖动、Delete 删除；对象条也可选择。
- 矩形、椭圆：拖动创建，选中后用手柄调整大小。
- 箭头：创建起终点，选中后拖动端点；颜色、线宽可调。
- 自由画笔：连续向量路径，颜色、线宽可调。
- 文字：点击插入，多行文字，双击编辑，字号/颜色可调。
- 滚轮或按钮缩放、适应窗口、平移工具/空格/中键平移。V/H/R/E/A/P/T 切换工具，Esc 取消当前绘制/拖动。
- 标注唯一 ID，应用数据可序列化为 JSON。坐标、线宽、字号使用原图像素；视图缩放和平移不改写文档，Konva 节点不作为存储格式。
- 原始 Blob 与标注分离，图片加载/尺寸错误明确提示。500 对象、单笔 5000 点、每图总 50000 点的上限避免无界状态。

- Ctrl+Z 撤销，Ctrl+Y / Ctrl+Shift+Z 重做，亦有工具栏按钮。每张图片保留最多 100 步，标注快照编码预算 8 MiB；大文档会先丢弃最老记录。新编辑清理重做；选择、平移、缩放不进历史。
- “另存可编辑项目”通过系统对话框写入 *.researchclip.json 与同目录唯一 original-*.png。JSON 只保存版本、原图引用和可编辑对象；原始 PNG 不被编辑/导出覆盖。移动资料时须一起移动这两个文件。
- “打开项目”校验项目版本和标注版本（当前均为 1）、原图引用、尺寸与 PNG；不支持的版本、损坏或缺失原图显示错误并保留当前会话。没有自动保存或数据库项目记录。
- “导出标注 PNG”和“复制合成图片”使用独立 Konva 画布，原图分辨率、透明背景、当前对象顺序及本机字体；不传入视口、选中框或手柄。不必先保存项目，但此操作不会保存可编辑数据。
- 保存/导出均拒绝覆盖已有文件，请选择新的名称。项目 JSON 提交失败可能保留本次原图副本，并明确提示；不删除用户文件。

截图页的“保存 PNG / 复制图片”仍处理原始截图；编辑器提供带标注的独立导出操作。历史不写入项目，重新打开后从已保存内容开始新的历史。

## 检查

```powershell
npm run check
npm run test:rust
npm run format:rust
npm run check:rust
npm run check:docs
npm run test:smoke -- --run-id local-ui-1
npm run test:smoke:editor -- local-editor-1

# 隔离设置的原生工作流：先编译 debug app 和测试输入辅助程序
npm run tauri -- build --debug --no-bundle
node scripts/with-rust.mjs cargo build --manifest-path src-tauri/Cargo.toml --locked --example workflow_input
npm run test:smoke:workflow -- local-native-1
npm run test:smoke:workflow -- local-hotkeys-1 --settings-only
npm run test:smoke:editor:native -- local-editor-native-1
npm run test:smoke:storage:native -- local-storage-native-1
npm run test:smoke:library:native -- local-library-native-1
node scripts/with-rust.mjs cargo build --manifest-path src-tauri/Cargo.toml --locked --example knowledge_fault
npm run test:smoke:knowledge:native -- local-knowledge-native-1
npm run test:smoke:experience:native -- local-desktop-native-1

# 原生 GDI 探针，仅输出尺寸和资源计数
node scripts/with-rust.mjs cargo run --manifest-path src-tauri/Cargo.toml --locked --example capture_probe -- 1
```

每次使用新报告 ID，保留 artifacts 内的历史证据。编辑器冒烟使用明确标记的测试图片，只验证编辑器，不证明屏幕捕获；原生测试必须在真实 Windows 桌面运行。调试用的固定图片入口必须同时设置两个测试环境标志，仅在 debug 构建存在，普通启动和 release 不生成图片。

## 当前目录

```text
src/
  app/                       # 主界面组合、导航、偏好应用、编辑器交接
  features/
    capture/                 # 选择层、完成操作、临时预览状态
    editor/                  # types、geometry、model、history、serialization、export、api、Canvas 与测试
    settings/                # 设置、系统主题监听与持久化状态
    desktop/                 # 桌面事件监听、退出协调与未保存标注提示
    library/                 # 卡片网格、组合筛选、快捷搜索、分页store与测试
    notes/                   # 科研知识卡片、防抖保存、待保存入口与测试
    export/                  # 单卡/批量Markdown导出、草稿保存协调及结果提示
    tags/ home/              # 标签管理与欢迎引导
  shared/api/ types/ ui/     # 有类型的 IPC、DTO、通用组件
src-tauri/
  src/commands/              # 截图、设置、编辑器与受限素材库入口
  src/services/              # screen_capture、region_capture、workflow、settings、editor、desktop
  src/capture/               # Windows GDI、物理坐标与裁剪
  src/storage/ migrations/   # SQLite schema6、素材/卡片/同步事务、组合搜索、文件一致性与迁移
  capabilities/              # main / capture-overlay 分离
  examples/                  # 仅供测试的原生辅助程序
scripts/ docs/                # 检查、冒烟、架构和验收
```

.gitignore 排除 .tools、依赖、构建、测试资料、生成权限和本地日志。未删除用户数据或重置数据库。

## 文档与边界

[AGENTS](AGENTS.md)、[PRD](docs/PRD.md)、[ARCHITECTURE](docs/ARCHITECTURE.md)、[ROADMAP](docs/ROADMAP.md) 定义工程规则与阶段目标。[区域历史验收](docs/REGION-CAPTURE-VALIDATION.md)、[全屏历史验收](docs/SCREEN-CAPTURE-VALIDATION.md) 保留以前的实际记录。

下一步建议复验大图库与长笔记的性能、实际系统主题切换及真实科研资料的日常管理。物理快捷键、画图/Word粘贴、剪贴板竞争、只读/磁盘满、多显示器与高DPI等既有硬件场景，以及macOS仍需独立验收。


## 高级图片标注

编辑器新增“马赛克”“局部模糊”“编号”。前两者拖动矩形创建，选择对象后移动/调整手柄及强度；编号点击创建，可改颜色、大小和数字，删除不重排其他编号。所有数据按原图像素保存，使用原有撤销/重做、可编辑项目和素材库。滤镜真实应用于导出 PNG，并保留透明度；原图不覆盖。滤镜在原图层处理，其他矢量标注在上方。详细范围、资源上限及实际验收见 [高级标注验收](docs/qa/RELEASE-0.1.4.md)。


## 科研项目与批量整理（v0.2.0）

素材库上方创建/重命名项目，切换全部素材、未分类或具体项目，并显示图片/文档数量。每图最多一个项目，标签跨项目共享；移动只写SQLite关系，不改变原图、ID、标注、笔记或来源。项目仅允许确认删除空项目，不提供隐式删除项目内数据。

Ctrl点击图片/标题多选，Shift选择本页范围，可跨页勾选；全选当前筛选结果包含所有分页（上限5000，超限明确拒绝）。批量添加标签是合并，支持移动、收藏/取消收藏、Markdown导出（每批100）和数量确认删除。批量写入使用带版本校验的事务；删除清理任务先持久提交、失败保留供恢复。取消删除不会删除图片。

Markdown仍为标准外部.md与assets；通过应用打开/保存后登记受限文件引用，可在面板或文档列表关联项目并重开。未保存草稿不登记；应用内重命名保持同一文档ID，另存为独立文档。应用外移动文件需重新打开登记；不扫描磁盘，也不将正文复制进数据库。

SQLite v2→v3升级会先验证并备份，新增projects、screenshot_projects和markdown_documents表及索引；不重写既有迁移、不搬移资料。旧版程序会拒绝写入v3，请保留升级备份并用新版本打开，勿删除数据库解决版本问题。完整操作和数据安全见[指南第5节](docs/USER_GUIDE.md#5-科研项目与批量整理)。


v1.0.0 已发布为正式 Latest：[附件与匿名下载校验回执](docs/qa/PUBLICATION-1.0.0.md)。

后续分阶段开发遵守[ResearchClip Pro规范](docs/PRO-DEVELOPMENT.md)。版本历史已在v1.0.5实现；WebDAV仍处于设计阶段，不开发PDF导出或独立备份恢复系统。[规范记录验收](docs/qa/PRO-CONSTRAINTS.md)。

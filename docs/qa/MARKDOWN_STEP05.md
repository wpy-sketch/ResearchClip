# Step05 Markdown保真编辑验收

目标：增量增加源码、近所见即所得、实时/只读预览、同步滚动、六种斜杠命令和图文整体拖动。先行[选型记录](../architecture/EDITOR_STEP05_DECISION.md)，基点aa56f83。无新增运行依赖、权限、数据库迁移或文件服务；SQLite12。未实现完整富文本引擎，CodeMirror/Milkdown/Tiptap均未安装。

## 实现与修改范围

projection.ts只读mdast位置，保留原片段；applyDomText把DOM规范换行映射回原字符串，仅替换改动区间。VisualEditor以非活动渲染、活动块原文输入实现近WYSIWYG；未知容器/超预算转受保护原文，不丢语法。模式切换仅修改视图状态，原文、caret/past/future、dirty继续由既有DocumentStore管理。

text-target与FormattingToolbar桥接全文、可视片段和旧caption选区；SlashMenu只接受六个固定插入动作，图片复用LibraryPicker，取消不改原文。DocumentView按原块ID拖动整个图片/caption；源文图片组包含紧随的连续说明段。拖动使用原撤销快照。同步滚动双向按比例，ResizeObserver和图片加载后测量，非逐行精确对齐。

MarkdownPanel保留Ctrl+S和关闭确认。已登记且未修改的文档保存不重写，避免旧格式归一化。Rust既有names增加标题后的路径边界识别；原严格asset_name及32图限制不变。未改原图、已发布迁移或用户目录。

主要修改：src/features/document/{MarkdownPanel,DocumentView,FormattingToolbar,VisualEditor,SlashMenu,store,projection,text-target,use-scroll-sync,visual-editor.css}；相应单测；src-tauri/src/services/markdown_document.rs；guide、混合练习稿与原生图；发布运输脚本仅扩展414且受已授权精确草稿/摘要约束。

## 本轮实际结果

Windows、Node22.22.2、Edge/WebView2；隔离artifacts目录。npm test：557/70文件通过，退出0。首次完整npm run check在标题图片补测前为556项，退出0；最终npm run check也已557项/70文件通过，类型、lint、构建退出0。Rust fmt/test/clippy退出0，237通过、7忽略。

node scripts/markdown-workspace-smoke.mjs v414-drag-pointer：7组通过，退出0。实际12次模式切换、CRLF混合稿、选区/撤销、HTML5鼠标拖动、扩展保护、六命令及取消、代码/合成IME防护、双向滚动和30个深浅主题/宽度/CSS缩放组合。合成IME不代表物理输入法。

node scripts/markdown-workspace-native-smoke.mjs v414-first：7组通过，退出0。实际Windows debug构建后执行；旧v11隔离库启动迁移到12，无警告。八次模式切换和Ctrl+S原文件字节一致；带标题受管PNG实际解码；可视选区编辑/撤销保存仅改变指定字节；六命令/素材库插入/取消/撤销；源码及旧图文稿实际HTML5指针拖动；关闭取消；双向比例滚动；真实进程退出重启后重读，PNG哈希不变。

前期失败保留：模式组件重复key导致工具栏重复，已用唯一key并加入每次切换数量断言修复。最初Playwright dragTo未产生dragstart，改为足够视口内分步实际鼠标拖动，记录dragstart/drop并通过；不是降低业务断言。文档检查原先不能识别图片title，已修正解析并保留含title的复杂样例。

## 未验证与人工验收

物理中文输入法组字、多显示器100/125/150/200%DPI、跨屏拖动、真实科研大稿、macOS、Zotero和HarmonyOS未执行。此前额外workflow-smoke物理截图因前台归属保护失败，不能算本轮通过。浏览器CSS缩放不能替代Windows系统DPI。

人工请复制练习稿和assets：反复切四视图后比较文件；选中文字加粗/撤销并检查公式；逐个斜杠插入与取消；拖动含两段说明的图片并撤销；旧图文稿核对ID/关联；改变窗口/字号与物理IME；Ctrl+S、取消关闭、退出重开检查内容。未知容器、大于512Ki字符或超过300块限制拖动，源码可继续编辑；渲染不执行HTML、不加载任意远程图。窗口中的块级原文输入不是完整富文本。

构建、旧功能回归、安装及最终发布结果见[发布检查](RELEASE-4.1.4.md)。本轮停止于用户指定Step05；后续Step06已获另行授权，须待v4.1.4正式发布完成后开始。

旧功能回归本轮实跑：工具栏10组、命令8组、工作区7组、资源兼容6组，均退出0；含Win32最大化/最小化/恢复、Konva实际标注保存及旧引用导航。连同本轮新原生7组共38组。指南浏览器14组、34图真实解码通过。证据摘要见[本轮检查](evidence/v414-editor-checks.json)。一条批量命令曾拼错command-native脚本名，Node未执行该用例；随后使用实际commands-native和resources-native脚本完成，日志保留。

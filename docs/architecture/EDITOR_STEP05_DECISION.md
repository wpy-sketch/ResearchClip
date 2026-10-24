# Step05 技术选型：源码保真的近所见即所得

日期2026-10-24。本记录先于Step05业务实现写入。审查基点aa56f83，应用4.1.3、SQLite12；已检查MarkdownPanel、DocumentView、DocumentStore、FormattingToolbar、MarkdownPreview、model与WorkspaceManager。原保存、assets、图文块、独立会话和脏确认可复用；现有textarea不是通用富文本模型，但无需为本次视图交互更换权威存储。

## 候选比较与官方核实

| 方案 | 核实版本/许可 | 优点 | 对本项目保真的影响 | 本轮决定 |
| --- | --- | --- | --- | --- |
| CodeMirror6 | @codemirror/view6.43.15，MIT | 文本/选区/事务与可组合视图，适合源码及装饰式即时预览 | 不要求AST重打印；接入仍需解决CRLF/混合换行映射、现有撤销和工具栏桥接、Windows IME | 三者中首选的后续源码引擎；本轮不安装、不声称已集成 |
| Milkdown | @milkdown/core7.22.3，MIT | Markdown导向的ProseMirror/remark插件体系，成熟富文本交互 | 解析/序列化需要覆盖数学、GFM和每种私有/未知语法；默认模型不能作为原始字符保真的证明 | 不作为本轮权威模型 |
| Tiptap Markdown | @tiptap/markdown3.32.0，MIT | ProseMirror生态、表格、拖动和菜单扩展 | 官方Markdown仍标beta，Marked→JSON→Markdown；官方列出注释和表格单元格限制，未知扩展需自定义tokenizer/renderer | 不采用有损整体往返 |

版本/许可来自本轮实际npm view；未安装候选，不把文档阅读称为平台实验。CodeMirror网站抓取403、Milkdown架构页不可访问，已改查官方仓库；不捏造不可读页面的内容。

官方依据：[CodeMirror开发仓库](https://github.com/codemirror/dev)、[view组件](https://github.com/codemirror/view)、[Milkdown](https://github.com/Milkdown/milkdown)、[Tiptap](https://github.com/ueberdosis/tiptap)、[Tiptap Markdown架构和限制](https://tiptap.dev/docs/editor/markdown)。选型是基于源码和官方资料的工程判断，非三个候选均完成Windows集成的声明。

## 采用的增量方案

采用现有ReactMarkdown/GFM/KaTeX渲染与textarea原文编辑，增加源码投影视图：非活动块渲染，点击当前块编辑它的原文，保留整篇源码为唯一权威。明确称为“近所见即所得/块级即时编辑”，不是完整富文本编辑器。满足本轮允许的近WYSIWYG目标；不引入ProseMirror、第二套正文或撤销服务。

投影只读取既有mdast位置，所有块携带原文区间，展示和模式切换不调用AST序列化。局部编辑仅替换原区间；拖动按原文片段重排，保留未编辑片段、间隔、公式、表格、引用定义和相对路径。复杂未知扩展不能安全分块时明确保留原文并限制结构拖动，不能静默删语法。投影区间是临时视图坐标，不是新的持久资源ID。

旧ResearchClip图文稿继续使用原块ID、图片/注释和保存格式。其“源码”视图为原有分块原文编辑；视图切换不自动调用toSource、不抹掉私有元数据。显式转为普通Markdown仍保留旧入口并说明这是文档变更。图片与整个caption作为原图文块一起拖动；普通源码按明确标识的图文组移动。

源码、近所见即所得、实时预览和只读预览只保存视图状态；原caret/past/future、Ctrl+S、pending关闭确认继续权威。斜杠菜单作用于当前选区，只接受封闭的heading/table/image/math/code/quote；图片继续使用LibraryPicker，取消不改原文。同步滚动使用双向滚动比例/位置映射并防反馈，图片加载/布局变化重新测量；不声称物理多DPI已经验证。

## 数据、依赖与验收门槛

无数据库迁移，无新文件服务、权限、网络请求或运行依赖。继续使用2MiB文字/32图/既有历史预算；原图不变，编辑稿显式保存。任意外部Markdown只作为数据，不运行HTML、脚本或斜杠文字之外的系统命令。

先以复杂混合稿验证投影还原、切换无变化、数学/表格/未知扩展/CRLF与相对路径、局部编辑和图文整体拖动；测试撤销/重做、过时拖放拒绝、IME菜单抑制和滚动反馈。再执行npm run check、相关Windows构建/原生回归和安装验证。失败回退源码视图，原文和原存储不变。未执行结果必须明确记录，完成后停止，不进入其他阶段。

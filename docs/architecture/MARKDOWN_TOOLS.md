# v4.1.3：现有 Markdown 编辑器增量工具栏

MarkdownPanel仍属于原有EditorSession，原DocumentStore、图文块模型、Rust DocumentFiles和WorkspaceManager继续权威读写。没有新文件服务、第二套Markdown正文、分栏系统、资源ID或数据库迁移；SQLite保持12，不需要操作真实数据库。

## 组件和数据流

MarkdownPanel → FormattingToolbar → formatting纯函数 → 该面板DocumentSessionContext中的原editSource/editBlock → 原past/future → 原saveDocumentFile。DocumentView仅标记可编辑字段与选区，不改变图文块序列或元数据。图片按钮使用原LibraryPicker/addImages，原合成与assets保存流程保持。

固定工具栏位于面板顶部，横向滚动；浮动栏在存在文本选区时出现并锚定编辑面板顶部。DOM目标只允许所属面板的data-markdown-target textarea；执行前复核目标仍挂载、字段ID及当前值，防止切换/卸载后写入过时目标。链接输入和查找字段不成为正文目标。预览和文件忙碌期间禁用编辑操作。

纯函数处理UTF-16选区、CRLF、中文、多行行前标记和字面查找替换。选区末尾恰好在下一行起点时排除下一行。Unicode大小写折叠保持原始偏移；不以正则解释查找或替换文本。格式操作与全部替换分别进入一次既有撤销记录；历史沿用100条/8MiB预算，新增blockId仅定位图文模式的历史光标。图文模式的结构增删/移动/标题修改会清空文字历史，图片增删仍按原流程释放资源，防止旧注释快照恢复已释放的图片。

恢复焦点和选区使用requestAnimationFrame与既有同宽文本镜像，先滚动目标位置再恢复范围；卸载时取消帧。组合输入开始/结束及KeyboardEvent.isComposing/keyCode229阻止格式快捷键。面板内Ctrl+B/I/K/F/H/Z/Y明确处理，截图组合/全局命令面板不被占用；App全局Ctrl+K先检查已处理事件和Markdown范围。监听在每次effect清理时释放。

## 安全预览与依赖

复用react-markdown10.1.0、remark-math、离线KaTeX和原有受管图片/链接白名单。新增remark-gfm4.0.1（MIT）用于删除线、表格和只读任务复选框；自有remarkHighlight仅将文本节点的成对==标记转换为mark，不执行HTML或处理代码/公式节点。外部图片不自动载入，外部链接沿用原安全策略。

官方兼容/API依据：[remark-gfm](https://github.com/remarkjs/remark-gfm)、[react-markdown](https://github.com/remarkjs/react-markdown)。新增前核实npm4.0.1元数据、ESM/Node16+及unified11系列，与本项目Node22/现有渲染管线一致。替代手写GFM解析会重复成熟语法实现；没有更换编辑器或引入完整IDE。锁文件其他既有包版本不升级。18个新增包的完整许可见[NOTICES](../licenses/remark-gfm-NOTICES.txt)，同时进入Windows安装包和便携包。

## 兼容、边界与回退

格式栏不是完整WYSIWYG解析器；复杂嵌套的当前格式状态采用明确标记范围，最终以源码和预览为准。高亮是扩展语法，其他Markdown程序可能显示原标记。图文模式查找替换仅当前字段，图片添加沿用既有图文块顺序；源码模式操作全文选区。未打开Markdown仍不创建面板。

失败时原文留在会话中并显示原服务错误；未知文件/数据库版本仍由原服务拒绝。回退可撤回工具栏前端与渲染插件提交，.md与assets保持普通文件和原格式；不得回滚真实数据库或删除资料。未保存草稿不能通过降级程序恢复，先显式保存并退出。测试与人工条件见[验收](../qa/MARKDOWN_TOOLS_4.1.3.md)。

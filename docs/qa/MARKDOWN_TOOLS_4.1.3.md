# Markdown 工具栏验收记录

本轮目标：现有Markdown编辑器增加固定/选区浮动工具栏，沿用文件、图文块、分栏和独立会话；不推进后续PDF/IDE/Agent功能。环境Windows x64、Node22、React19、Tauri2/WebView2、SQLite12。没有数据库迁移或真实用户资料修改。架构与文件职责见[工具栏设计](../architecture/MARKDOWN_TOOLS.md)。

## 已执行与待完成

- npm ci：退出0，锁文件干净安装成功。新增remark-gfm4.0.1及18个新增包，许可已核对。
- 最终npm run check：退出0，69文件524测试通过，类型/lint/生产构建通过（artifacts/dock-evaluation/v413-check-history-atomic.log）。新增选区/历史/查找/Unicode/资源删除断言。npm run format:rust、test:rust、check:rust均退出0，236通过/7忽略。
- node scripts/markdown-toolbar-smoke.mjs v413-final：退出0，10组实际DOM/键盘交互；选区与浮动栏、焦点/光标、撤销重做、Ctrl+K作用域、多行标题/列表、代码公式/表格、查找替换、组合事件、两主题、480/760/1440宽度及75–200% CSS缩放、原图文文本字段通过。
- npm run tauri -- build --debug --no-bundle：退出0，实际Windows调试应用。
- node scripts/markdown-toolbar-native-smoke.mjs v413-second：退出0，8组实际Windows WebView2，真实Rust保存与进程重启，图片块ID/sourceScreenshotId/图片在上注释在下、PNG/assets哈希保持。

- guide-reader-v413-first：退出0，14组，21章127例33图真实解码、导航/主题/字体/缩放/多面板通过。
- workspace-native-v413-final、commands-native-v413-final、resources-native-v413-final：均退出0，分别7/8/6组原生回归通过。
- Python发布传输8个、继承映射3个测试均退出0。

- markdown-toolbar-native-v413-structure：退出0，最终9组原生回归通过；新增真实素材选择器插图、删除其已编辑图文块后文字历史清空，第一张图片和原PNG保持。对应调试构建退出0（v413-debug-structure.log）。

- 最终原子历史版本：v413-atomic-debug.log构建退出0；v413-atomic-tools/commands/workspace/resources.log分别9/8/7/6组均退出0，共30组原生回归。
- 插图验证失败时不改变正文/光标/撤销历史，既有图文模式插入文本也重新开始文字历史，避免旧快照吞掉新增块；新增单元回归覆盖。
- 最终隔离正式安装构建和安装检查均退出0，详见[发布QA](RELEASE-4.1.3.md)。

公开发布与下载校验待完成，尚未记录不代表通过。

## 修复与未通过记录

初轮单元测试发现空首行H1没有插入，已修复；初轮浏览器发现effect清理取消光标恢复帧，已修复；窄窗CSS放大时根body最小480px造成横向溢出，已改为弹性最小宽度并压缩图标侧栏，矩阵重跑通过。原生初轮因图文编辑器懒加载尚未挂载便读取块数失败，补充等待实际字段后8组通过，未删除断言。

正式构建初轮因用户正在运行target/release中的旧程序而无法删除EXE，退出1；不关闭用户程序，改用全新隔离构建目录及复制的缓存重建。Step03额外旧workflow-smoke前台窗口保护失败、历史advanced-native退出超时仍不是本轮通过项。没有物理截图、用户真实库、macOS、HarmonyOS、Zotero写回、外部IDE/Agent验收结论。

## Windows人工验收（未执行）

1. 用练习稿选择中文/英文/emoji、多行与空行，逐一测试H1–H6、粗斜体/删除线/高亮、列表/引用/分隔线；核对只修改目标范围和预览。
2. 实际启用Windows中文输入法组词、候选、提交、撤销；组合期间按格式快捷键，确认不吞字。自动组合事件不能替代此步骤。
3. 图文练习稿添加素材图并格式化图片注释，保存、关闭、重开；确认图片位置、注释、原图、引用和assets完整。
4. 两份Markdown与图片编辑器并排，分别Ctrl+B/I/K/Z/Y、Ctrl+Shift+P和已配置截图组合；确认状态/撤销和全局命令不串面板。
5. 查找/替换字面中文、标点及大小写，检查跨块范围提示；全部替换后一次撤销。预览/源码往返后显式保存并重启。
6. 两主题、12/16/20px字号、短/窄窗口、Windows125–200%物理DPI及跨屏恢复；工具栏横向滚动、浮动栏和光标保持可用。CSS缩放不是物理DPI证明。

已知限制：复杂嵌套格式的状态是保守识别，非完整富文本语法树；高亮是扩展。图文模式查找限当前字段，图片按钮沿用已有添加顺序。下一阶段须另行授权；本轮完成后停止。

发布前额外长度边界检查复现极大量替换的RangeError，候选尚未正式发布。已加入结果长度预判与可见错误，UTF-8超限仍由既有文件模型拒绝，只有实际写入成功才显示替换数量；新增失败后原文保持的单元/浏览器/原生用例。最终结果补充后才发布。候选NOTICES尾部多余空行已清理，许可正文完整保留并重新构建验证。

## 最终限额修复候选的实际结果

npm run check退出0，69文件525测试通过（v413-replace-limit-check.log）。浏览器工具栏11组通过（v413-limit-browser-final.log）；最终Windows工具栏10组、命令8组、工作区7组、旧资料6组均退出0，共31组；明确覆盖预判字符串长度和既有UTF-8模型拒绝两条失败路径，原文不变，不显示替换成功。调试与正式构建均退出0，v413-limit-distribution实际安装/卸载4组退出0，16个AppData文件只读哈希保持，新增许可校验通过。最终SHA见发布QA。

该修复发生在v4.1.3草稿公开前：此前af70b6c为本任务创建的候选标签，不是正式Release；只在核对远端仍为该候选且Release仍是草稿后，以期望SHA保护更新本轮候选，历史正式标签不动。旧候选和全部附件清单保留在本地artifacts中。

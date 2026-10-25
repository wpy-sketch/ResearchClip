# Step07 数学论文 Markdown 验收

本轮目标：数学公式、八种环境、可配置编号、稳定标签/交叉引用、源码兼容性、界面和指南，以及v4.1.6正式发布。基点9ac5702，先写[技术契约](../architecture/MATH_PAPER_STEP07.md)再实施；不进入其他阶段。

## 设计和修改范围

paper-model负责只读解析、规则验证和全文编号；paper-document给旧图文caption提供全文偏移。MarkdownPreview只切分渲染片段，保留跨片段引用定义，绝不AST序列化正文。PaperTools通过原editSource/editBlock提交，核对所属会话及原文，沿用保存、关闭确认和撤销。MathFormula复用已安装KaTeX0.19.0的受限渲染边界。

projection把支持的定理容器与标签看作整块，未知容器仍保护。content-model适配带标签的图片，原截图/资源ID和caption关联不变。MarkdownPanel、VisualEditor、DocumentView接入这些显示与操作。paper-tools.css、GuideView、USER_GUIDE、练习稿及版本文件更新；发布脚本仅扩展既有受控附件继承流程。文件清单以本轮Git Diff为准。

新增直接依赖声明micromark-extension-gfm3.0.0、mdast-util-gfm3.1.0，均为原锁文件已安装版本，MIT，现有GFM许可覆盖；没有升级传递包。见[官方摘要核对](evidence/v416-dependencies.json)。不修改Rust业务、迁移、SQLite12、Tauri权限或真实用户资料。

## 当前执行状态

2026-10-25最终npm run check退出0：612测试/73文件，类型、Lint与构建通过；npm ci --ignore-scripts退出0。Rust格式、测试、Clippy退出0：237通过、7忽略。发布传输/继承/文件IO的16项Python测试通过。文档检查通过，不将其称为运行验收。

本轮53组Windows原生、45组浏览器与2组真实记事本读取通过。最新论文8组覆盖矩阵/aligned、八种环境、错误原文保存、单步撤销/重做、配置、带标签图片、未保存关闭、重开/真实重启，以及指南键盘焦点；原生其余45组覆盖表格/图片/代码、分栏、工具栏、工作区、命令与资源。浏览器含24种论文主题/窗口/CSS缩放组合，指南24章149练习36图片实际解码。

开发期直接cargo启动未找到生产页面，该次未通过，改为Tauri嵌入构建后运行成功；指南定位曾丢失焦点，修复预览重复渲染后在浏览器及Windows重跑通过。最终证据见[检查清单](evidence/v416-editor-checks.json)，保留早期失败日志，不冒充通过。

最终Windows NSIS构建退出0；实际安装/占用保护/卸载/清理4组通过，三份许可哈希匹配，16个既有AppData文件只读哈希不变，隔离安装已移除。安装器SHA-256：825f557ca52537b3cdadb047d5f7a67b511e5633fbbdc36fc6c9df966e3d13c5。正式发布、匿名下载和旧发布终审均已完成，详见文末正式回执。

## Windows人工验收与已知边界

在练习副本打开examples/markdown-paper.md及assets，检查矩阵/aligned、八种环境、图表与引用；改变编号规则并重排，再保存、关闭和重开。复制标签制造重复、输入缺失引用和错误LaTeX，确认保留原文与可见诊断；修正后恢复。核对旧截图、图文caption、Ctrl+B/I/K、Ctrl+S与未保存关闭取消。

在解锁Windows下用中文输入法、多显示器/DPI、12–20px字号和小窗口操作公式弹窗、长公式滚动与拖放。浏览器和WebView自动化不能代替物理IME、OLE拖放和多DPI人工验收。已有锁屏OLE前台归属保护失败仍保留，不能绕过；没有原生PDF位置选区、Zotero写回或外部Agent新业务。

公式单式8192字符、全文256式/128KiB预算，论文对象2048上限；超限保留源码并提示。未知/嵌套环境、未来配置版本不会静默覆盖。普通编辑器能读完整源码，是否渲染扩展由其自身决定。安装包未签名。

开发、正式发布和附件验证均已完成，本轮停止。下一步前置条件仅为解锁Windows后完成上述人工验收；其他开发阶段需要新的用户请求。


正式发布与最终审计已完成：261项实际匿名下载通过，旧35版1210附件前后终审通过；应用标签b5216de未重写。见[正式发布回执](PUBLICATION-4.1.6.md)。本轮停止，不进入下一开发阶段。

# Step06 可视化内容编辑设计

2026-10-24，基点e9654db（v4.1.4正式发布回执后）。已复查AGENTS、现有架构、model/store、FormattingToolbar、投影、预览和导入链。继续使用同一正文、原服务、受管assets与撤销；SQLite12，无迁移。仅用户Step06，不接入IDE/PDF阅读器或PDF导出。

表格是当前原文字段内的有界局部编辑：网格插入、单元格、增删行列、对齐与Tab导航；只有显式提交才用普通GFM表格替换该区间，未编辑区间保持原字节。遇到不规则行列、未支持的复杂语法或超预算时说明原因并保留原文，不能默默截断。表格编辑器持有打开时的文档ID/字段ID/原文快照，目标已变则拒绝提交。源码、近WYSIWYG的当前块及旧图文caption共用同一选区桥接。

图片拖入复用原导入服务，素材插入复用LibraryPicker与insertImagesAt。图片操作按投影图文组或原ImageBlock进行复制、移动、删除和属性修改。复制创建新块时使用既有newId；原图不改。源码相对路径保持，旧私有文档标题以caption中的可读图题表示，避免升级私有格式或让旧版丢失关联；普通源码支持标准Markdown alt与可选title属性。尺寸、行号与折叠均为视图选项，不写HTML宽高或私有扩展到.md；普通软件使用自身布局，未知原属性不自动清洗。

新增代码预览组件只消费围栏文本，语言选择局部修改info字符串；复制用既有受限copy_ocr_text IPC，不增加剪贴板权限。行号/长代码折叠为视图状态。选择lowlight3.3.0（MIT）及其锁定highlight.js11.11.2（BSD-3-Clause），以安全HAST→React文本/span渲染，禁止innerHTML；仅注册lowlight的common常用语言集合以及MATLAB/LaTeX，有界处理，未知/超大代码明示纯文本回退。官方依据：[lowlight](https://github.com/wooorm/lowlight)、[Highlight.js](https://github.com/highlightjs/highlight.js)、[核心API](https://highlightjs.readthedocs.io/en/latest/api.html)。本轮实际npm view已核实版本、许可及依赖；安装后已核对实际API与两份许可，并完成实际WebView高亮/复制/语言/折叠实验。替代方案为自行词法着色或完整IDE，前者维护/准确性不足，后者超出范围。

最近格式仅保存少量已成功格式动作名，复用工具栏，不持久化正文或路径。快捷插表/图片限定当前Markdown正文、排除组合输入和截图/全局命令组合；/table、/image保持有效。图片插入为空笔记时自动创建可编辑“注释：”区域，覆盖已有PDF截图从素材库插入的共同路径；PDF阅读器/原生PDF选区截图命令目前仍未实现，不伪称该接口已验收。

测试覆盖表格转义/代码中的管道、大小/空表/删除边界/对齐、过时提交、图片组及旧ID关联、复制/删除撤销、语言围栏和原文保真、快捷键与最近格式；再做Windows真实保存、PNG哈希、进程重启、现有回归与安装。新增依赖做干净安装；发布255附件时继承34旧版955附件原名原字节，保持既有发布顺序。

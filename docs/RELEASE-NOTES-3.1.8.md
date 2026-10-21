# ResearchClip v3.1.8 · 命令面板与可组合工作流

- Ctrl+Shift+P或顶部命令入口搜索9种常用操作，支持方向键、Enter、最近文档和无结果提示；检查并阻止与全局截图配置冲突。
- 三个主动触发预设：截图建卡、截图OCR校对插入、选择素材识别公式校对插入；自定义组合只显示兼容动作，可保存并重启恢复。
- 步骤进度、取消、结果查看、从失败步骤重试；保留已完成检查点，卡片写入不确定时停止，防止重复写入。
- OCR与公式结果暂停等待用户校对，修改后重新确认；Markdown目标切换时停止，未打开文档时在截图前提示。
- 指南18章顺序编号、110项示例；命令列表、步骤时间线及窄窗口/明暗主题布局优化。

无新依赖或数据库迁移；SQLite保持schema10。文件和资料继续受管，没有脚本或任意命令执行权限，没有自动上传或模型下载。取消Windows文字OCR需等待引擎返回；任务不跨重启自动恢复。原有功能及兼容WebClipper v1.0.2、MathOCR Setup v1.0.4、离线指南、模型清单均保留；Qwen大资源复用[v3.1.2附件](https://github.com/wpy-sketch/ResearchClip/releases/tag/v3.1.2)。

操作见[使用指南](USER_GUIDE.md)，架构见[命令与工作流](https://github.com/wpy-sketch/ResearchClip/blob/v3.1.8/docs/COMMAND-WORKFLOWS.md)，实际测试与边界见[本版验收](qa/RELEASE-3.1.8.md)，安装见[配套附件](https://github.com/wpy-sketch/ResearchClip/blob/v3.1.8/docs/RELEASE-COMPONENTS.md)。Windows需要WebView2，程序未签名；未完成的硬件及macOS矩阵不作为已通过宣称。

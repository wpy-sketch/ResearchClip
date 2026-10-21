# ResearchClip v3.1.6 · 界面与功能演练指南

## 阅读和界面

- 指南新增84项功能示例，覆盖全部16章，每项含操作与完成标志；目录可切换“操作目录／功能示例”，按关键词搜索并定位正文。
- 整理任务路线、目录与标题层级，修正OCR/相似检索的章节指引；保留步骤、失败处理及29幅离线图示，更新本版深浅色阅读界面图。
- 调整侧栏滚动、背景层次、工具按钮圆角、卡片边界和正文行宽；短窗口可滚动访问导航，底部使用指南保持可达。
- 原有截图、标注及版本历史、OCR、素材库、项目、文献、Markdown、WebDAV、相似检索、双向链接和知识图谱继续保留。没有新增依赖或SQLite迁移，schema9不变。

## 下载与配套附件

本Release提供桌面EXE、NSIS安装器、兼容 **WebClipper v1.0.2 ZIP**（含Native Messaging Host与注册脚本）、**MathOCR Setup v1.0.4 ZIP**、离线指南ZIP（含虚构Markdown/BibTeX练习稿）、模型说明及清单、SHA256和源码提交记录。配套组件沿用原版本与原始摘要，不冒充重新开发。

可选Qwen模型及CPU运行时与v3.1.2相同，已有`similarity-ai`可继续使用。首次使用请按本版`MODEL-RESOURCES.txt`和`OFFLINE-RESOURCE-INDEX.json`从 [v3.1.2资源发布页](https://github.com/wpy-sketch/ResearchClip/releases/tag/v3.1.2) 获取；`.021`–`.024`各有8个运输片，先还原四卷再合并全部25卷。只取解压后的模型目录，使用本版3.1.6应用；旧包EXE仍为3.1.2。轻量安装器和MathOCR脚本ZIP均不包含大模型，公式OCR还需按说明主动准备Python及权重。

旧Release及其全部附件保留，发布脚本在发布前后比较24个历史Release的ID、标签、源提交及附件ID/大小/摘要。新Release全部附件分别匿名下载校验；历史元数据核对不冒充重新下载所有旧模型。

操作见[使用指南](USER_GUIDE.md)；附件安装方法见 [组件说明](https://github.com/wpy-sketch/ResearchClip/blob/v3.1.6/docs/RELEASE-COMPONENTS.md)；实际验证与未执行项见[本版验收](qa/RELEASE-3.1.6.md)。Windows需要WebView2，程序未签名。物理高DPI/多屏、macOS及第三方服务完整矩阵仍需分别验证；本版不新增PDF或独立备份恢复系统。

# ResearchClip v3.1.7 · 图片区域锚点

- 截图编辑器新增点/矩形锚点：标题、说明、原像素坐标、显示隐藏、拖动/精确编辑、独立保存和删除确认。
- Markdown提供“图片锚点”选择器，在光标插入稳定ID引用；预览点击打开截图、居中缩放并高亮3秒，保留右侧未保存文档与编辑状态。
- 原图SHA256版本绑定、独立乐观锁、失效提示；普通标注历史变化仍保持位置，原图版本不符时停止跳转。
- A10反向链接/索引兼容区域引用；标准Markdown副本保留可读标题、坐标及说明，失效目标保留文字警告。
- 指南17章按从上到下顺序重新编号，94个功能示例；窄分栏编辑工具和锚点管理布局优化。

SQLite从9迁移至10，沿用升级前校验/安全快照/事务回滚；无新第三方依赖、OS深度链接注册或任意本地路径访问。锚点当前仅本机，不随WebDAV、旧可编辑项目JSON或PNG迁移。

本Release保留兼容WebClipper v1.0.2、MathOCR Setup v1.0.4、离线指南和模型资源清单；Qwen大资源仍复用 [v3.1.2资源](https://github.com/wpy-sketch/ResearchClip/releases/tag/v3.1.2)，已装无需重复下载。附件与安装方法见[组件说明](https://github.com/wpy-sketch/ResearchClip/blob/v3.1.7/docs/RELEASE-COMPONENTS.md)。旧Release及附件保留。

操作见[使用指南](USER_GUIDE.md)，数据契约见[图片锚点](https://github.com/wpy-sketch/ResearchClip/blob/v3.1.7/docs/ANNOTATION-ANCHORS.md)，实际检查和未验证项见[本版验收](qa/RELEASE-3.1.7.md)。Windows需要WebView2，程序未签名；完整物理硬件矩阵和macOS须分别验证。

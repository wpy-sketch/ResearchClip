# ResearchClip v4.1.4

Markdown新增源码、近所见即所得、实时预览和只读预览四种视图。近所见即所得渲染非活动块，点击当前块编辑原文；整篇原文为唯一正文，模式切换不会重新序列化公式、表格、图片路径或未知扩展。

新行输入/调用heading、table、image、math、code、quote；支持键盘选择、取消与既有撤销。图片继续从原素材库选择。拖动手柄重排内容，图片和连续说明段整体移动；旧图文稿按原块ID、图片/caption整体移动。实时预览双向比例同步滚动，窗口和图片加载后重新测量。Ctrl+S、未保存状态和关闭确认沿用原服务。

局部文本修改保留未触及CRLF/混合换行；修复带标题的受管相对图片引用读取。未知容器和大型稿明确回退受保护源码、限制结构拖动。未引入CodeMirror/Milkdown/Tiptap运行依赖，选型比较见[技术记录](architecture/EDITOR_STEP05_DECISION.md)。这不是完整富文本编辑器，比例同步也不是逐行像素对齐。

深浅主题与窄窗工具栏延续现有颜色角色；指南扩为22章133项示例34图、10条任务路线。Windows和保真测试、未验证项见[Step05验收](qa/MARKDOWN_STEP05.md)及[发布检查](qa/RELEASE-4.1.4.md)。物理IME、多DPI及真实资料仍须人工验收。SQLite保持12，无新权限、遥测或资料上传。

本版249个附件覆盖此前33版全部706个原附件：历史223分卷原样继承，前版241个无冲突附件直接提供，v4.1.3的SOURCE/SHA在对应Metadata.zip中保留原名原字节；详见V414-HISTORY-INVENTORY.json及[下载说明](RELEASE-COMPONENTS.md)。不更改历史Release或标签。没有Markdown导出PDF或独立数据备份/恢复系统。

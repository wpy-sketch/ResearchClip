# A12 命令面板与有限工作流

`features/commands/model.ts`是动作元数据和配置契约；`commands.ts`的9个命令复用已有UI/服务。`CommandCenter`提供搜索、键盘选择、最近文档、兼容动作组合、进度、校对和结果。`runner.ts`是单个有限线性执行器；`runtime.ts`把7个已知动作绑定到截图、受管素材、Windows OCR、本地公式识别、Markdown草稿服务。沿用Zustand和React，无新依赖、数据库迁移、网络上传或通用脚本运行器。

## 动作类型与配置

| actionId | 输入 | 输出 | errorPolicy |
| --- | --- | --- | --- |
| capture / select | none | image | retry |
| card | image | result | inspect |
| ocr / formula | image | draft | retry |
| confirm | draft | confirmed | retry |
| insert | confirmed | result | retry |

每个注册项还包含title、execute；运行前验证注册项与声明一致，动作完成后核对输出类型。配置仅包含`id/title/actions`，不能提供execute、文件路径或命令。自定义ID为`custom-<32小写十六进制>`，最多16项，每项2–8步，不重复动作，必须从none到result。Rust设置服务独立验证相同白名单与边类型，拒绝未知字段；前后端均验证名称/数量/重复ID。三个预设保留固定ID。配置在现有settings.json原子写入，保留外部损坏/未来格式的原文件，默认空配置兼容旧设置。

## 执行与取消

用户主动启动后只允许一个运行。当前步骤、已完成检查点、值和错误只保存在进程内；重启读取配置但不恢复任务。成功动作先记检查点，再检查取消；取消时成功写入的卡片仍可查看，不自动删除或重复执行。可重试步骤只从失败索引继续。卡片写入错误可能是IPC回执不确定，errorPolicy=inspect，不自动重试，要求先核对库中结果。

选择/确认等待可取消。Windows OCR本身无现有取消API，因此等待返回，丢弃取消后的结果；公式任务调用已有取消接口并等待终态。框选截图可以Esc取消。状态为cancelling期间不能启动新运行。已完成的识别、截图不因后续重试重做。

OCR与公式都经过确认动作；修改校对内容会取消确认勾选。公式以KaTeX安全预览，识别文本不执行HTML或脚本。插入绑定开始时的Markdown文档ID，后续关闭、切换或忙碌时停止；不写入另一个文档，不自动保存磁盘文件。没有Markdown时预检直接停止，尚未截图。截图插入命令复用标注合成；工作流OCR使用原图整图；局部框选/选语言仍由已有面板提供。

## 快捷键与安全边界

源码检查覆盖应用Ctrl+K、文档Ctrl+S/Shift+S/Z/Y、编辑器Z/Y和单键工具及Esc、系统截图配置。Ctrl+Shift+P仅在main页面捕获，忽略IME、重复按键和其他打开的对话框。Rust拒绝将其新设为全局截图快捷键；旧配置保留字节，但该全局绑定暂停并明确提示。系统或其他应用可能自行拦截；工具栏命令入口可用，Windows实际键盘验证见本版验收。

工作流只调用已有受限IPC，没有新Shell、任意路径、脚本、批量上传、循环或自动覆盖权限。选择图片限定素材库，外部图片用既有导入流程；所有资料写入仍由原有存储服务和文件授权校验。最近文档通过稳定ID和已有文件授权打开。

实际测试、Windows覆盖和未完成矩阵见[本版验收](qa/RELEASE-3.1.8.md)，逐功能示例见[使用指南第16章](USER_GUIDE.md)。

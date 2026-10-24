# Step03：受控命令与跨模块通信

实现位于src/features/commands/platform和Rust commands/research_commands.rs。复用既有命令面板、逐步OCR/公式工作流、DocumentFiles、WorkspaceManager和SQLite资源目录；没有新建业务存储服务、通用工作流引擎或完整IDE后端。无新增依赖，SQLite保持schema12，本阶段无需迁移。

## 调用与职责

可信UI显式动作 → CommandRegistry → PermissionService → TaskManager → ResourceResolver → Rust类型化只读校验 → 原有文档/图片服务 → 类型化通知。事件不会自动执行命令。

| 组件 | 职责与界限 |
| --- | --- |
| CommandRegistry | 7个封闭命令名，明确参数，资源级互斥；同请求ID/同参数复用结果，异参数拒绝，失败不自动重试 |
| PermissionService | 每个实例私有WeakMap签发/撤销最小模块句柄；调用前和await后复核，不信任module字符串或客户端权限字段 |
| ResourceResolver | 按既有ResourceKey(kind,id)查询；只返回key/formatVersion/editor/moduleAvailable，不返回路径、正文、token或任意目录列表 |
| EventBus | 3个受限事件及独立发布/订阅能力；冻结载荷、因果链去环、命令/事件共同因果链8层上限、异常隔离、128监听预算、once/AbortSignal/幂等释放 |
| TaskManager | 最多4个并行任务，协作式取消、提交边界、单调阶段进度、有限历史、结束后释放控制器 |
| ErrorReporter | 有界20条本地诊断，固定安全消息和校验后的命令/请求ID；原始异常文本、正文、路径、凭据不进入诊断 |

布局与EditorSession的存储、可见性、脏检查仍见[工作区架构](WORKSPACE_SYSTEM.md)。文档与图片状态没有复制到命令层。

## 命令契约

请求为{version:1,requestId:32位hex,command:{name,args}}。资源ID沿用Step01；请求ID仅为操作去重键，不创建第二套资源标识。输入由TypeScript重建允许字段，Rust serde枚举与deny_unknown_fields再校验版本、类型、ID和范围。

| 命令 | 参数 | 本版状态 |
| --- | --- | --- |
| document.open | resource：document/screenshot/pdfAttachment键 | Markdown/已入库图片可用；PDF附件不可用 |
| markdown.insertImage | document键、screenshot image键 | 向已打开、已登记的指定文档插入已保存素材，保留草稿与撤销，不自动保存文件 |
| pdf.navigateTo | pdfLocation键 | 明确MODULE_UNAVAILABLE |
| screenshot.capturePdfSelection | pdfLocation键 | 明确MODULE_UNAVAILABLE |
| annotation.create | pdfLocation键、highlight/comment/ink枚举 | 明确MODULE_UNAVAILABLE；不与私有Stroke混用 |
| codeWorkspace.open | codeWorkspace键 | 明确MODULE_UNAVAILABLE，无终端/IDE进程 |
| agent.requestEdit | agentSession键、document键、正整数expectedRevision | 明确MODULE_UNAVAILABLE，无模型调用或执行桥 |

本轮实际UI接入「命令面板→最近文档/当前素材面板」及已登记Markdown的素材库InsertButton。未保存文档、编辑器未保存合成图及其他旧入口保留原服务，未一次性重写全部旧命令。原逐步工作流的配置、确认、取消和重试仍由原runner管理。

## 权限边界

新增Rust接口只有resolve_research_resource与prepare_research_command，均仅main窗口可调用；覆盖层/贴图不获得权限。真实窗口身份来自Tauri，不接受caller/actor/grants字符串。没有通用文件系统、SQL、Shell或任意事件执行权限。prepare仅做元数据校验，并非一次性文件授权或事务提交票据；实际业务仍由既有Rust命令重新校验文件/资源约束。

TypeScript句柄用于可信前端模块之间的调用约束，不宣称同一WebView中的JavaScript模块构成OS安全沙箱。外部文献不运行脚本，当前没有Agent进程或外部命令桥；未来Agent必须另建后端受限协议与用户审批，不能直接拿UI调用入口或Tauri main权限。

## 重复、事件与取消

每次运行保留最多1024个请求收据（含失败），不逐出旧收据以避免旧ID重放。达到预算明确拒绝，用户保存后重启。不是跨进程exactly-once；重启不自动重放任务。调用方重试必须生成新请求并先核对结果。

事件与后续异步命令携带同一因果上下文；事件回调再次请求同一祖先命令时，在第二次处理器开始前拒绝。处理器显式调用子命令须传入context.cause。

事件仅resource.opened、markdown.imageInserted、command.completed；动态字符串、未知字段、伪造身份/因果句柄被拒绝。仅同步监听器受支持，异步监听器明确拒绝，意外thenable被释放并报告；异步业务应经命令执行，不在事件中偷偷启动自动工作流。事件异常不阻断其他订阅者。

取消发生在资源查询等协作阶段；进入原有文件/草稿操作前commit禁止取消，避免已修改却报告取消。后端只读查询不能中断SQLite调用，但返回后检查abort，不启动编辑处理。完成/失败释放资源锁；诊断不自动重试或清理资料。

## 兼容、验收与回退

无数据库迁移、无原图/PDF页面结构修改，不访问Zotero数据库。原有权限与数据安全规则继续有效。失败保留草稿和注册记录；未接入模块不返回成功。

本轮测试见[Step03验收](../qa/COMMAND_SYSTEM_STEP03.md)。回退需保存草稿、退出，再使用既有版本；不要降级真实数据库或删除布局/原图来排错。Step03停止后不自动进入PDF、Zotero或Agent实现。

布局适配补充：CSS内容缩放时，将停靠库DOMRect坐标平面归一化，再在业务面板保留目标缩放；根窗口高度同步归一化，顶栏按主区域容器宽度紧凑显示。没有修改依赖源码或把CSS模拟当作物理DPI验收。

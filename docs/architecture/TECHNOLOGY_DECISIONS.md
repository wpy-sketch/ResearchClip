# Research IDE 技术决策记录

Step02实施说明：[科研工作区](WORKSPACE_SYSTEM.md)。前述Step00选型状态为历史时间点，当前增量仅新增固定版FlexLayout。

实施状态更新：本轮仅落地Step01资源模型，复用既有依赖；具体契约见[RESOURCE_MODEL](RESOURCE_MODEL.md)。IDE、PDF、Zotero、HarmonyOS及Agent候选接口仍未集成。

日期：2026-10-23。状态：Step 00 设计，未安装候选依赖、未升级锁文件。参见[现状](CURRENT_STATE.md)、[目标](TARGET_ARCHITECTURE.md)、[路线](../ROADMAP_RESEARCH_IDE.md)。

## 1. 保留当前架构

保留Tauri2/React/Rust/rusqlite模块化单体，Rust唯一协调资料写入。复用Zustand、Markdown文件服务、截图和保存管线。不增加ORM、DI容器、第二个全局状态框架、微服务或通用工作流引擎。必要外部运行时按需启动。

## 2. 三条IDE路线

下表是架构成本判断，未进行三产品本机集成或性能对比。

| 维度 | Monaco | Eclipse Theia | Code - OSS |
| --- | --- | --- | --- |
| 定位 | 浏览器代码编辑器组件 | 可定制IDE平台 | 完整VS Code开源工作台 |
| React/Tauri复用 | 高，新增视图 | 中低，需要桥接平台布局/前后端/DI | 低，不是React组件 |
| 文件树/终端/LSP/DAP | 需本项目分阶段实现 | 平台已有相关能力 | 工作台已有，但自行构建维护 |
| VS Code扩展 | 不可直接运行 | 兼容扩展仍须逐项测试 | API接近上游，授权与市场不继承 |
| 进程 | Rust加按需语言进程 | Node后端加前端，桌面通常Electron | Electron发行版或复杂web/server适配 |
| 存储一致性 | 容易复用现有契约 | 需桥接平台文件服务 | 需长期桥接两个工作台 |
| 许可 | MIT | 包声明EPL-2.0 OR GPL-2.0-only WITH Classpath-exception-2.0 | 源码MIT，微软发行版/商标/部分扩展不同 |
| 维护代价 | 自行承担语言/文件/终端集成 | 平台升级、后端、扩展及双布局 | 上游大范围变动、构建分发和安全补丁 |
| 决策 | 首选渐进路线 | 必须兼容明确扩展集合时再评估 | 暂不采用，不fork工作台 |

Monaco官方区分model/URI/editor/dispose，普通VS Code扩展不能直接运行，也不承诺移动端支持。只使用公开API、ESM和本地worker，首期作为代码视图，不替换Markdown保存服务。[官方README](https://github.com/microsoft/monaco-editor)

Theia采用Node后端、JSON-RPC通信及自身DI体系，不是简单安装一个编辑组件。[官方架构](https://theia-ide.org/docs/architecture/)、[包与许可](https://github.com/eclipse-theia/theia/blob/master/packages/core/package.json)

Code-OSS源码许可不代表微软发行版和市场授权；官方FAQ限制替代产品访问Visual Studio Marketplace，不能承诺专有扩展通用兼容。[源码](https://github.com/microsoft/vscode)、[微软FAQ](https://code.visualstudio.com/docs/supporting/faq#_can-i-use-a-microsoft-extension-outside-of-vs-code)

## 3. 实际版本核实

| 候选 | 2026-10-23只读查询 | 兼容性与实际验证 |
| --- | --- | --- |
| monaco-editor | npm 0.57.0，MIT | 未安装；Vite8/Tauri/CSP/IME/worker/Diff未验 |
| @theia/core | npm 1.76.0，上述双许可表达式 | 官方同版React19 peer范围，不等于本工程集成通过 |
| Code-OSS | 官方main/package.json为1.144.0、MIT | 浮动开发快照，不是推荐锁定版本；未克隆/构建 |
| pdfjs-dist | npm 6.4.299，Apache-2.0，Node >=22.13.0或>=24 | 本机22.22.2满足，项目最低22.12.0不满足；需选择兼容固定版或另行调整最低环境 |

查询命令为npm view相应包的version/license/engines，无install。候选官方仓库可读且持续维护，但不代表任一版本都兼容当前工程。PDF.js开发分支Node要求与npm发布包不同，不能混用。[PDF.js](https://mozilla.github.io/pdf.js/)、[开发元数据](https://github.com/mozilla/pdf.js/blob/master/package.json)

Monaco进入实施前需通过离线worker、中文IME、撤销、分栏、主题、真实Diff、dispose和内存预算实验；失败回退现有textarea/用户外部编辑器，不放宽权限。

## 4. 统一资源与迁移

旧项目、截图、Markdown、文献保留已有ID。新领域使用相同ID规范，统一查询采用类型加ID的适配层/视图，不为旧资源分配第二套UUID。持久ID、临时session ID、授权token、内容hash、格式版本和revision分别管理。文件移动改locator，不能按文件名/DOI误配。

schema11及001–011迁移原字节保留。新增模型先隔离演练，旧ID/原图/标准Markdown不改写。CHECK列表扩展须版本迁移并核对外键/索引/触发器。未知未来schema拒写，不自动降库。旧WebDAV不自动同步新类型。

## 5. Zotero插件API

插件运行在Zotero进程，ResearchClip不直接读写Zotero SQLite。JS文档并不完整，7/8平台有变化，兼容范围必须由实际版本验证决定。[JS API](https://www.zotero.org/support/dev/client_coding/javascript_api)、[插件开发](https://www.zotero.org/support/dev/zotero_7_for_developers)、[Zotero8变化](https://www.zotero.org/support/dev/zotero_8_for_developers)

标准批注权威留Zotero。官方说明批注存其数据库，外部PDF页结构变化会破坏定位，因此禁止增删/重排/旋转原PDF页面。[PDF Reader](https://www.zotero.org/support/pdf_reader#using_an_external_pdf_reader)

本轮读到上游Zotero.Annotations.toJSON/saveFromJSON及类型定义，仅列为后续探针候选；未在用户安装版本调用，不能据此编造Reader定位或Connector端点。[上游annotations源码](https://github.com/zotero/zotero/blob/main/chrome/content/zotero/xpcom/annotations.js)

上游代码AGPL-3.0-or-later，不复制其Reader到主应用。插件和SDK分发许可须逐项核查。Zotero库身份与item/attachment key组合映射，单独item key/DOI不作为全局ID。

## 6. PDF与私有手写

PDF.js为Apache-2.0只读解析/渲染候选，先验证worker/CSP/text layer、损坏加密PDF、旋转CropBox。失败先保留Zotero原生阅读，不自建解析器。ResearchClip私有ink与标准批注使用不同领域类型和文件空间，不自动互转；既有图片pen格式不改，新ink独立schema。

## 7. HarmonyOS

采用ArkTS/ArkUI原生伴侣，不搬Tauri/Monaco。华为API14触摸文档搜索结果可读，但正文抓取为空/曾超时，补读OpenHarmony官方对应源码。不能据此证明商业HarmonyOS SDK或目标机型可用。[华为文档](https://developer.huawei.com/consumer/en/doc/harmonyos-references-V14/ts-universal-events-touch-V14)、[OpenHarmony文档](https://github.com/openharmony/docs/blob/master/en/application-dev/reference/apis-arkui/arkui-ts/ts-universal-events-touch.md)

实施时固定DevEco/SDK/API/机型/笔，真机验证pressure、tilt、历史点、手掌抑制、断线。未上报压感标缺失，不用固定值冒充。OpenHarmony许可不替代华为SDK条款。

## 8. Codex与Claude Code

Codex官方app-server适合自有客户端的会话、流式事件和批准，默认stdio，可以由固定CLI生成匹配schema。WebSocket被标实验性，首选stdio；线程/turn/取消/批准事件为后续探针候选。thread/shellCommand等旁路不默认暴露，不复制宽权限示例。[Codex官方协议](https://learn.chatgpt.com/docs/app-server)

Claude有TS/Python Agent SDK及CLI子进程路径，其权限、session、流式契约与Codex不同。[Agent SDK](https://code.claude.com/docs/en/agent-sdk/overview)、[权限规则与回调](https://code.claude.com/docs/en/agent-sdk/permissions)

本轮未运行供应商会话、未读认证文件、未发模型请求。Claude CLI受商业条款约束，不能因GitHub仓库公开便假设可自行分发。一个SDK LICENSE路径读取404，许可未完全核实，不猜测捆绑授权。[平台要求](https://code.claude.com/docs/en/setup)、[CLI许可](https://github.com/anthropics/claude-code/blob/main/LICENSE.md)

优先用户自装固定运行时，不自动改全局配置。Agent在受限暂存项目形成Diff，核对hash/CAS后应用。cwd、Job Object、提示词或批准回调都不是OS隔离证明；越界访问探针不过则仅开放只读建议，不开放执行。云推理发送选中上下文必须明示授权。

## 9. 未验证接口

| 接口 | 本轮证据 | 实施门槛 |
| --- | --- | --- |
| Monaco/Tauri worker/Diff | 官方文档及包版本 | 固定版Windows离线实验 |
| Theia/Code-OSS嵌入 | 架构/许可/元数据 | 变更选型时另开实验 |
| Zotero读取/定位/写回 | 官方文档与上游源码 | 实际版本、隔离profile、幂等/并发/权限 |
| HarmonyOS压感 | 部分官方文档 | SDK编译及真机/笔 |
| Codex协议/沙箱 | 官方文档 | 固定CLI schema、认证/取消/拒绝/越界 |
| Claude权限/许可 | 官方文档，SDK许可未全取得 | 固定运行时、分发条件、OS隔离 |
| PDF坐标互转 | 官方渲染资料 | CropBox/旋转/UserUnit/页标签往返 |

以上推荐是基于源码和官方资料的设计判断，不是候选接口已在ResearchClip运行成功的声明。

# Step01：统一资源模型实际验收报告

日期：2026-10-23（Asia/Hong_Kong）。本报告覆盖Step00完成后的Step01增量；[Step00基线](BASELINE_REPORT.md)独立保留。源码原基点为53e4bfa994d4c648b5b4d0545cb1b87a164e033f，原工作区干净。没有提交、发布或操作真实用户资料库。

## 1. 任务目标与完成情况

完成统一身份、类型/版本、双向关联查询、schema12迁移、旧截图/Markdown适配、改名/移动后的定位维护、只读孤立/坏链接诊断及Rust访问层和测试。12类实体均有实际模型或对旧表的等效适配，详见[资源契约](../architecture/RESOURCE_MODEL.md)。

| 请求验收 | 实际证据 | 结论 |
| --- | --- | --- |
| 旧截图仍能打开 | 独立v11库迁移；PNG字节/hash、原ID及annotation保留；Windows真实IPC加载并进入图片编辑器 | 自动化通过；真实用户库未操作 |
| 旧Markdown引用正常 | 原ID、中文正文/hash、相对PNG assets、researchclip截图URI；真实右侧源文/预览及跳转 | 自动化通过 |
| 数据重启后保留 | 新元数据/多类型关联的Rust关闭重开；Windows进程退出后重新启动读旧资料 | 自动化通过 |
| 同一PDF关联多个截图 | 一个PdfAttachment关联两份旧截图，双向查询及重开一致 | Rust隔离库通过；这是元数据关联，不是PDF阅读验收 |
| 不同类型可关联查询 | 旧目录及新模型依赖投影、手动关联、Incoming/Outgoing/Both、分页及失败原子性 | Rust隔离库通过 |

本轮没有PDF导入/阅读、Zotero桥/写回、HarmonyOS界面、IDE运行后端、Agent执行及外部移动修复UI。这些不是Step01验收的业务承诺，不能因模型存在宣称已实现。

## 2. 修改的文件与主要技术设计

| 文件 | 变更 |
| --- | --- |
| src-tauri/src/domain/research.rs | 具体类型、ResourceKey、版本、几何/路径/笔迹/大小校验及Zotero复合定位键 |
| src-tauri/src/research.rs；domain/mod.rs；lib.rs | 公共Rust访问入口及旧实体别名，复用原Storage |
| src-tauri/migrations/012_research_resources.sql | 新实体元数据、旧表UNION目录、关联、Zotero键、Markdown指纹、索引和保护触发器 |
| src-tauri/src/storage/resources.rs；resources/tests.rs | 查询/写入/CAS/依赖/只读审计/重连；18项隔离单测 |
| src-tauri/src/storage/migrations.rs | schema12事务迁移及结构验证，沿用迁移前一致快照及未知未来版本拒写 |
| src-tauri/src/services/markdown_document.rs | 在原文件服务边界增加只读UTF-8内容指纹 |
| src-tauri/src/storage/projects.rs | 原Markdown登记事务附加指纹；项目删除识别新模型必需依赖 |
| src-tauri/src/storage/mod.rs、anchors.rs、embeddings.rs、links.rs、ocr.rs、references.rs、revisions.rs、similarity.rs、sync.rs | 当前schema断言改用常量；历史夹具仅在cfg(test)移除新表再重建历史schema，原失败/快照断言保留 |
| src/shared/api/library.ts；library.test.ts；src/features/library/store.test.ts | 素材库接受schema12；新增拒绝11/13/109的明确回归用例 |
| scripts/check-docs.mjs | 文档校验匹配schema12；保持001–011发布SQL逐项不变检查 |
| scripts/resources-native-smoke.mjs | 真实Windows隔离v11→12、旧截图/Markdown/引用/进程重启自动化 |
| docs/architecture/CURRENT_STATE.md、TARGET_ARCHITECTURE.md、TECHNOLOGY_DECISIONS.md；docs/ROADMAP_RESEARCH_IDE.md；docs/qa/BASELINE_REPORT.md | Step00五份交付，附本轮增量状态链接；保留原基线事实 |
| docs/architecture/RESOURCE_MODEL.md；docs/qa/RESOURCE_MODEL_STEP01.md | 当前实际模型、API、边界、升级方案与本报告 |

设计采用原32位随机hex稳定ID和(kind,id)键；旧记录不重编号、不复制正文。标准PDF批注与私有手写层/Stroke分离，PDF位置绑定内容hash，Zotero键按instance/library/item限定。模型自带的必需关联在事务内更新，用户显式关联保留。文件路径只是定位元数据，不构成文件/执行授权。

新依赖：无。package.json、npm/Cargo锁文件、001–011迁移、Tauri capabilities/命令权限不变。未连接Zotero数据库、未运行模型或Agent、未新增上传/遥测。未移动/删除/覆盖用户文件；演练只在新建临时/仓库artifacts隔离目录操作。没有新增独立备份/恢复系统；迁移快照沿用原安全机制。

## 3. 实际测试环境、命令与结果

Windows 10.0.26200 x64；WebView2 155.0.4283.62；Node22.22.2、npm10.9.7、Rust/cargo1.99、Python3.14.6。默认沙箱通道后续对简单读取也超时，实际命令使用正常Windows权限并限定仓库/隔离资料；不借此对真实AppData执行测试。具体基线及初始环境失败见Step00报告。

下列日志位于artifacts/research-ide-step00-20261023（忽略的验证产物，不加入源码）。重复重跑不累计为新增测试。

| 最终命令 | 退出码 | 实际结果/日志 |
| --- | --- | --- |
| npm run check | 0 | TypeScript、ESLint、63文件429测试、Vite构建通过；step01-frontend-final.log |
| npm run format:rust | 0 | cargo fmt --check；step01-format-final.log |
| npm run test:rust | 0 | 230通过、0失败、7忽略；含18项新增资源测试；main/doc tests各0；step01-rust-final-r2.log |
| npm run check:rust | 0 | Clippy all-targets -D warnings；step01-clippy-final.log |
| npm run tauri -- build --debug --no-bundle | 0 | 嵌入最终前端的Windows debug应用；step01-debug-final.log |
| npm run desktop:build | 0 | 最终Windows release EXE，无安装包；step01-release-final.log |
| node scripts/resources-native-smoke.mjs step01-20261023-r4 | 0 | 六组Windows WebView2/真实进程检查通过；step01-native-r4.log；独立report.json位于artifacts/resources-native-step01-20261023-r4 |

额外运行既有advanced-native编辑器回归：research-ide-step01-20261023及-r3各完成4组，但等待原生应用关闭超时，整体FAIL；脚本随后清理仍未正常结束，最终仅终止本轮已核实PID/隔离profile所属进程，首轮执行会话退出-1；-r3经CIM终止后外层会话为0，这不是正常脚本通过，以report.json的FAIL为准。-r2因前次WebView占用专用CDP端口而立即退出1、0组。日志step01-advanced-native.log、step01-advanced-native-r2/r3.log均保留；原脚本未改。Step00曾完整通过5组，本轮未重现其关闭/重启部分，原因未确定；这是未解决的额外原生回归项。新资源专用脚本r4的真实退出/重启六组已独立通过，不能据此覆盖该失败。

资源单测覆盖：v11→12旧ID/PNG/annotation/笔记/Markdown/链接和已校验schema11快照；012失败事务回滚；未来schema拒写；跨类型关联及重开；所有新领域元数据持久化；Zotero跨库/实例同键不冲突和禁止静默重绑；CAS；未知格式；PDF不可变内容/hash不一致；标准批注/私有层隔离；有限坐标、压力null/范围、点时间与层预算；路径越界；依赖/端点不存在回滚；旧改名/整份文档与assets移动后的重连/重开、撤销失效WebDAV路径授权；未建立基准、错误内容及已登记目标拒绝；孤立/坏链接/旧锚点只诊断不清理；旧ID冲突及必需依赖删除保护。

Windows六组验证使用测试生成PNG，证明存储/界面兼容，不能替代物理截图测试或真实PDF解析。Step00另有三次物理GDI探针结果；本轮未扩充硬件矩阵。

## 4. 未通过、初始失败及修复记录

- Step01初次Rust全量为220通过、6失败、7忽略：历史测试通过“当前schema再删除旧表”构造旧版本，却遗留012对象；另有Clippy collapsible_if。已在cfg(test)夹具明确清除新对象并采用当前schema常量，保留快照、旧资料与失败断言；生产迁移不忽略冲突，故意012冲突用例仍验证回滚。最终230通过。
- 原生r1/r2失败于素材库加载，r2直接IPC已报告schema12，但前端parser只接受11；修正实际契约并补回归，最终npm check通过。
- 原生r3完成三组后失败：脚本把已有截图URI的知识卡片导航误认为直接进入编辑器；夹具也须遵循v11已有的Windows canonical verbatim路径格式以免制造重复文档。校正夹具/交互断言，r4还明确验证唯一原Markdown ID和指纹ID；通过六组。没有通过改业务导航或删除验收目标绕过问题。
- Step00原有split-native/smoke旧文案失配仍未修复，保持原报告FAIL，不宣称全量原生回归全绿。

七项Rust ignore不计通过，仍需剪贴板/屏幕/真实识别或外部服务条件。没有删除核心断言、放宽未来版本拒绝、清空真实库或用伪结果制造通过。

## 5. 已知问题与限制

1. 统一资源层当前为Rust数据访问接口。PDF/代码路径是元数据，文件尚未登记或没有独立授权时诊断明确报告；不自动读代码树、不启动进程。
2. 外部Markdown移动修复提供Rust接口，界面入口尚无。调用方须先关闭原会话、经原生选择提供目标及assets；旧文档需先正常打开/登记建立内容指纹。无基准或内容变更时拒绝猜测重连。
3. 新实体当前仅本机存储，未加入既有WebDAV协议；不能声称新资源跨设备同步。只读审计有预算/truncated，未扫描项目不是“无问题”。
4. 用户真实资料库尚未升级；本轮自动化仅验证独立旧库。发布安装器、签名、回退到真实资料目录未执行。
5. 既有Vite主chunk约780kB警告仍在；未以本阶段名义大改打包或升级依赖。
6. 额外advanced-native旧编辑器回归在退出阶段重复超时，重启组未完成。应以隔离profile复现窗口关闭/后台线程释放并定位原因；本轮没有修改该流程，不宣称已修复。
7. 预先存在的.gitattributes对所有文件启用LFS而当前HEAD保存普通文本，普通git diff会显示LFS指针。实际源码审阅使用临时禁用clean/process的git命令，未改Git配置/属性、未stage/commit。后续发布前应另行核对LFS策略；此问题不影响已执行的构建/测试。

## 6. 用户数据保护与现有应用升级方案

先隔离演练，后真实升级；本轮已完成前者。现有Storage::open验证quick_check/结构、校验迁移前一致SQLite快照并禁止覆盖，再执行012事务。012失败不提交新版本或部分DDL，原资料/快照保留；未知未来schema拒写。旧PNG/Markdown文件和原表ID不重写。

现有应用升级时，先保存草稿并退出旧实例，用用户许可的资料副本验证，然后再启动已验构建；逐项核对截图、项目、Markdown及assets、引用和搜索。旧应用应拒写schema12，禁止手改user_version降库；若确需回退，仅经用户明确授权在新位置使用已验证升级前副本，不覆盖唯一资料。完整步骤见[模型文档](../architecture/RESOURCE_MODEL.md)。

## 7. 需要人工验收的操作

| 操作 | 状态与条件 |
| --- | --- |
| 用户代表性旧库升级、连续编辑/保存/退出、外部文件缺失/移动提示 | 未人工验收；需经用户允许的副本及实际文档 |
| Windows安装包/升级/卸载、混合DPI/多屏/HDR/区域/窗口/滚动截图 | 未人工验收；需对应环境；自动WebView脚本不能代签 |
| Zotero标准批注往返 | 未实现/未验证；在对应阶段使用隔离profile，不直接写SQLite |
| HarmonyOS压感/倾斜/手掌抑制 | 未实现/未验证；需固定SDK/真机/手写笔 |
| IDE、Codex/Claude协议、Diff及OS权限拒绝 | 未实现/未验证；需后续阶段和固定运行时 |

文档检查：npm run check:docs退出0，130份文档、769个本地链接、代码围栏及命令权限/schema12和001–011一致性检查通过。该结果仅证明文档/静态契约一致，不是业务运行验收。

## 8. 下一步前置条件

本轮Step00及Step01到此停止，未进入Step02。下一阶段须用户明确指定范围；多工作区复用本次资源键与唯一Markdown服务，PDF/Zotero/手写/IDE/Agent分别按路线的依赖和人工门槛实施。正式升级真实用户资料另需代表性副本验收及用户安排，不能把本报告自动化结果当成全硬件/发布批准。

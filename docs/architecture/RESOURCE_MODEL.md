# Step01：统一科研资源模型与升级契约

日期：2026-10-23。实施范围：资源标识、版本、可查询关联、schema12、旧资源适配、文件定位维护与诊断、Rust数据访问层。依赖[Step00方案](TARGET_ARCHITECTURE.md)；实际验证及未完成项见[Step01验收](../qa/RESOURCE_MODEL_STEP01.md)。不开发PDF阅读器、Zotero插件/写回、HarmonyOS界面、代码执行或Agent运行时。

## 1. 统一身份，不复制旧资料

规范键为ResourceKey(kind,id)。id沿用现有32位小写hex（128位随机等效UUID），旧项目、截图、Markdown、文献的ID原样保留。路径、名称、DOI、会话token和内容hash均不替代资源ID。

research_resource_catalog是只读UNION视图：直接投影projects、screenshots、markdown_documents、literature_references及新增research_entities。它没有为旧记录再分配UUID或复制正文；旧服务仍是权威写入口。research_entities只存新增领域元数据，以(resource_type,id)为主键，格式版本1、revision乐观锁、创建/修改时间及有界JSON。JSON是具体的Rust带标签枚举，不接受用户SQL/脚本。

| 请求实体 | 实际模型/类型 | 兼容方式 |
| --- | --- | --- |
| ResearchProject | 既有Project的Rust别名；kind=project | 直接投影projects，原创建/重命名逻辑保留 |
| DocumentResource | Markdown投影＋新增PDF/Code文档元数据 | 既有Markdown仍由DocumentFiles和markdown_documents管理；新接口禁止新建Markdown或将.md登记为代码副本 |
| LiteratureReference | 既有Reference别名；kind=literature | DOI/引用键/来源及原ID保持，Zotero键另存映射 |
| PdfAttachment | documentId、可选referenceId、SHA256、byteSize | 必须引用PDF类型文档且hash匹配；同一附件可被多个截图关联 |
| PdfLocation | attachmentId/hash、0基pageIndex、显示页标签、页宽高、多个矩形、文本exact/prefix/suffix | 未旋转CropBox左上原点、PDF点；这是内部格式定义，尚未完成Zotero坐标适配 |
| ScreenshotAsset | 既有Screenshot别名；kind=screenshot | 原PNG/annotation/schema1不变，来源通过Source关联和原笔记来源获取 |
| Annotation | locationId、标准类型、text/comment、Pdf或Zotero来源 | 标准批注元数据，无Zotero写回；与私有层严格分开 |
| HandwritingLayer | PDF附件/hash/页及尺寸 | 私有层；绑定不可随意更换，不写原PDF或标准批注 |
| Stroke | layerId、工具/颜色/宽度、点坐标/时间/可空压力与倾斜 | 只允许挂到私有层，不能挂到标准批注；缺压感保留null |
| CodeWorkspace | 可选研究projectId、rootPath定位元数据 | 记录路径不构成授权；不读取源码、不启动终端/LSP |
| ResearchEvidence | claim、可选documentId及Supports/Source等关联 | 保存待核实研究结论及来源，不自动宣称科学正确 |
| AgentSession | workspaceId、provider、外部sessionId、phase | 仅元数据；无模型请求、默认权限、终端或网络执行 |

数据库schema12、资源格式1、实体revision、原Annotation schema1、PDF内容SHA和Zotero外部版本是不同概念。未知资源格式拒绝读取为可编辑实体/更新，诊断可报告unsupportedVersion；不抹掉未知字段。旧表无revision的对象在目录返回null，修改继续走原服务，不能伪造CAS版本。

## 2. 数据访问接口

Rust入口为researchclip_lib::research，复用同一个ResearchRepository（既有Storage类型别名）。内部SQLite连接和SQL仍私有，没有新增Tauri命令、capability或前端通用文件权限。

| 接口 | 语义 |
| --- | --- |
| research_resource / research_resources | 类型化键读取或分页目录；每页最多100，offset有界 |
| create_research_resource | 新领域元数据，后端随机ID，格式/依赖/大小校验，事务提交 |
| update_research_resource | kind不变、expected revision匹配；PDF内容版本和私有层页面绑定不可变 |
| relate_research_resources | Related/Source/Contains/Citation/Supports/DependsOn，有界枚举，重复关联幂等 |
| research_relations | Incoming/Outgoing/Both分页，每页最多200；能跨资源类型查询 |
| bind_zotero_resource / zotero_resource | 按实例＋library ID＋item key映射，重复同目标幂等，不允许静默重绑 |
| relink_markdown_resource | 调用方已选择的新位置＋原基准hash；只更新登记，不移动/覆盖文件 |
| audit_research_resources | 1–1000资源预算的只读诊断，超预算truncated；绝不清理用户资料 |

新资源可以经其带标签metadata反序列化为ResearchData。legacy=true的目录项应通过现有截图/项目/文献/Markdown服务读取完整内容，不能用新实体更新接口覆盖旧表。

## 3. 关联与约束

research_relations保存显式manual关联和从模型必需字段派生的model依赖。更新模型只替换该源的model边，保留用户manual边。research_resource_edges还实时投影旧项目成员、旧文献关联及link_index，旧写入口的变化即时可见；重复文中引用在资源层去重，原位置索引仍保留全部位置。

视图不能作为SQLite外键父表：关联插入用同一IMMEDIATE事务和数据库触发器验证两端；必需model依赖通过删除保护触发器防止旧项目、Markdown、文献或新实体被错误删除。显式manual关联及旧链接在目标删除后保留可诊断信息，不自动删除来源资料。原截图删除行为和原图文件清理仍由原服务处理。

ZoteroKey包含instanceId、正整数libraryId及8位itemKey，唯一键为三者组合。相同itemKey在不同库/实例不冲突，itemKey绝不是ResearchClip全局ID。本轮只保存外部定位元数据，完全不连接Zotero SQLite。

新文档locator具有唯一索引；PDF为受管pdfs下相对路径，代码为代码工程根内相对路径。校验拒绝绝对相对混淆、反斜杠、冒号、空段、..、Windows设备名等。路径只是定位元数据，不授予磁盘能力；代码文件检查报告fileGrantRequired，PDF尚未登记文件报告fileNotRegistered。

## 4. 资源校验和私有笔迹隔离

元数据上限2MiB；标题1–200字符。PDF附件hash为64位小写hex、大小大于0且不超过2GiB；这不是已导入/解析PDF的成功声明，真实文件导入属于后续阶段。

PDF位置页宽高/矩形须有限、非负、在页面内；页索引0基，页标签独立。位置/手写层必须与附件内容hash一致，内容变化登记新版本资源，不自动把旧位置套到新PDF。

私有Stroke最多1万点，层最多1万笔画/10万点；时间非递减，压力可空或0–1，倾斜可空或-90至90度，坐标/笔宽有限且在所属页内。测试只验证格式和预算，不能替代HarmonyOS真机压感。标准Annotation即使类型为Ink，也不接受私有Stroke挂接或自动互转。

## 5. 改名、移动及授权

应用内Markdown改名沿用DocumentFiles.rename及index_document(previousPath)，同一markdown_documents.id不变，正文/assets不改写。登记在同一事务写research_document_fingerprints（hash和字节数），使用现有Markdown文件边界读取，未建立第二套正文服务。

应用外移动后，数据访问层relink要求关闭旧文档会话，由上层原生选择流程明确提供目标。核对已登记基准hash、目标精确内容、原位置若仍存在时的内容和路径唯一性，成功只改locator，所有ID关联保持；不移动文件、不复制assets、不覆盖同名文件。调用方须一并选择/保留配套assets并用既有打开流程校验；迁移演练覆盖整份.md和assets移动后重新打开。

旧v11文档不会在迁移期间扫描外部磁盘。首次成功打开/保存/登记后才记录指纹；未采集指纹时拒绝猜测重连。失效指纹或目标已登记给另一文档均失败，原登记保留。外部路径重连会撤销不再匹配的WebDAV路径授权，权限不随ID自动转移。这里只提供Rust重连接口；当前界面没有新增外部移动修复按钮，后续UI必须复用系统文件选择和会话守卫。

## 6. 只读资源检查

诊断返回不带正文/完整路径的资源键和错误码：unassociated、unsupportedVersion、invalidMetadata、invalidModelBinding、missingOrUnsafeFile、missingFingerprint、fileNotRegistered、fileGrantRequired、missingRelationEndpoint、missingExternalKeyTarget、missingAnnotationAnchor。孤立是信息，不等于可删除；没有自动清理/修复。

检查原截图/已登记Markdown文件存在性，受管PDF逐段拒绝重解析路径，检查模型依赖/附件hash及旧图片锚点。没有凭此获得代码根读取授权，也不扫描未知文件。不存在深度PDF解析或对全部大文件重新计算hash的承诺。

## 7. schema12隔离演练与现有应用升级方案

已经先完成隔离v11→12测试：旧ID、原PNG精确字节、annotation原JSON字节、笔记、Markdown文本和旧URI索引保持；升级快照可只读打开且仍为schema11。故意对象冲突验证012DDL/版本回滚及原记录保留；未来schema拒写。完整执行结果以Step01验收为准。

现有应用升级建议：

1. 用户保存全部草稿并真正退出旧实例。保留原图、原库、外部.md/assets；不在运行时仅复制SQLite主文件。
2. 先使用已测试构建和旧资料的独立副本演练。正式启动由原Storage::open执行quick_check/结构校验、VACUUM INTO一致快照、禁止覆盖提交和012事务。
3. 只有验证快照和迁移成功才报告schema12就绪；失败停止打开写路径，保留库/快照和错误，不重置资料。多版本升级各版独立事务，支持从已提交中间版本重试。
4. 打开旧截图、旧Markdown/相对assets、旧URI、项目和搜索；检查原hash和新只读诊断。外部缺失文件不阻断所有其他资料，不按名称自动配对。
5. 若需回退，旧应用对schema12拒写，不能直接降库。仅在用户明确授权下使用升级前验证副本在新位置运行兼容版本，不覆盖唯一数据库。本轮没有在真实用户资料根执行升级或恢复。

这是现有迁移安全方案，不新增独立备份/恢复系统，也不涉及Markdown导出PDF。

## 8. 依赖与后续边界

新npm/Cargo/Python依赖：无。Rust数据层继续rusqlite/serde/sha2；Windows兼容脚本仅使用已有Playwright、Node和本机Python生成隔离夹具。旧001–011不改，旧WebDAV协议不变；新增实体尚不加入同步范围，必须明确显示本机模型范围，不声称完整跨设备同步。

下一步需要用户指定Step02或其他阶段，才增加会话UI、文件选择重连入口、PDF导入/阅读、Zotero能力、Harmony手写或Agent。元数据模型存在不代表这些业务已经实现。

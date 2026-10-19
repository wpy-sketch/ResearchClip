# A06 M1 数据模型适配与迁移提案

**仅提案，未应用。** 当前仍使用[005迁移](../../src-tauri/migrations/005_annotation_revisions.sql)及schema5；没有新增006 SQL，没有改动用户库。按用户要求，发现现有模型不足后先交付方案，不强行编写同步业务。

## 1. 现状、入口与阻塞

| 现状及实际文件 | 对资源同步的影响 | 级别（针对尚未实现的同步） |
| --- | --- | --- |
| [library.rs](../../src-tauri/src/storage/library.rs) update_screenshot / save_note使用单机整数CAS | A/B都可从revision1变2而内容不同；不能以revision大小判定新旧 | P1 |
| [revisions.rs](../../src-tauri/src/storage/revisions.rs) record使用同图MAX(version_number)+1；get_screenshot以MAX作显示版本 | 两端分叉均v2，直接插入触发UNIQUE；导入旧分支后MAX不代表current-head | P1 |
| library.rs delete_screenshot/cleanup；[projects.rs](../../src-tauri/src/storage/projects.rs) batch删除 | 资料物理删除、历史级联；文件队列完成后删除记录消失，无法可靠传递删除意图 | P0风险：错误推导可能丢资料；当前应用无同步，因此不是已发生的数据丢失 |
| projects.rs register_document以canonical path登记，非跨设备文档身份识别 | 两机同名文件ID可能不同；同一个绝对路径字符串不能在另一台机器复用 | P1 |
| [markdown_document.rs](../../src-tauri/src/services/markdown_document.rs) DocumentFiles grants / names / save | 文档是外部UTF-8文件+内存授权，非SQLite正文；外部正文/assets变化不更新索引时间，现有图片清单只覆盖受管PNG命名 | P1 |
| tags.normalized_name / projects.normalized_name本机唯一；标签跨项目共享 | 独立创建同名不同ID不能盲插/REPLACE；同步共享标签会影响未选项目的展示 | P1 |
| [knowledge.rs](../../src-tauri/src/storage/knowledge.rs) primary_note及additionalNoteCount | 导出同步只能取第一条UI笔记会漏资料，必须保留全部notes | P1 |
| [files.rs](../../src-tauri/src/storage/files.rs) ordinary/image_path/read/write | 现有受管原图校验/禁止覆盖可复用，不能为了WebDAV开放磁盘 | 可复用基础 |
| [clips.rs](../../src-tauri/src/storage/clips.rs) 受管web-clips-v1及请求幂等 | 继续保留已有文档ID/目录和浏览器协议；requestId不是同步revision | 可复用基础 |

审计脚本运行实际001–005 SQL，使用Python SQLite3.53.2和两个隔离数据库；测试不是Rust migration runner，也没有调用WebDAV。具体复现、环境和状态见[实际报告](../qa/A06-M1.md)。这些缺口不应被误报成v1.0.5现有单机功能已失败。

## 2. 拟议schema6（评审后再实施）

不重写001–005，不重置数据库，不删除任何原图/标注/笔记。继续使用既有迁移前完整性检查、唯一VACUUM INTO备份、校验和事务；对未知未来版本拒写。以下表名/列为设计，不是可调用API或已创建数据库。

| 拟新增实体 | 主要字段与约束 |
| --- | --- |
| sync_connections | connectionId、HTTPS endpoint、remoteWorkspaceId、credentialRef、capabilityResult/version/time；不保存用户名/密码/令牌 |
| sync_identity | workspaceId、deviceId、durableCounter；与本机OS安全绑定核对，禁止克隆库后复用同一设备序列 |
| sync_resources | workspaceId、kind、resourceId、localEntityId、localFingerprint、localRevision、remoteHeadHash、scope、状态；复合唯一；无remote local_path |
| sync_baselines | connectionId、resourceId、baseManifestHash、basePayloadHash、remoteETag、confirmedRevision、scopeGeneration；原子推进 |
| sync_outbox | operationId主键、resourceId、固定revision/snapshotHash、expectedBase/ETag、阶段、attempts、nextAttemptAt；重试身份不变 |
| sync_tombstones | workspaceId、resourceId、删除revision、baseManifestHash、创建时间、显式确认状态；不因清理原图或取消勾选项目被删除 |
| sync_conflicts | conflictId、resourceId、base/local/remote引用、原因、解决状态及resolutionOperationId；保留两份可编辑数据 |
| sync_apply_journal | applyId、resourceId、old/new hashes、staging/file/db阶段、授权引用、错误类别；断电后可判断完成或冲突 |
| sync_scopes | connectionId、selectedProjectId/未分类、generation、纳入时确认；取消范围不产生tombstone |
| sync_document_bindings | documentId、managed/external、仅本地路径/授权引用、lastSavedByteHash、assets inventory hash、确认状态；不自动取得磁盘授权 |
| screenshot_annotation_heads | screenshotId主键、revisionId，复合同图FK；显式当前编辑head，不用MAX代替 |
| sync_revision_map | workspaceId、globalRevisionId、screenshotId、localVersionNumber、immutablePayloadHash；global ID唯一，显示编号只在本机有意义 |

按最终实现合并能够共用的字段/实体，避免把提案机械变成一套通用工作流框架。sync_outbox是同步未完成工作，不替代现有用户标注撤销重做；tombstone是删除语义，不是独立全库备份。确认的删除不能通过队列完成就失去传播依据。

### 事务边界

1. 截图/标注/笔记/标签/项目/关联的新增修改，与本机sync变更标记在同一SQLite事务完成；不能“先改业务表，稍后异步写队列”而遗漏退出前的修改。
2. 删除前，在同一事务写带基线的tombstone，再走既有显式确认与文件清理。截图历史级联之前，若未同步的历史/原图仍被在途操作引用，需先固定有界内容对象或阻塞并提示；不能删完原图才发现还需要上传。
3. 应用远端更改必须识别为远端来源：基线/本地实体/索引同事务提交，不能把接收动作重新判定为本地新改动而无限回声上传。新用户编辑仍单独标记；禁止用无边界全局布尔值屏蔽所有触发器。
4. SQLite事务之外的外部Markdown保存遵守持久apply journal；文件先变更、索引写入失败时，重启必须能从磁盘hash识别实际完成阶段。未保存会话永不作为隐式同步源。
5. 同步快照固定后，网络不持有资料库mutex。长历史分段校验/暂存到磁盘；最终激活使用有界读入和SQLite事务，取消/失败回滚，不能把半份历史标为完整成功。

## 3. 旧数据初始化

1. schema6迁移事务只创建本地状态和head映射，不联网、不下载、不生成远端删除。
2. 既有截图ID、原图路径/字节、全部笔记/标签/项目/文档ID原样保留。同步默认关闭，服务器配置为空，不自动选全部素材。
3. 旧单机库中，每张截图的当前标注需与已保存JSON语义核对，再映射其对应历史revisionId；有多条相同内容时可选择最近匹配。没有一致匹配、未来schema或损坏数据则标为needsReview，不创建空标注、不悄悄修复。
4. 既有version_number保持不变；接收同一截图的其他设备历史分支，原revisionId保留，分配新的**本地显示**编号并记录映射，parent仍用全局ID。current-head单独指向所选分支。A05保存/恢复/get_screenshot必须同步改为读显式head；此依赖是M1不能直接跳过的原因。
5. 旧库已完成的删除没有可追溯tombstone，无法补造历史删除：从未被本同步连接确认过的资源缺失只能当unknown。首次同步绝不把旧缺失项当删除远端的理由。
6. 外部Markdown索引先标为未授权/未纳入。用户选择项目后列出文档数量，逐项或选定目录授权，再按原文件字节和真实图片引用生成manifest；不静默搬移或覆盖现有路径。
7. 现有assets只读扫描通过后才纳入。未支持命名/媒体、缺失资源、路径越界或大小写冲突会阻塞该文档，保留原文，不用删掉引用来制造“同步成功”。

## 4. 冲突落地与ID保持

- screenshot/note/tag/project/document ID沿用；同名不意味着同ID。冲突项先暂存，不把服务端ID拼接成文件名以外的任意路径。
- 同ID不同不可变原图hash属于完整性冲突，不能更新原图。保留另一份受管副本并要求用户决定是否建立新截图ID；新ID副本明确保留origin关联。
- 同一标注revisionId对应不同JSON/hash属于损坏/来源冲突，不能UPDATE历史覆盖旧内容。正常分支必须是不同revisionId，M5验证父关系与显示序号映射。
- 采用远端/本地不是立即销毁另一份；base/local/remote清单与所有引用在conflict可读区保留。保留两者需要新实体ID和完整引用映射，并通过现有名称约束校验。
- 外部文件不因同步自动成为可覆盖目标；每次手动同步预览中授权“应用这些更新/待确认删除”后，仍要核对旧hash。计划后文件再变，回到冲突，不沿用旧授权强行替换。

## 5. 后续迁移验收（本轮NOT_RUN）

1. 用包含真实格式原图、全部基本/高级Annotation对象、分叉历史、旧notes、跨项目tags、外部Markdown及普通assets的schema5隔离库迁移到6，逐项核对原始bytes/IDs。
2. 模拟备份不可写、建表失败、head不匹配、未来格式、磁盘满，验证回滚/保留原库及明确提示；严禁删库修复。
3. 两个设备身份从同一资料副本开始，分配不同deviceId；模拟复制库、重启、凭据不可读，不复用旧counter或覆盖远端身份。
4. 所有业务写入口（含批量、浏览器剪藏、知识卡片、防抖保存、A05恢复）都维护标记/基线，故障注入后没有业务已改但队列缺失的窗口。
5. 无变化同步不新增版本/文件；远端接收不回声；不同device同ordinal历史并入后当前编辑head和A05恢复仍正确。
6. 队列中断、远端已提交但本地未确认、文件已提交但DB未提交及半下载恢复，均用专用数据验证，不能借测试覆盖真实外部文件。

在上述门槛验证前，不应开始完整双向同步或v1.0.6正式发布。M2还需要实际测试服务器地址/类型和可写的独立测试根目录，用户已说明稍后提供；凭据应在实现的安全入口中本地输入。

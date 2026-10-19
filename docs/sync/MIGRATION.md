# SQLite schema6：WebDAV 一致性迁移

本轮实际实现，001–005已发布迁移不变。最初评审保留在[历史草案](MIGRATION-M1-DRAFT.md)。协议见[协议v2](PROTOCOL.md)，验收见[3.0报告](../qa/RELEASE-3.0.0.md)。

## 1. 版本化升级

复用现有迁移前验证、SQLite安全备份和事务；备份完成后运行006，失败回滚，未知未来版本拒写。没有删除/重建现有截图表或修改原图路径。schema5→6测试保留ID、标注、历史与PNG精确字节，检查备份仍为schema5；刻意冲突的迁移验证回滚及原记录保留。

## 2. 新表

| 表 | 用途 |
| --- | --- |
| annotation_heads | 当前revision ID与needs_review；不再用最大本机序号决定当前版本 |
| sync_identity | 本机计数器与当前绑定设备ID；设备身份另由OS凭据验证 |
| sync_config | 无密码配置：HTTPS目录、子目录、项目选择 |
| sync_state | 每连接/资源已确认hash、manifest、ETag |
| sync_queue | 固定清单待提交，prepared/uncertain/acknowledged |
| sync_conflicts | 双方清单、原因、处理状态 |
| sync_tombstones | 截图/文档/空项目明确删除记录及原项目范围 |
| sync_document_grants | 明确授权的已登记文档规范路径 |

## 3. 现有行为的最小修改

旧标注匹配现有历史JSON，能精确匹配时建立head；不能映射时needs_review并拒绝同步，要求用户先确认保存。不把损坏资料“修复”为空标注。新显式保存触发更新head；版本显示号仍单机递增，并发导入保留revision全局ID和父节点。普通撤销重做不产生同步版本。

删除触发器在级联前记tombstone，清理PNG失败不丢失删除意图。接收远端资源在短事务内修改领域记录、head与同步基线；文件先安全落盘，失败只可能留下无引用受管文件，不激活缺失依赖的记录。没有将网络请求放在数据库事务/锁中。

Markdown只有原本已登记且经用户授权的路径可读。接收内容使用新受管目录与原ID登记，不覆盖外部文件；这属于同步版本传输，不提供独立备份恢复功能。标签名称冲突人工核对，不合并不同ID。

## 4. 依赖

复用锁内版本并显式启用：reqwest0.13.5 blocking/native-tls（MIT/Apache2，Windows系统证书）、quick-xml0.42.0（MIT）、sha2 0.10.9（MIT/Apache2）、windows0.62.2 Win32_Security_Credentials。不新增前端依赖，不捆绑WebDAV服务器。开发隔离测试使用WsgiDAV4.3.3 MIT及cheroot11.1.2 BSD3，与应用无运行时依赖。

API依据安装版本源码/编译验证；凭据API见[CredWriteW](https://learn.microsoft.com/en-us/windows/win32/api/wincred/nf-wincred-credwritew)。未验证macOS安全凭据与同步运行；非Windows不能退化到明文密码文件。

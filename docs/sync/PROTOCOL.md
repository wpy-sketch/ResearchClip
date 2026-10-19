# WebDAV 资源同步协议 v2

v3.0.0 实现契约。SQLite schema6、Annotation schema1、远端协议2是独立版本。历史设计保留在[最初草案](PROTOCOL-M1-DRAFT.md)；实际测试见[发布验收](../qa/RELEASE-3.0.0.md)。本文不能证明任意服务商兼容。

## 1. 身份与一致性

每台设备保留自己的 SQLite，**不上传数据库、WAL或锁文件**。截图、项目、文档采用 `kind/32位hex` 稳定ID；标注revision保留原ID和父关系，本机显示序号不作为远端身份。Device ID保存在Windows安全凭据内，绑定连接与规范化工作区路径；复制DB到另一位置不会自动获得同一设备凭据。数据库持久计数器只递增，重试复用已准备的清单。

| 状态 | 实际存储 |
| --- | --- |
| Resource Revision | 清单内device与十进制counter；parents记录最多两个先前清单 |
| Content Hash | SHA256；快照序列化字节与原始UTF8/PNG分别哈希，不拿ETag当摘要 |
| Remote ETag | catalog GET的强ETag，原样用于If-Match |
| Last Synced State | sync_state内每连接/资源的内容hash、清单hash与catalog ETag |
| Pending Queue | sync_queue内prepared/uncertain/acknowledged；提交前先记uncertain |
| Conflict State | sync_conflicts内双方不可变清单hash；内容对象仍保留 |

## 2. 资源与范围

- **截图bundle**：不可变原PNG、当前标注JSON和明确head、全部版本历史、全部笔记及来源、标签定义/关联、项目引用、完成或纠正的OCR。缩略图在接收端生成，不上传。历史按父先序/ID稳定排序，不按单机version_number排序；并发分支合并后仍能继续编辑。
- **项目**：独立稳定ID与名称。截图中的项目只是依赖引用，不能覆盖独立同步的项目修改。
- **标签**：随截图带上定义与关联，不删除其他截图。不同ID同名、同ID不同名会暂停导入为冲突；须先人工统一/重命名再重试，首版没有自动标签合并器。
- **Markdown**：已登记的document ID、原始UTF8、相对assets与项目。外部文件必须经系统文件夹选择器授权；授权只涵盖已登记且规范化路径匹配的文件，不扫描任意文档。当前仅支持应用生成的 `assets/researchclip-ID.png`。其他本地路径、引用式图片或HTML图片会明确拒绝，不静默漏图。HTTPS远程引用保留，不主动下载。
- 设置、模型、Python、模板、临时截图、未保存草稿不在范围。选择项目与“未分类”决定范围；取消选择不产生删除。移动到范围外时远端旧资料保留，需人工核对。

## 3. 远端布局与条件提交

所有对象位于用户指定HTTPS目录内的 `researchclip-...` 子目录；客户端仅生成受限路径，不使用服务器提供的本地路径。

```text
researchclip-sync-v2/
  workspace.json
  catalog.json
  objects/<hash前2位>/<完整SHA256>
  probes/<随机ID>/...
```

使用 MKCOL、Depth1 PROPFIND、GET、PUT，不执行远端DELETE。目录列表最多2MiB/1000条；资源发现使用有界catalog，避免依赖服务商可能分页截断的对象目录遍历。catalog最多5000资源/2MiB，超限失败，不截断。

上传顺序：不可变二进制对象 → 不可变manifest → 条件更新catalog。对象PUT使用If-None-Match:*并读回验hash；catalog采用If-Match或首次If-None-Match:*。每次提交核对该资源预期的远端manifest；其他资源并发变化最多重新读取合并3次，同资源变化转为冲突。远端commit成功而本地ACK丢失时，重试先发现目标manifest已经生效，补确认，不新建重复版本。

**安全门槛**：每次手动任务开始都在新探针目录验证If-None-Match、正确/过期/伪造If-Match、读回字节，以及两个并发相同ETag写入恰有一方成功。不支持强ETag或可靠条件写入时停止，不能切到无条件覆盖模式。探针保留用于审计，不自动清理；测试消耗请求额度。

## 4. 下载、冲突与删除

下载先验证清单身份、大小、SHA256、Annotation schema/尺寸/历史父关系及PNG内容；依赖完整后才在本地事务中激活。原图只允许相同ID相同字节，绝不覆盖不同原图。Markdown接收版本写入新的受管 `sync-documents/<id>/<manifestHash>/`，不覆盖外部原文件；更新项目登记路径。

相同内容补基线；只有一端修改则传输；两端都修改保存双方快照，要求逐项选择本地、远端或两者。两者会用稳定派生ID创建独立副本，重试不重复；遇到远端删除，“两者”保留本地活动资料与远端删除快照。确认前再次验证双方内容未改变。

删除用持久tombstone表达；不存在≠删除。远端删除永远转为待确认，不自动清空本地。明确采用远端删除后，仅删除该记录；受管PNG进入可恢复清理队列，外部Markdown原文件保留。恢复活动截图会取消旧文件清理意图。不自动清理历史、对象或冲突快照，也没有独立全库恢复入口。

## 5. 执行与安全边界

单后台线程、单资源顺序处理、无自动定时同步；UI轮询只读本机状态。停止在当前请求结束后生效（连接6秒/整请求30秒），随后保留完成项与队列；退出时先请求停止；任务未结束会提示稍后重试退出，未确认任务下次手动恢复。网络失败不自动无限重试；用户再次点击手动重试。ETag冲突不属于网络重试。

生产仅HTTPS，使用Windows系统TLS且不禁用证书验证，不跟随重定向；Basic凭据仅向已配置origin与子目录发送。Windows Credential Manager保存凭据，数据库、JSON配置、日志和WebView状态不返回密码。输入大小、JSON未知字段、路径、文件类型、版本均严格校验；WebView只获6个main窗口限定业务命令，无通用fs/HTTP权限。

### 5.1 当前上限

单blob64MiB；截图最大16,777,216像素；单快照24MiB；标注历史最多1024条且JSON总计16MiB（超限失败，不删除）；笔记最多128条；Markdown2MiB、最多32张受管PNG、图片合计128MiB；项目最多500。同步缓存暂不自动回收，应预留磁盘空间。安全探针不是服务器全时正确性的数学证明，仍依赖服务器后续遵守HTTP条件语义。

## 6. 服务器验收边界

指定坚果云地址为 `https://dav.jianguoyun.com/dav/ResearchClip/`；旧Zotero目录不在测试/同步范围。此环境没有用户本机授权密码，因此认证后的坚果云验收未运行。不能据独立本地HTTP测试称坚果云兼容。

本机独立WsgiDAV4.3.3默认配置实际未通过并发/内容ETag门槛；保留失败结果。另一明确增加原子条件写入与SHA256 ETag的隔离配置用于双逻辑客户端一致性验证，不能把该结果归于默认WsgiDAV或坚果云。服务商实际不支持时应用应拒绝写入。

参考：[HTTP条件请求](https://httpwg.org/specs/rfc9110.html#field.if-match)、[WebDAV RFC4918](https://www.rfc-editor.org/rfc/rfc4918)、[坚果云第三方应用授权说明](https://help.jianguoyun.com/?p=2064)。坚果云请求额度/目录条数可能随套餐变化，首次只选择小项目。

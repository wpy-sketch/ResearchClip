# Step11：Windows外部PDF打开与Zotero只读来源

2026-10-25用户明确授权与Step10并行开始，发布顺序保持v4.1.9→v4.2.0。v4.1.9源码标签5358b22及安装/指南归档已经冻结；本文件属于后续Step11。

## 实测依据

本机实际Zotero10.0.3；只读GET到127.0.0.1:23119/api/和个人库单条附件接口均返回200，API v3、schema44、Server-ID存在。没有打开Zotero SQLite或修改真实偏好；未提交真实文献、服务器ID或个人库ID。安装源码fileHandlers.js的一般Windows外部阅读器分支将文件路径交给所选应用，不随路径提供可靠的library/item身份。

官方说明：[Local API](https://www.zotero.org/support/dev/web_api/v3/local_api)、[外部PDF阅读器](https://www.zotero.org/support/pdf_reader)。只读访问需要用户启用本机API；本阶段不申请写入密钥，不发出修改Zotero资料的请求。Zotero 10的服务器标识用于区分数据实例。接口事实与本机实测分开记录，旧版本没有可靠实例标识时普通PDF仍可读，来源持久绑定明确标为不支持。

## 打开路径和进程边界

复用现有单实例插件和PdfDocumentService，新增有限原生外部请求队列；现有Markdown没有外部启动队列，不虚构复用。启动argv和已有实例回调接收路径，作为独立参数处理中文、空格及合法特殊字符；不拼接Shell、不执行URL或文件内容。受限IPC只领取原生已经收到的请求ID，WebView不能提交任意路径。文件仍导入只读副本、校验摘要，内容相同聚焦已有PDF会话。

安装器注册HKCU内ResearchClip自己的PDF打开候选和带引号的命令，不夺取Windows默认应用，不改Zotero外部阅读器设置。卸载只移除本程序拥有的注册。用户在Zotero常规设置中明确选择ResearchClip程序路径。源码/测试、实际Windows安装注册和人工Zotero选择步骤分别验收。

## 来源匹配与权限

只读HTTP客户端固定到loopback23119，禁代理、重定向和任意URL输入；限制连接/请求时限、响应字节、候选条数和任务时间，提供取消及错误。先检查API版本和实例标识，再在明确个人/群组范围查附件候选，用API返回的本地文件URL核对实际授权路径及内容。父目录中的8位字符串最多是查询线索，不能单独构成关联；同名文件、相同key跨库、API未启用/未运行/权限拒绝/不匹配时保留普通PDF。

元数据预览显示标题、作者、年份、DOI、library范围和item/attachment key；确认后只写ResearchClip。原始API信息作为待核实来源，不能把Zotero字段当成已经核实的DOI元数据。已有同DOI文献可以复用稳定ID，原记录不被自动覆盖。

## 资源与迁移

继续使用原Document、PdfAttachment、LiteratureReference和关系层，不增加资源ID系统或第二套文献/Markdown存储。schema16扩展既有research_zotero_keys的库范围及来源快照：保留旧映射为legacy范围，新增user/group范围及0个人库兼容，身份同时考虑实例、库类型、库ID与key。旧无范围接口仅访问legacy，不猜迁移为新范围。实际API库ID与请求中的users/0别名分清；Source/DOI都不充当全局资源ID。

迁移前一致快照、版本化事务、失败回滚和未来版本拒写沿用既有机制。先隔离15→16及失败演练，核对旧ID/关系/截图/Markdown/引用/阅读位置。来源绑定与必要文献创建、映射、资源关系在同一事务，失败保留原数据；旧引用键与元数据不被改写。

## 验收与回退

验证Windows argv中文/空格/括号/&等、已有实例交接、重复PDF、仅路径普通打开、API关闭与超时、同名错误文件、范围隔离、来源确认及重启。测试只用隔离资料；真实Zotero只读探测不能替代外部阅读器UI验收。配置失败可继续使用原打开PDF按钮；移除来源绑定不删除PDF或文献。没有Zotero SQLite直接写入、批注写回、PDF页面修改、Markdown导出PDF或独立备份恢复。

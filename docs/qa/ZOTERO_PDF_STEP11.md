# Step11：Zotero外部PDF与只读来源验收

任务目标：在已有可停靠PDF工作区接收Windows/Zotero外部打开，增量保存明确范围的文献来源，完成v4.2.0发布；不进入Step12。

主要设计见[架构](../architecture/ZOTERO_PDF_STEP11.md)。复用单实例插件、PdfDocumentService、PdfViewerSession、稳定资源/文献存储和事务迁移。原生队列只向主窗口提供短期请求ID，来源确认只接受短期预览ID与PDF授权，不接受任意路径/SQL。HTTP固定loopback、GET、无代理与重定向，有限时限/响应/条数，可取消。来源由实例、库类型、library ID、item/attachment key共同定位；内容摘要和API条目版本确认时重新核对。

修改文件包括src-tauri/src/services/pdf_open.rs、zotero_local.rs、zotero_session.rs、commands/pdf.rs与zotero.rs、storage/zotero.rs及references.rs、migrations/016_zotero_scope.sql、lib/build/capabilities、Windows installer-hooks，前端pdf/ExternalPdfOpen、ZoteroSourcePanel、zoteroApi、PdfViewerSession与引用来源标签。指南/图标/发布脚本同步更新。没有新增第三方依赖，001–015迁移不改。

## 数据保护异常记录

首次原生测试的次进程启动遗漏隔离目录环境，误启动默认资料库实例。已立即停止该测试进程并修复脚本，后续首/次进程均继承同一隔离目录。默认库升级前一致快照已自动保留；只读比较确认原schema1仅app_metadata一条，原记录完整保留，迁移至schema16并增加一份虚构练习PDF。主库及此前备份文件原摘要保持，WAL/SHM发生变化。不擅自回滚、删除练习记录或覆盖数据库。此事件不能写成“全程只使用隔离资料”；新增测试记录可由用户日后通过应用自行处理。只提交汇总，不提交真实资料路径、正文或数据文件。

## 实际测试

实际Windows11 x64、Node22.22.2/WebView2环境。下列通过项实际执行并退出0；应用标签与测试格式后续修正分开记录，详见[机器证据](evidence/v420-editor-checks.json)：

- npm ci --ignore-scripts：327包干净安装通过；锁文件仅应用版本变化，未增依赖。
- npm run check：81文件、662项前端单元测试通过，类型/ESLint/前端生产构建通过。
- 应用标签的npm run test:rust、npm run check:rust：272通过、7忽略及Clippy通过；忽略项不是通过。标签新增测试的npm run format:rust原有格式差异，先前汇总误计为通过。后续只格式化四份cfg(test)区段，逐文件比较确认生产代码前缀字节不变；重新执行格式检查、272项Rust测试和Clippy通过，标签与安装包不重写。
- node scripts/with-rust.mjs tauri build --debug --no-bundle：实际Windows内嵌生产资源调试程序。
- 工作区7组、命令8组、资源6组、引用11组、PDF13组、Zotero外部打开与来源10组，合计55组Windows原生通过；全部使用隔离目录的最终重跑通过。
- node scripts/guide-reader-smoke.mjs v420-final：14组浏览器通过；28章/187练习/47图，明暗主题、不同字体、480–1440宽度、CSS缩放75–200%，不能替代物理DPI。
- 发布基础设施20项测试通过；既有001–015迁移未修改，新增16迁移成功保留旧键/阅读状态并有失败回滚测试。来源所有者、过期/重放、取消、跨库/实例、旧同DOI信息不覆盖均覆盖。
- npm run desktop:bundle及pwsh -NoProfile -File scripts/installer-smoke.ps1 -RunId v420-distribution -TargetDir src-tauri/target/v413-distribution：实际NSIS构建与5组安装/忙碌保护/关联注册/卸载/清理通过，四份许可摘要一致，安装期间所有原有AppData摘要保持。这个安装期间结论不能抵消上文测试误启动事件。

HTTP元数据为显式虚构练习内容，不能代表真实Zotero UI验收。首次原生脚本的隔离遗漏和后续退出时WebView先于IPC响应关闭的脚本竞争均已修复；完整最终重跑通过，旧失败日志保留在artifacts。Rust格式首次汇总错误也已在机器证据中明确纠正；不能把后续格式通过归于原应用标签。

## 未验证与人工验收

本机实际安装Zotero10.0.3，真实只读/api/和单条附件列表曾返回200、API3/schema44/实例标识；不包含真实文献数据。用户关闭原实例后记录了连接拒绝。独立Zotero测试配置放artifacts内，Windows界面访问授权等待超时，故未完成真实设置→双击附件链路；没有改真实Zotero设置或数据库。请按[指南第28章](../USER_GUIDE.md#28-从zotero打开pdf与关联来源)选择已安装程序，双击个人库与群组练习附件，核对来源预览与重启。普通PDF、API关闭、同文件重复打开和来源重启由原生自动测试覆盖；物理DPI、多屏和稀有PDF仍需人工。

旧版无实例标识只提供普通PDF阅读；仅路径不能确定Zotero键，独立附件无父文献不自动造引用；页面链接、批注读写和未下载附件不支持。下一阶段需要独立授权，本阶段完成后停止。

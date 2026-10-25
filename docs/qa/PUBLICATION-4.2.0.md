# v4.2.0 正式发布回执

2026-10-26，[ResearchClip v4.2.0](https://github.com/wpy-sketch/ResearchClip/releases/tag/v4.2.0)已正式发布，非草稿/非预发布，Release ID423576298，发布顺序保持v4.1.9→v4.2.0。应用标签固定ddfb6a33eb1b22a1e163a5fa4d8ec65265ca0046；[78文件完整变更](https://github.com/wpy-sketch/ResearchClip/commit/ddfb6a33eb1b22a1e163a5fa4d8ec65265ca0046)。不重写旧标签或冻结安装包。

Step11完成Windows PDF“打开方式”候选、独立路径参数与已有实例接收、同内容标签聚焦，以及可选的Zotero只读来源预览/确认。复用原PDF、资源和文献服务；以实例、库类型/library ID、item/attachment key共同分区，来源保持待人工核实。SQLite16事务迁移保留旧键和阅读位置；不写Zotero数据库、不修改PDF原件。没有新增应用依赖。

[设计](../architecture/ZOTERO_PDF_STEP11.md)、[验收与修改文件](ZOTERO_PDF_STEP11.md)、[机器证据](evidence/v420-editor-checks.json)、[设置与完整指南](../USER_GUIDE.md#28-从zotero打开pdf与关联来源)、[下载与附件说明](../RELEASE-COMPONENTS.md)。

## 发布附件与真实下载验证

285个附件全部实际匿名下载，大小与SHA-256逐项符合本地冻结清单；下载不携带Authorization。[只读验证40221072768](https://github.com/wpy-sketch/ResearchClip/actions/runs/40221072768)，job121252567022；接收器核对工作流身份、源提交、完整文件集合和本地摘要，见[公开下载证据](evidence/v420-public-verification.json)。

[继承40220634024](https://github.com/wpy-sketch/ResearchClip/actions/runs/40220634024)，job121251248436，在GitHub侧核对并复制277个既有非冲突附件，本地只上传本版8个新增附件。此前39个正式版本2290项原附件前后审计均PASS；原223运输卷不重压缩，同名SOURCE/SHA的原字节保存在版本Metadata.zip。应用主程序、安装器、便携包、当前指南、旧配套组件与旧发布附件均有明确清单定位。

NSIS SHA-256：d2ff6c4bd5f5819ab4c10d58968029c79fc94bfcca2b64d380b120302ca8ed33。安装器未签名，下载后可自行核对校验文件。

## 实际测试与修正范围

前端81文件662项通过，干净安装327包；应用标签Rust272通过、7忽略，Clippy和Windows构建通过。最终隔离工作区原生55组（工作区7、命令8、资源6、引用11、PDF13、外部打开/来源10）、指南浏览器14组、发布脚本20项通过。NSIS实际安装、占用保护、PDF候选注册与默认/UserChoice保留、卸载/清理共5组通过；四份许可一致，安装期间23个原有AppData摘要保持。

原应用标签新增测试的格式检查有差异，早期汇总误计为通过，已纠正。后续[16f7c5479f8ceda1265cc490a5a42909b7094cb7](https://github.com/wpy-sketch/ResearchClip/commit/16f7c5479f8ceda1265cc490a5a42909b7094cb7)仅修正四份cfg(test)区段的格式与文档；逐文件比较确认生产代码前缀字节不变，格式检查、272项Rust测试和Clippy重新通过。此通过属于后续提交，不能写成原标签格式检查通过；安装包和标签保持冻结。

原创科研烧瓶图标调整蓝绿黄红配色；来源面板使用明暗主题、圆角及有界滚动。指南28章、187项练习、47幅实际加载的离线图片；新增3幅原生练习截图，明确标为虚构文献/隔离HTTP服务，而非真实Zotero UI截图。

## 数据保护异常与未验证项

首次原生脚本次进程遗漏隔离目录，误启动默认资料库，已停止并修复。升级前一致快照自动保留；只读比较确认原schema1只有app_metadata一条，原记录保留完整，迁移到16并增加一份虚构练习PDF。旧主库/已有备份摘要保持，WAL/SHM变化；未擅自回滚或删除该记录。后续若需清理，须先确认用户意图与可用接口，不能覆盖恢复数据库。不能把本轮描述成“全程未触及默认资料库”。

实际Zotero安装为10.0.3；真实只读健康检查及附件列表曾返回API3/schema44/实例标识，独立测试配置健康GET也200。没有直接访问Zotero SQLite或改真实设置。Windows应用访问授权等待超时，未完成Zotero设置→双击附件的真实UI链路；原生argv/来源确认采用隔离HTTP服务测试，不能替代该项人工验收。请按指南28章实际选择researchclip.exe，测试个人/群组附件、重复打开、API关闭及重启。

7项忽略测试不是通过。物理多屏DPI/系统放大镜、旧WebView2、长期内存及稀有复杂PDF仍需人工；其他平台未验证。无可靠实例标识的旧Zotero接口只保留普通PDF阅读；仅路径不能证明item身份。Zotero批注读取/写回、页码传入、未下载附件仍不支持。

Step10/v4.1.9与Step11/v4.2.0开发、实际检查、正式发布和附件验证已完成；本轮停止，不进入Step12。下一步前置条件为真实Zotero人工验收和用户对后续阶段的新授权。不包含Markdown导出PDF或独立数据备份/恢复系统。

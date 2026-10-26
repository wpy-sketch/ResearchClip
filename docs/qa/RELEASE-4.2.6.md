# v4.2.6 / Step15 验收记录

## 目标与实现

在Step14稳定发布后，增量增加受控标准批注写入。复用ZoteroClient、ZoteroSessions、PDF原件授权/内容哈希、已确认来源、批注Repository与schema19缓存。无新增依赖或业务数据库迁移。高亮、下划线、普通便签使用官方HTTP接口；其他类型与未实测版本禁用。密钥只在Rust内存，完整实例/库/父文献/附件与PDF摘要绑定。POST逐项结果检查，PATCH/DELETE对象版本条件，GET复读核实；未知提交状态保留意图，禁止自动重发。

## 已执行检查

环境：Windows、PowerShell、Node22+项目工具链，Zotero10.0.6/Build20261007153950/API3/schema44。专用profile/data位于忽略的artifacts下，无账号同步，独立端口32122；从未直接写Zotero SQLite。

- 独立真实Zotero Python预检：5检查、30请求，PASS。三个类型逐一创建/读取/改字改色/读取/旧版本412/删除/404，原PDF SHA不变；缺密钥401与缺Server ID428。测试key仅注入全新测试profile的官方key文件，**不代表授权GUI通过**。
- Rust客户端真实Zotero集成：显式标记profile测试1项PASS，三类型CRUD、401、条件412及原件SHA。命令`cargo test ... real_marked_zotero_official_http -- --ignored --test-threads=1`，退出0。前两次编译/HTTP验证失败及修复后日志保留。
- `npm run check`：730测试/91文件通过，类型、lint及构建退出0（最终复跑同为730/91、退出0）。
- `npm run test:rust`：301通过、8默认忽略，退出0；其中真实Zotero集成已单独显式执行。`npm run check:rust` Clippy退出0；格式检查退出0。全量RUST_TEST_THREADS=4。
- `npm ci`在独立安装目录：327包，退出0。
- 发布基础设施：23测试通过，退出0。历史附件准备核实4090原附件，冻结45个旧Release快照。
- Windows原生最终回归：133检查，退出0；工作台7、命令8、统一资源6、本地引用11、PDF13、画质5、布局5、双向定位/摘录25、只读批注28、受控写入25。原生UI连接的是明确标记的虚构HTTP夹具，真实Zotero CRUD另计。
- 指南：独立Edge14检查，32章/241练习/65图，退出0；原生工作台亦验证指南加载。冷启动首轮goto超时保留，复跑通过。
- Windows安装包：实际NSIS构建退出0，隔离安装/卸载/运行中保护及载荷校验5检查通过，退出0；未覆盖已有安装，结束无残留。安装器17,000,692字节，未签名，SHA256 `91896da7a44d0b6e23da40e8a0745fcf8f20d5fd938287db4064ba156466e70c`。
- 默认ResearchClip资料：只读审计27文件、0变化，PASS/退出0。仅证明该应用资料范围，不代表全机任意文件审计。
- 文档检查退出0；最终源码与附件冻结后的正式发布、321附件匿名字节下载及旧Release不变审计，记录于后续发布回执。上述本地检查不冒充正式发布完成。

## 修复与失败记录

真实API早期401可能缺少LocalAPI身份响应头：只接受其为拒绝，不视为成功；成功仍严格核对实例和API头。Zotero10.0.6按JSON字段顺序导入属性：创建序列化显式将annotationType置前，避免排序JSON映射导致拒绝。Windows测试socket继承nonblocking导致读请求前WouldBlock，修正夹具socket并保留GET-only断言。首轮原生调试构建未嵌入前端，使用custom-protocol重新构建；其失败不计功能通过。原生测试还发现Saved额外字段被严格DTO拒绝，前端改为只提交AnnotationSource字段并加回归断言。

## 明确未验证与人工验收

真实Zotero授权弹窗需用户自己处理；computer-use技能禁止自动化认证/隐私授权，未代点Allow。真实只读群组、非作者批注、所有Zotero策略差异仍需专门资料库人工验收；HTTP403模拟测试不能替代。未验证自由文本、区域和ink写入，功能禁用。物理多屏DPI、真实客户端跨进程视觉定位仍需人工核对。

测试夹具XPI和Marionette初始化实验未成功，不作为接口通过；未引入产品插件、远程调试端口或私有API回退。首次隔离Zotero启动出现其Word集成安装成功日志；事前未采集Word模板hash，不能证明它未重写。事后用户STARTUP模板与安装包模板SHA256均为5B502497AC15BC1AFDC408A0C2A7F9E70E6C8C417A08FF83F7B2A1974096B6EC；后续全部隔离profile禁用Word/LibreOffice集成安装，未删除或猜测恢复用户模板。

## 修改文件与设计

核心：services/zotero_write_model.rs、zotero_write_transport.rs、zotero_write_tests.rs，既有zotero_local.rs与zotero_session.rs；commands/zotero_write.rs及main-only IPC注册。前端Writer/WritePanel、现有PdfViewerSession/AnnotationsPanel及PDF关闭确认。文档见[先行架构决策](../architecture/ZOTERO_ANNOTATION_WRITES_STEP15.md)、[本版说明](../RELEASE-NOTES-4.2.6.md)、[指南](../USER_GUIDE.md)。图标与发布继承工具同步更新。完整清单见[修改文件](CHANGED-FILES-4.2.6.md)，机器汇总见[检查证据](evidence/v426-editor-checks.json)。

## 数据与回退

schema19保持不变，原001–019迁移不变；旧截图/Markdown、PDF原件和Zotero数据库不覆盖。写入失败保留会话草稿，未知提交只读复核。关闭会话前确认草稿处理，本版不承诺重启恢复待处理修改；Step16将另行实现持久同步状态。默认ResearchClip资料27文件前后只读审计0变化。

## 下一步条件

本地原生回归与安装器已完成，正式发布仍须完成匿名附件校验；按用户新授权再开始Step16/v4.2.7。未实现Step16的同步队列不应写成本版已有功能。

## Windows人工验收步骤

1. 使用独立Zotero练习资料库与校准PDF，运行10.0.6并启用Local API；按第28章关联完整文献/附件。普通生产资料库不用于自动写入测试。
2. 自己处理Zotero授权对话框，分别检查拒绝、单次、记住授权；确认ResearchClip不自动选择权限。
3. 在ResearchClip从同页选文创建高亮、下划线，选择区域创建普通文字便签；到Zotero核对类型、文字、颜色、页码与实际位置，四旋转及不同缩放分别核对。
4. 在Zotero修改同一批注，再提交ResearchClip旧版本草稿；应显示冲突且保留输入，不覆盖。编辑新读取版本后更新，再明确确认删除并检查真正不存在。
5. 在专用只读群组及非作者批注上验证拒绝策略；接口403模拟通过不能替代该真实验收。自由文本、区域和ink写入保持禁用。
6. 断开API后观察旧缓存、草稿和待核实状态；重连仅复核，禁止重复创建。关闭草稿PDF应先确认，取消保留输入。v4.2.6会话草稿不承诺重启恢复，须先提交/人工核对或明确放弃。
7. 使用实际多屏和Windows显示DPI核对选区，不将CDP DPR/CSS zoom模拟说成物理验收。原PDF页结构及内容不变。

本轮无仍在失败的自动化检查；初始编译/HTTP字段顺序/来源DTO/夹具时序/退出竞态等失败均保留原日志，修复后重跑受影响检查。两个原生脚本复用相同证据目录导致EEXIST保护拒绝，改用新run ID完成写入最终回归，未删除旧证据。真实GUI/群组权限/物理DPI为未执行项，不写“通过”。

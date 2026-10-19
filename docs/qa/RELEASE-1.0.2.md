# v1.0.2 / A02 网页剪藏验收

日期2026-10-19，Windows11 x64、Node22.22.2/npm10.9.7、Rust MSVC1.99、Tauri2.12.3/CLI2.12.2、真实Edge/WebView2。只实现A02，后续A03/A04按用户新指令依次进行，本报告不代替后续版本。

## 实现与架构

- 独立browser-extension：MV3、TypeScript/Vite、固定本地ID、activeTab/scripting/nativeMessaging/storage四权限，无常驻内容脚本、Cookie权限或全站主机权限。扩展独立npm/Cargo锁文件可重建。
- 四模式：链接/来源、正文选区、Readability主要正文、页面图片选择；DOMPurify清理表单及危险标签/属性后Turndown转标准Markdown，预览纯文本，不执行HTML。实际提取源见scripts/clipper-browser-smoke.mjs。
- 独立native-host + protocol Rust crates：小端字节长度帧、来源/结构/版本/尺寸校验、标准输入/网络超时；Host不链接Tauri或SQLite。
- services/clipper.rs与commands/clipper.rs：用户按次启用，随机本机端口+256位令牌的非HTTP IPC，顺序有界处理。两项受限命令仅主窗口可用，不给WebView通用网络、文件、shell或SQL权限。
- storage/clips.rs：沿用DocumentFiles和markdown_documents，schema v3/001–003迁移不变。受管目录、请求摘要、禁止覆盖、事务登记；重复同ID返回已有，内容冲突拒绝，文件提交后SQL失败可由同请求恢复。原图及已有资料不修改。
- 前端features/clipper复用现有Markdown会话、脏文档确认和项目列表；设置卡片、剪藏通知、深浅主题、13张离线图/14章指南。文件与入口详见[通信契约](../../browser-extension/PROTOCOL.md)、[安装说明](../../browser-extension/README.md)。

## 已实际执行

| 检查 | 实际结果 |
| --- | --- |
| npm run check（初步最终check-third） | PASS/0；45文件308项，类型/ESLint/生产前端构建；后续收尾检查另记 |
| npm run test:rust | PASS/0；135项，0忽略；本轮新增存储/SQL恢复/路径测试 |
| 独立Cargo workspace test | PASS/0；3项协议测试，Host可执行程序编译 |
| npm run check:rust及Host Clippy | PASS/0，全部目标-D warnings |
| browser-extension npm run check | 最终PASS/0；5项Node测试、TS与popup/content两份生产构建 |
| node scripts/clipper-browser-smoke.mjs v102-lists | PASS/0，真实Edge DOM5组，中英文/列表/引用/代码/来源/选区/图片及表单和恶意HTML |
| node scripts/clipper-native-smoke.mjs v102-lifecycle | PASS/0，真实Host→带令牌IPC→Tauri/SQLite/文件7组，无Mock响应 |
| unpacked Edge加载探针 | PASS/0，真实chrome-extension固定origin可加载；不是activeTab端到端验收 |
| node scripts/clipper-extension-smoke.mjs 各次 | 整套FAIL：真实MV3 runtime.connectNative读取SQLite项目列表完成1组；后续自动action popup等待超时，没有把已失败整套标PASS |
| Windows工具栏UI复验 | BLOCKED：Computer Use无法可靠确认浏览器URL，已停止输入，没有换工具绕过该检查 |

原生7组涵盖：未启动/来源拒绝、显式开启/伪令牌拒绝、真实提取正文保存到项目/重复ID/内容冲突、分栏重开、PNG字节及相对assets、脏文档取消与明确放弃、修改保存/关闭/进程重启。安装脚本只临时创建经预检的本目录HKCU项，退出后注销；不覆盖用户已有注册。

## 首轮FAIL与修正

1. 首次npm安装触发npm内部edgesOut异常；缩减不必要的测试依赖，改用Node内建测试及官方registry后安装/独立构建通过，未重置主项目或资料。
2. Rust初编译发现错误枚举名与CommandError字段可见性，TS发现库筛选字段/会话恢复类型，ESLint未用导入及控制字符正则，Clippy复杂返回类型；均修正后重跑，不关闭规则掩盖。
3. 浏览器夹具缺UTF-8及对列表空白的过窄断言，原提取内容符合Markdown；修正夹具与语义空白断言后5组PASS。
4. 原生测试首次“打开Markdown”重名按钮、错误的直接退出确认调用导致隔离实例残留；定位到通知按钮，改用现有正常退出请求，7组PASS，未强杀用户应用或删除锁。
5. Host注册清单原来按JSON字符串顺序比较，重复注册错误拒绝；修正为严格字段/类型/路径/origin语义比较，仍拒绝其他安装，实际浏览器Host项目读取复验通过。
6. 完整真实工具栏用户点击与activeTab提取仍未执行完成。自动popup等待失败和Windows工具URL安全阻挡均保留，不能用直连Host或DOM脚本替代。

## 新依赖与限制

Readability0.6.0 Apache-2.0、DOMPurify3.4.17 MPL-2.0 OR Apache-2.0、Turndown7.2.4 MIT，版本/API/许可证核实并离线打包。Rust共享协议直接使用已有锁定的serde/json、url2.5.8、base640.22.1、sha20.10.9、getrandom0.4.3；没有下载OCR/AI模型。扩展包附第三方许可证。

正文192 KiB、消息预检700 KiB/Host768 KiB、32图引用，本地复制最多4图、单PNG256 KiB及419万像素/合计384 KiB。只匿名尝试同源HTTPS PNG/JPEG/WebP；其他图保留来源/失败状态，不绕过访问控制。浏览器解码有额外瞬时内存，不声称固定进程内存。配置令牌的保护依赖当前用户目录ACL，不防御拥有相同用户权限的恶意本地进程。取消等待不撤销已接收的保存；重试同一请求可确认。

## Windows人工验收清单（NOT_RUN）

1. 用Release扩展包在Chrome、Edge分别按README注册和手动加载，再由真实工具栏图标触发四种模式；核对activeTab授权和目标页未切换。
2. 普通中英文文章、技术代码文档、复杂网站/跨域frame/选中文字；表单和浏览器内部页明确失败，不尝试绕过登录/付费墙。
3. 图片勾选、多图关联、同源HTTPS复制、Cookie省略、跨域/损坏/超限/无访问权限的失败来源。真实浏览器下载链尚未完整验收；底层PNG保存/重开已测。
4. 关闭应用、关闭连接、断开Host/关闭popup再开、重复保存，验证同ID无重复；改内容应新建。已接收请求取消后的状态需重试确认。
5. 保存到项目→打开右分栏→遇已有脏文档取消→修改注释Ctrl+S→重启→另存为并移动整个.md/assets目录，在其他Markdown软件检查。
6. 干净Windows安装/升级、Host目录迁移及与已有安装注册冲突、长期内存/句柄、Chrome不同版本。macOS端到端NOT_IMPLEMENTED。历史QA-001/002及QA-008卸载间歇问题不据此关闭。

## 收尾记录

Windows生产EXE及NSIS构建PASS/0；隔离NSIS安装/繁忙保护/默认卸载PASS/0，7份既有用户资料摘要保持不变。远程发布另附回执。完整工具栏UI验收受阻，发布属性不改变该状态。没有以删除数据、重置SQLite或降低断言制造成功。

收尾检查：独立扩展干净npm ci及5项Node测试/两份构建PASS；桌面45文件308项检查PASS，Rust135项与协议3项PASS，严格Clippy PASS。真实Host恶意帧/未知字段/未启动/10秒超时5组PASS。记录原始FAIL，Windows工具栏测试仍BLOCKED。可公开[摘要](evidence/v102-acceptance.json)不含令牌、页面正文或用户文件。生产构建/发布结果另补。

最终命令：npm run lint、npm run check:docs（64文档389链接）、npm run desktop:bundle、node scripts/clipper-host-smoke.mjs v102-release --release、pwsh -File scripts/installer-smoke.ps1 -RunId v102-final -TargetDir src-tauri/target均退出0。Release Host实际5组协议/超时检查PASS。扩展交付ZIP含57份依赖许可条目；压缩工具将早于1980的LICENSE时间戳规范化，文件字节保留。安装成功不关闭QA-008历史间歇问题。

GitHub正式Latest发布与5附件匿名下载校验PASS；12个旧Release保持不变，见[发布回执](PUBLICATION-1.0.2.md)。

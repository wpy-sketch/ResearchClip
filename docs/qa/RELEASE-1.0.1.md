# v1.0.1 浏览器滚动长截图实际验收

日期：2026-10-18。范围：显式单窗口、单屏矩形区域、手动拼接及普通浏览器自动滚动MVP；复用原截图、PNG、素材库和编辑器。没有实现多应用自动化、PDF或独立备份系统。

**306项前端、132项Rust、Windows构建、滚动专项9组及列出的回归最终PASS。真实十屏像素用例PASS，但其所在整套脚本随后因旧按钮定位失败，整套仍为FAIL。安装最终两个全新目录复验PASS，初期间歇卸载FAIL保留且根因未确认。不能称全套硬件及长期稳定性验收通过。**

## 环境与数据

- Windows11 x64 10.0.26200；Node22.22.2/npm10.9.7、Rust MSVC1.99、Tauri2.12.3/CLI2.12.2；真实Edge/GDI/WebView2/系统对话框/SQLite。
- 原生测试使用Tauri CLI构建的debug EXE、生产前端及隔离资料。生产EXE另做构建、安装载荷与版本核验，没有启动它写入真实用户资料。
- 主屏实际100% 2560×1440；枚举到负坐标副屏2560×1600。125%/150%只有纯坐标测试，不是实屏验收。
- schema仍为3，001/002/003迁移及标注格式不变，无新增依赖/上传服务。两个锁文件仅改变项目自身版本。
- 统一五态：PASS实际执行且通过；FAIL实际执行且失败；BLOCKED环境受阻；NOT_RUN未执行；NOT_IMPLEMENTED未实现。代码和Mock不替代原生采集。
- 测试仅写新建artifacts隔离目录。安装前后只读核验7份既有资料摘要；不删锁文件、不重置AppData、不强制终止用户进程。

## 修改文件、架构与实现状态

| 模块 | 文件/关键入口（Rust相对于src-tauri/src，前端相对于src） | 实现与验证 |
| --- | --- | --- |
| 拼接 | services/stitching.rs：StitchingEngine、estimate_offset、append、frame_difference | 已实现；粗匹配/密集校验、低纹理/歧义拒绝、重复识别和增量底部接入；纯图像及真实屏幕样本PASS |
| 会话/坐标 | services/scrolling.rs：ScrollService、Session、validate_region | 已实现；有限目标/单会话、CapturePermit、专属窗口授权；边界/取消/门控测试PASS |
| 原生采集 | services/scrolling_windows.rs：list、validate、activate、visible、cursor、wheel | 已实现；Edge真实PASS，Chrome实测NOT_RUN；macOS NOT_IMPLEMENTED |
| IPC/worker | commands/scrolling.rs：scroll_targets/prepare/start/status/action、stable、run | 已实现；启停/手动/自动/失焦/取消/保存重开PASS |
| 原区域复用 | commands/region.rs：select_region、run_region_capture | 冻结后才显示覆盖层，捕获门控贯穿移交；原生21组回归PASS |
| 前端 | features/capture/ScrollCapture.tsx、scroll-api.ts、scroll-store.ts、scroll.test.ts | 设置弹层/独立控制窗、加载/暂停/失败/恢复；6项新测试及真实UI通过 |
| 接入 | App.tsx、main.tsx、CaptureControls/CapturePanel、library-actions/open-editor、shared/api及types/capture、styles.css | 复用CaptureWorkflow、PNG、SQLite、既有编辑器；截图类型增加scroll |
| 生命周期/权限 | lib.rs、build.rs、commands/services的mod.rs、services/desktop.rs；capabilities/main.json、scroll-control.json、tauri.conf.json | 控制窗仅status/action，无通用磁盘/输入/创建窗口授权；隐藏/退出取消 |
| 字体 | vite.config.ts | 强制小字体为本地文件，修复原有font-src self拒绝data字体；CSP不放宽 |
| 验收脚本 | scripts/scrolling-native-smoke.mjs、smoke.mjs、guide-native-smoke.mjs、check-docs.mjs、installer-smoke.ps1、installer-quiescence.ps1 | 像素断言、旧入口修正、只读占用前置检查；未删失败断言 |
| 文档/版本 | README、CHANGELOG、PRD、ARCHITECTURE、ROADMAP、USER_GUIDE、GuideView、docs/images/v101-scrolling.png、QA报告/计划/证据及发布说明；package/Cargo两个清单/锁文件 | v1.0.1，指南14章/11张离线图；完整名单见本次Git提交 |

链路：显式选窗口→冻结屏幕并复用区域覆盖层→有界worker稳定采集→重叠校验/增量接入→原有完成区→PNG/素材库/编辑器。控制窗采集时隐藏，终态销毁并恢复主窗；只保留合成图和上一帧，不缓存所有原始帧。

自动模式仅对chrome.exe/msedge.exe及符合的窗口类开放。每次滚轮前检查PID/HWND、前台、可见区域、鼠标和修饰键；失焦/遮挡/移出暂停并提供手动回退。不用后台PostMessage，不绕过受保护截图。检查与SendInput并非原子操作，请勿自动时切换窗口。顶栏匹配排除区可保留导航一次；底部/复杂浮层需框选排除或手动分段，不保证所有网页。

上限同时生效：64接入帧、180秒、30,000像素高、16,777,216总像素；单帧最多4,194,304像素、宽≤4096。两次重复只是疑似到底/滚动无效；重叠不足、低纹理、歧义报错，不完整堆帧。采用已有Rust像素/PNG管线；OpenCV的原生库及Clang增加打包复杂度，本阶段未引入。

核对来源：[SendInput](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendinput)、[RmGetList](https://learn.microsoft.com/en-us/windows/win32/api/restartmanager/nf-restartmanager-rmgetlist)、[NSIS临时卸载子进程](https://nsis.sourceforge.io/Docs/AppendixD.html)。父进程退出0不证明卸载成功，仍核验文件和注册项。

## 实际命令与结果

Windows原生/文件检查获得必要沙箱权限后执行。原始日志保留于本地artifacts/qa-v101；可公开摘要见[验收证据](evidence/v101-acceptance.json)。

| 命令 | 实际结果 |
| --- | --- |
| npm ci --proxy http://127.0.0.1:7897 | PASS/0，303包干净安装 |
| npm run check | PASS/0，类型/ESLint/Vitest 44文件306项/生产前端构建 |
| npm run format:rust | PASS/0 |
| npm run test:rust | PASS/0，132项，0忽略 |
| npm run check:rust | PASS/0，all-targets Clippy -D warnings |
| npm run tauri -- build --debug --no-bundle | PASS/0，原生测试程序；普通cargo build后devUrl程序不能代替 |
| node scripts/scrolling-native-smoke.mjs --run-id v101-browser-2 | 整套FAIL/1，手动3屏及自动10屏像素用例PASS，随后旧矩形按钮定位超时 |
| node scripts/scrolling-native-smoke.mjs --short --run-id v101-steady | PASS/0，9组；autoScreens=3，不能写成10屏 |
| node scripts/scrolling-native-smoke.mjs --setup-only --run-id v101-guide | PASS/0，1组健康设置与实际指南图；未执行10屏采集 |
| node scripts/smoke.mjs --desktop --region-only --run-id v101-region-verified | PASS/0，21组，真实冻结/裁切/连续20次/重启/权限 |
| node scripts/guide-native-smoke.mjs --verify-only --run-id v101-regression | PASS/0，5组；11图解码、Enter/Esc、14章导航/窄窗 |
| node scripts/split-native-smoke.mjs --run-id v101-regression | PASS/0，11组；系统保存/重开/分隔线与画布坐标 |
| node scripts/editor-native-smoke.mjs --run-id v101-regression | PASS/0，6组；原工具、真实剪贴板跨Edge像素比对/PNG/重启 |
| node scripts/math-native-smoke.mjs --run-id v101-font-fixed | PASS/0，8组；离线字体/安全/源码原样保存重开 |
| node scripts/material-ui-smoke.mjs --run-id v101-regression | PASS/0，8组；仅Edge布局，480–1440深浅色，不计原生采集 |
| npm run desktop:bundle | PASS/0，最终Windows release与NSIS |
| pwsh -NoProfile -File scripts/installer-smoke.ps1 -RunId v101-quiescent -TargetDir src-tauri/target | PASS/0，4组；安装/默认卸载、占用拒绝、元数据、7资料摘要保留 |
| 同命令 -RunId v101-quiescent-repeat | PASS/0，另一全新目录4组，默认系统TEMP；未绕过保护 |
| npm run lint、git diff --check | PASS/0，最后脚本改动后重新执行 |
| npm run check:docs | PASS/0，59份文档、369个本地链接；权限与schema v3/既有迁移一致 |

### 真实长图证据

1. 手动3屏720×1848：16,568抽样RGBA通道，0差异；重复帧不增高度。
2. 自动10屏720×6195：58帧、55,176抽样通道，0差异；固定顶部一次、末尾重复停止。这是自有确定画布浏览器夹具的真实GDI/输入采集，不是注入假帧；不是全图逐像素比较。
3. 最终完整3屏专项：自动720×1848、16,264通道0差异。另对普通HTML标题、变长段落及固定导航完成实际自动拼接。
4. 系统对话框实际保存PNG并读回尺寸；长图创建可编辑矩形→SQLite→退出重启→重开原图和标注。受管原图与保存PNG字节相同。
5. 重叠不足暂停、取消释放门控、失焦后1200ms无继续滚动、设置取消无新结果，实际通过。

## FAIL、修正及残余问题

| 项目 | 复现与结果 | 处置/优先级 |
| --- | --- | --- |
| 沙箱文件限制 | Rust首轮70过61失败，原子提交被拒；Vitest临时模块ENOENT，28套失败/15套过 | BLOCKED于该上下文；Windows正常权限完整132/306 PASS，未弱化断言 |
| 前台激活 | 已是前台目标但SetForegroundWindow返回0，首轮拒绝/覆盖层等待超时 | P1已修复：已在目标前台直接接受；后续仍验证焦点，原生选区PASS |
| 测试夹具/入口 | devUrl等待；参考坐标漏4px非客户边；旧矩形按钮定位；设置缺viewMode/font字段 | 修正构建/几何/定位/夹具并保留首轮FAIL；健康设置单独PASS，早期9组曾使用默认回退配置 |
| 安全暂停 | 两轮鼠标离开ROI，等待完成的脚本FAIL | 保留FAIL，守卫符合预期；用户确认暂不操作后完整9组PASS，未禁用守卫 |
| 旧区域脚本 | 指南独立页面、旧入口及只读命令白名单落后 | 修正入口及三个明确只读命令，最终21组PASS，不放开任意IPC |
| 小字体CSP | 内联KaTeX Size3字体被font-src self拒，零错误断言FAIL | P2已修复本地字体文件；真实数学8组和区域21组PASS |
| 默认卸载间歇 | final/confirmed/private-temp/direct-repeat在30秒内未清除文件/注册项，父进程0 | **QA-008/P2 OPEN待归因**；受控重试可卸载且7资料摘要相同。直接启动、只读占用=0前置条件后两个新目录默认/S PASS；不宣称产品根因修复，不手动删程序冒充卸载 |
| 历史退出 | QA-001/P2 PostMessage 0x80070578，本轮未执行原素材库专用退出复现 | 保留OPEN；本轮其他专项无异常不能关闭历史问题。QA-002托盘间歇问题同样保留 |

卸载WMI子进程退出追踪因访问拒绝BLOCKED，未绕过到管理员。私有TEMP成功不替代默认TEMP复验；新增静默前置检查没有观测到非零占用码，故“残留占用导致失败”仍是假设，不能当根因。未观察到新增P0/P1或资料丢失；安装间歇问题需干净系统继续跟踪。

## Windows人工验收（均NOT_RUN，已列自动原生证据除外）

1. **125%/150%实屏**：普通网页从页顶框正文，手动3屏及自动到尾，核对物理尺寸/接缝/末行。混合DPI及负坐标副屏单独验收，单元坐标PASS不能替代。
2. **Chrome及网页**：Chrome/Edge不同页面，固定顶部/底部、浮层、动画、懒加载、无限列表、重复段落。排除浮层后仍无法匹配必须暂停，不能静默拼错。
3. **输入安全**：切窗、遮挡、移动/最小化目标、鼠标移出、修饰键，检查暂停和其他窗口无滚动；受保护内容明确拒绝，不绕过。
4. **上限/资源**：180秒、64帧/像素限制、快速启停取消、采集中隐藏/退出主窗、反复重开、句柄/内存长稳。单次有界测试不等于长期泄漏检查。
5. **生产链路**：最终EXE真实科研网页→PNG→素材库→标注撤销/重开→Markdown；逐段放大检查接缝和文字，不以夹具像素测试泛化所有网页。
6. **安装/退出**：干净Windows10/11、交互卸载、旧版升级、缺WebView2/SmartScreen、默认TEMP重复卸载；失败保留现场。未执行用户勾选删除资料的破坏性分支。
7. **平台**：macOS本功能NOT_IMPLEMENTED；其他架构/HDR/长期工作NOT_RUN；不承诺任意应用、受保护内容适配。

## 产物与阶段结论

- `src-tauri/target/release/researchclip.exe`：20,113,920字节，1.0.1，SHA256 `eed8912073448f3d41a4489b137e672a57921319dfd8edcc2fc41eed1795c164`。
- `src-tauri/target/release/bundle/nsis/ResearchClip_1.0.1_x64-setup.exe`：6,396,745字节，SHA256 `b6ebd9876dd69ebbf15d5830aee2fe0265f52f98d0c5c5534ca7f0b97717b2af`；未签名，需WebView2。
- 核心MVP及最终必要构建通过，QA-001/002/008和未完成矩阵公开保留。按用户授权准备非预发布v1.0.1，不声称完整平台认证；发布结果随后单独附回执。
- 本阶段完成后停止，不自动开发下一项Pro功能。

发布续验：普通Latest及四份匿名下载校验已通过，见[发布回执](PUBLICATION-1.0.1.md)。本轮安装问题更正为QA-008；源码标签及SOURCE.txt中的QA-003为早期编号重复，程序与附件不变。

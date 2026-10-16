# Windows 区域截图实现与验收

初次记录：2026-10-12；补充原生验收：2026-10-13，Asia/Hong_Kong。本轮只实现区域选择、冻结帧裁剪及临时 PNG；没有热键、编辑器、卡片或数据库改动。

**当前结果：记录的 Windows 主显示器 2560×1440 / 100% 区域原生验收通过，包含 20 个连续会话和应用重启。** 具体命令与证据见文末 2026-10-13 补充。下文保留 2026-10-12 的 BitBlt / PERMISSION_DENIED 历史失败及当时受阻结论；本次不再复现，但不能据此确定早先拒绝的原因，也不代表全部显示器/DPI配置已验证。

## 实现与文件

| 文件 | 本轮责任 |
| --- | --- |
| src/features/capture/CaptureOverlay.tsx、geometry.ts/test.ts | 独立入口、冻结图、四块蒙层、pointer capture、实时像素尺寸、零面积重选、Esc、坐标数学 |
| src/main.tsx、styles.css | 按会话 query 选择独立渲染入口；覆盖窗口布局，不挂载主界面 |
| src/shared/api/region-overlay.ts、capture.ts/test.ts | 会话/结果 DTO 校验、一次性冻结 PNG、受限结果读取与 Blob 生命周期 |
| src/features/capture/store.ts/test.ts、CaptureControls/Panel.tsx | 区域入口、等待/错误/取消保留旧预览、结果尺寸和真实保存路径 |
| src/app/layout/WorkspaceDialogs.tsx、src/features/home/HomeView.tsx | 接通已有截图入口，更新可用范围说明 |
| src-tauri/src/capture/geometry.rs | 权威逻辑→物理选区校验，原帧逐行原地裁剪及精确像素/PNG 测试 |
| src-tauri/src/services/region_capture.rs | 有界单会话、窗口绑定、终态通道、唯一 PNG 禁止覆盖落盘、最新资源 ID |
| src-tauri/src/commands/region.rs | 冻结前隐藏、动态原生窗口、物理定位/大小、焦点/配置/超时/销毁/恢复编排 |
| src-tauri/src/commands/capture.rs、services/screen_capture.rs、capture/windows.rs | 复用隐藏恢复 guard、有界 PNG 编码、补充访问拒绝的失败步骤 |
| src-tauri/src/lib.rs、build.rs、模块入口、capabilities、tauri.conf.json | 注册状态/命令、主窗口关闭取消、主窗口/覆盖层最小权限 |
| src-tauri/Cargo.toml | tempfile 3.27.0 从已有 dev dependency 移到生产依赖；锁文件根关系原已包含，无版本升级 |
| scripts/smoke.mjs、overlay-smoke.mjs、check-docs.mjs、package.json | 真桌面/模拟 IPC 前端交互检查、文档/权限一致性检查及命令入口 |
| README、ARCHITECTURE、ROADMAP、本文件 | 当前行为、契约、阶段状态与受阻项 |

ScreenCaptureService 的 RAII 许可覆盖整次区域会话；RegionCaptureService 最多一个活动会话和一个最新结果路径，不持有无边界全局帧列表。冻结帧从现有 Win32 GDI 取得，随后才创建 capture-overlay-{sessionId}，用户松开后直接裁剪原帧，禁止重新拍屏。

覆盖层初始隐藏、无边框/阴影、置顶、不进任务栏、不可缩放；使用 PhysicalPosition 的带符号原点和 PhysicalSize 的物理客户区。前端 PNG 解码/布局完成后通过 ready 命令，Rust 校验尺寸、位置、窗口 scale_factor 与 CSS 客户区，再显示并聚焦。仅核对已安装 Tauri 2.12.3 的真实 API 并编译，不新增截图库或模拟屏幕获取。

映射使用 xRatio=frameWidth/clientWidth、yRatio=frameHeight/clientHeight；正/反拖动先归一化，左/上 floor、右/下 ceil，然后夹到图像范围。全局负原点只用于原生窗口定位，不加到裁剪局部坐标。像素尺寸提示采用同一规则，Rust 重新验证实际窗口位置、尺寸和固定客户区值。像素/PNG 上限沿用 16,777,216 像素 / 65 MiB。

零面积不提交，保持选择层允许重选；Esc/失焦/窗口关闭取消，主窗口关闭也通知取消。移动、尺寸、DPI 或目标显示器配置变化终止。15 秒准备预算、5 分钟选择预算；显示器约每秒复核。所有终态清除会话/销毁覆盖层并尝试恢复原有主窗口焦点；清理失败如实报告。同步 GDI 调用仍不能强制中断。

成功图像写到 `%TEMP%/ResearchClip/region-captures/ResearchClip-region-<unique>.png`：同目录 tempfile、write/sync、persist_noclobber。冲突/磁盘/权限错误不覆盖已有文件，暂存失败自动释放本次临时文件。已成功 PNG 保留；关闭预览或取消下一次截图不会删除它。临时目录可能被系统清理，长期保留需复制文件。本轮没有资料入库、SQLite 事务或编辑器，只有 PNG 和一张会话预览。

main 只能发起捕获和按最新 captureId 读取结果；overlay 只授权 info/image/ready/commit/cancel，Rust 再验证 sessionId+窗口标签。PNG 响应是 ArrayBuffer，无 base64/JSON 像素数组。没有 fs 插件、任意路径读写、额外 asset 范围、上传或普通像素日志。数据库/storage/迁移和用户 AppData 未改动。

## 2026-10-12 实际命令与结果（历史）

Windows x64；Node 22.22.2、npm 10.9.7、项目内 Rust MSVC 1.99.0、Tauri 2.12.3；React 19.3.0、Vite 8.3.4、TypeScript 6.0.3、Tailwind 4.3.3、Vitest 5.0.3。复用安装依赖，未升级 npm 包；没有全量冷缓存重建。

| 命令 | 实际结果 | 退出码 |
| --- | --- | --- |
| npm ls --depth=0 | 已安装依赖核对成功 | 0 |
| cargo fetch --manifest-path src-tauri/Cargo.toml --locked --offline --target x86_64-pc-windows-msvc（通过 with-rust） | 已有锁文件及离线缓存可解析 | 0 |
| npm run check | TypeScript、ESLint、7 文件 66 项 Vitest、生产构建通过 | 0 |
| npm run test:rust | 22 项通过；包含原有 14 项和本轮 8 项 | 0 |
| npm run format:rust | rustfmt 检查通过 | 0 |
| npm run check:rust | all-targets Clippy、-D warnings 通过 | 0 |
| npm run desktop:build | Windows release exe / no-bundle 成功 | 0 |
| npm run test:smoke | 6 组主界面检查（含 32 个布局组合）、4 个模拟 IPC 选择层缩放场景，无 renderer 错误 | 0 |
| npm run check:docs | 文档链接/围栏、命令权限、无 fs 授权、storage/迁移未改动与 git diff --check 通过 | 0 |
| npm run test:smoke -- --desktop | 应用真实启动、4 组基础界面/通信检查后，屏幕访问拒绝；整体 FAILED，区域正常流程/重启未执行 | 1 |
| cargo run --manifest-path src-tauri/Cargo.toml --locked --example capture_probe -- 1（通过 with-rust） | 真实枚举成功，BitBlt 访问拒绝；未完成一张捕获 | 1 |

Rust 坐标测试覆盖 100/125/150/200%、反向拖动、向外取整、边界夹取、非有限/无效值与零面积；裁剪测试验证逐像素行数据、完整帧/单像素、PNG 编解码实际尺寸、不完整缓冲/溢出/越界。会话测试覆盖窗口隔离、一次性预览、并发拒绝、准备前/重复提交、旧回调、主窗口关闭和 PNG 冲突/路径逃逸。前端测试本轮新增 14 项，覆盖坐标、取消/错误保留旧图、并发保护与元数据/受限结果读取。

模拟 IPC 的浏览器检查使用显式生成的测试 PNG，标记 `renderer-only-mock` 和 `nativeCaptureVerified:false`；检查独立入口、一次性加载、零面积、反向拖动、像素提示、四块蒙层、一次提交、提交后不晚取消和 Esc。浏览器 deviceScaleFactor=1/1.25/1.5/2 **不是实际 Windows DPI 验收**，也未验证原生聚焦、窗口边界/销毁或真实屏幕裁剪。

## 2026-10-12 原生受阻证据（历史）

| 显示器 | 本轮枚举结果 | 捕获状态 |
| --- | --- | --- |
| DISPLAY2 主屏 | 2560×1440，原点 (0,0) | BitBlt 失败，映射 PERMISSION_DENIED |
| DISPLAY1 次屏 | 2560×1600，原点 (-2560,0) | 仅枚举，没有测试其捕获 |

真实 exe 启动、版本、导航/主题、Rust test_connection 和显示器枚举可工作。主屏捕获报“无法捕获显示器画面…Windows 拒绝访问”，没有 renderer 错误。独立原生探针复现，故不能仅归因于前端选择逻辑。具体是锁屏、安全桌面、进程权限或其他桌面限制，当前证据无法确定；未尝试绕过系统保护。

在失败后的真实 main 窗口调用 capture_region，实际收到 PERMISSION_DENIED；返回时 mainFocused=true、overlayCount=0。说明这一失败发生在覆盖层创建之前，主窗口恢复焦点；这不能证明选择层创建后清理分支已经真机通过。完整测试保留 FAILED 状态，没有削弱成功断言或用假图替代原生流程。

证据：artifacts/region-capture-desktop-smoke.json（FAILED 与 regionAccessDiagnostic）、region-capture-desktop-failure.png、region-capture-preview-smoke.json（包含明确 mock 标识）、region-capture-preview-overlay-ui-mock.png。独立 WebView2 profile 为 artifacts/region-capture-webview-profile；TEMP/TMP 测试根为 artifacts/region-capture-native-temp，未重置真实资料目录。旧全屏、主界面和 M1 证据保留。

测试过程中修复 Clippy 条件合并、共享 PNG 编码接口、取消误触发“成功捕获”导航，以及访问拒绝提示丢失具体步骤；一次 String/&str 编译错误修复后重跑通过。没有把中间失败写成通过。

## 2026-10-12 人工验收清单与下一步（历史）

```powershell
npm run desktop:dev
# 或构建后运行 src-tauri/target/release/researchclip.exe
npm run test:smoke -- --desktop
```

请在已解锁的普通 Windows 桌面选择主显示器，点击“区域截图”：

1. 确认覆盖层无边框、完整覆盖屏幕、置顶并可立即接收 Esc；底图没有 ResearchClip 主窗口/选择工具。
2. 在选择期间改变底层内容，结果必须保持开始时的冻结画面；PNG 不含蒙层、边框和尺寸提示。
3. 正向/反向拖动与各边缘、单像素、越界拖动；核对物理像素提示、PNG 实际尺寸及内容，零面积后可继续选择。
4. Esc/Alt+Tab 失焦/关闭覆盖层/异常后主窗口恢复，原有预览保留；取消不生成新 PNG，连续至少 20 次无残留置顶窗口，随后全屏仍可用。
5. 查看预览显示的临时 PNG 路径，确认文件存在、能被外部图片查看器打开；重复截图不覆盖旧文件，重启后旧文件仍在但不会伪装为素材卡片。
6. 分别检查 Windows 100/125/150/200%、混合 DPI、左/上负原点目标屏、旋转、任务栏位置；接口支持带符号原点，当前不支持跨屏选区，未宣称其他显示器配置均通过。
7. 在独立测试环境检查显示器插拔/分辨率变化、文件权限/磁盘空间、锁屏、远程桌面、HDR/受保护画面及加载超时；记录分类错误、恢复及残留窗口。不得破坏用户资料来制造故障。

下一步建议先解决当前屏幕访问拒绝，在普通桌面复跑完整原生脚本并完成人工矩阵。当前原生正常路径未验收属于未解决阻塞，不进入热键、编辑器或数据库阶段。本轮完成代码与可执行检查后停止。

## 2026-10-13 补充原生验收（当前结果）

用户确认桌面已解锁。复跑 capture_probe -- 1 成功：主屏 2560×1440、GDI 对象计数 1→1，非法显示器拒绝；早先 PERMISSION_DENIED 本次没有再现，原因仍未确定，不能归因于锁屏。

本次沿用已有区域实现，生产代码只补充窗口关闭与失焦的具体取消消息，没有改变截图来源、坐标规则或安全权限。测试辅助与记录改进包括：

- scripts/smoke.mjs 支持 run-id，为每轮证据、WebView2 profile 与 PNG 目录增加后缀，保留旧失败记录。
- 新增仅供测试的 examples/raise_capture_fixture.rs：核对测试浏览器 PID 与固定标题，只把自己的色块窗口置顶，不修改 ResearchClip 或其他窗口。等待原生标题就绪，避免初始化竞态。
- PNG 文件与预览比较改为完整解码像素，避免 fetch(blob:) 被当前 connect-src 拒绝；没有放宽 CSP。
- 裁剪区域的全部 RGBA 像素计算 SHA-256，与开始时原生冻结帧的对应矩形比较；这比只检查五个采样点覆盖更多内容。
- 主窗口到 overlay 的显式越权请求已实际拒绝；命令观察单独允许记录这一次负向探针，普通 UI 命令列表继续严格限制。
- 增加区域专用范围 --region-only，完整全屏/区域色块套件仍保留。专用范围验证真实冻结帧、裁剪/文件/预览和生命周期，不宣称已执行完整套件的色块断言。
- package.json 新增 test:smoke:desktop，先编译辅助程序，再运行原生测试并转发范围参数。

Windows NT 10.0.26200.0 / x64；主显示器 DISPLAY2 2560×1440、原点 (0,0)，实际 WebView2 客户区 2560×1440、devicePixelRatio=1。次显示器 DISPLAY1 2560×1600、原点 (-2560,0) 仍仅枚举，没有验收其选区。工具链和依赖版本沿用上节，没有新增或升级包。

| 本次命令 | 实际结果 | 退出码 |
| --- | --- | --- |
| capture_probe -- 1（同上 manifest/with-rust） | 原生读取成功，GDI 1→1 | 0 |
| npm run check | 类型、ESLint、66 项测试、前端生产构建通过 | 0 |
| npm run test:rust | 22 项通过 | 0 |
| npm run format:rust / npm run check:rust | rustfmt / all-targets 严格 Clippy 通过 | 0 |
| npm run desktop:build | 修改后的 Windows release exe 构建成功 | 0 |
| cargo build --locked --example raise_capture_fixture（同上 manifest/with-rust） | 测试辅助程序真实编译成功 | 0 |
| npm run test:smoke -- --desktop --region-only --run-id 2026-10-13-region2 | 21 组原生检查、启动/重启通过，无 renderer 错误 | 0 |
| npm run test:smoke:desktop -- --region-only --run-id 2026-10-13-final | 新入口编译/转发参数与相同 21 组检查通过，无 renderer 错误 | 0 |

真实区域检查验证：覆盖窗口加载、客户区物理尺寸、键盘焦点、实时尺寸提示；1280×720 的原生冻结帧裁剪与全部像素 SHA-256 一致；保存文件与预览完整解码像素一致。改变底层色块后再提交，输出仍来自开始时冻结帧，PNG 不含本选择层的蒙层、边框或尺寸提示。零面积保持窗口、反向拖动输出 200×160、向左/上越界夹取输出 50×50；Esc 后旧预览保留、无新文件、覆盖窗口销毁、主窗口恢复焦点。

额外连续 20 个会话中 10 次保存、10 次 Esc；每次 sessionId 更新、仅一个 overlay 存在、结束后无 overlay target、主窗口恢复焦点。重启复跑正常/零面积/反向/Esc/越界与权限用例。合计 28 个区域会话，16 张唯一 PNG、12 次取消；已有文件没有覆盖。没有实际测量 RAM/全部 GUI 句柄增长，因此不把窗口检查说成完整资源泄漏证明。

最终证据保存在 artifacts/region-capture-desktop-2026-10-13-final-smoke.json（PASSED）、对应 selection/region/restart PNG。profile 与 PNG 根分别为同前缀的 -webview-profile / -native-temp。region2 的独立成功证据也保留。没有重置真实 AppData，未运行数据库业务或修改 storage/迁移。

本次失败记录仍保留：check1/2/3 的测试色块被固定窗口遮挡；用户确认存在旧选择层并按 Esc 取消后，check4/5 的两次全屏九点校验通过。check4 区域 PNG 已保存后遇到测试脚本的 CSP Blob fetch 失败；check5 的第二层在拖动前关闭，未确定触发来源，后续连续会话未重现。check6 为原生标题更新竞态，check7 再遇色块遮挡，check8 为测试辅助请求激活窗口被 Windows 拒绝；未绕过系统前台限制。[SetForegroundWindow 规则](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setforegroundwindow)。region1 全部区域用例完成后，观察器把刻意拒绝的越权探针误判为意外命令，修正精确计数后通过。**本次完整全屏/区域套件没有一次最终整体通过，不将其标记为通过；区域专用原生验收已通过。**

后续可运行：

```powershell
npm run desktop:dev
npm run test:smoke:desktop -- --region-only --run-id another-region-run
```

仍需人工验证真实鼠标/键盘操作、Alt+Tab 与系统菜单失焦、关闭主窗口/覆盖层、100% 以外实际 DPI、混合 DPI、负原点目标显示器、右/下边缘、旋转/插拔、远程桌面、HDR/受保护内容、磁盘权限/空间与超时。专用脚本使用真实 Windows 图像与原生窗口，指针/键盘由 CDP 驱动；它不代替所有物理输入和硬件矩阵。当前记录配置没有剩余区域正常路径阻塞；完整套件受桌面遮挡/焦点环境影响的限制保留。下一步建议完成以上人工矩阵，本轮停止，不实现热键、编辑器或数据库。

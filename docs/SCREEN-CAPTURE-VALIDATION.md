# Windows 全屏截图模块验收

日期：2026-10-12，Asia/Hong_Kong。范围：显示器枚举、指定显示器全屏捕获、实际像素尺寸与临时预览。**记录环境的主显示器验收通过**；没有实现区域、全局快捷键、截图保存、数据库或编辑业务，不能视为完整M2/MVP完成。

## 方案核对与选择

| 方案 | 核对结果 | 本轮决定 |
| --- | --- | --- |
| xcap 0.9.8 | cargo info下载发布源码，Apache-2.0；Monitor::all/id/name/width/height/is_primary/capture_image真实存在，返回Result，capture_image返回RGBA图像；默认Windows后端是GDI，wgc为可选特性 | 未引入依赖 |
| windows-rs 0.62.2 + Win32 GDI | 微软官方绑定，MIT/Apache-2.0；已在项目锁文件中，直接绑定EnumDisplayMonitors/GetMonitorInfoW/EnumDisplaySettingsW/BitBlt/GetDIBits等API；本轮实际编译/捕获通过 | 采用独立小型Windows适配器 |
| DXGI Desktop Duplication / WGC | 适合GPU帧获取，需要额外帧会话、线程/COM、旋转/设备错误处理；原架构中的DXGI为候选实验路线 | 本轮未实现或验收，不同时铺设多个后端 |

xcap发布包的Git提交为ef9e2a3892c8b972b5964aece131192d5777c660。源码审查发现默认capture_monitor未恢复选入内存DC的位图，scopeguard会先尝试删除该位图；微软文档明确当前选入DC的对象删除会失败。这是对该版本源码的资源释放风险判断，未运行xcap来测量泄漏，也不外推到其他版本或wgc特性。本轮直接控制GDI释放顺序，避免引入待验证的wrapper行为。

来源：[xcap仓库](https://github.com/nashaofu/xcap)、[对应发布源码](https://github.com/nashaofu/xcap/blob/ef9e2a3892c8b972b5964aece131192d5777c660/src/windows/gdi.rs)、[DeleteObject规则](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-deleteobject)、[微软GDI捕获说明](https://learn.microsoft.com/en-us/windows/win32/gdi/capturing-an-image)、[windows-rs](https://github.com/microsoft/windows-rs)。

Tauri命令返回二进制PNG采用已核对的Response::new，避免base64和JSON像素数组；前端收到ArrayBuffer。[Tauri二进制命令响应](https://v2.tauri.app/develop/calling-rust/#returning-array-buffers)

## 实现与边界

- Rust：capture/mod.rs定义显示器DTO、结构化错误、像素限制；capture/windows.rs隔离Win32；services/screen_capture.rs建立ScreenCaptureService和有界PNG编码；commands/capture.rs是两个薄IPC入口。
- ScreenCaptureService由Tauri持有，只有一个AtomicBool门控；在spawn_blocking前获取RAII许可，重复请求立即BUSY，不缓存图像/原生句柄、不堆积任务队列。正常、错误和panic展开均释放许可。
- list_displays返回当次设备id/name、物理widthPx/heightPx、带符号xPx/yPx、isPrimary。capture_fullscreen(displayId)重新枚举并匹配，未知/已断开ID明确拒绝；不会把ID当任意文件路径。
- 工作线程使用Per-Monitor V2 DPI上下文，退出恢复原上下文；物理矩形和当前显示模式尺寸一致才分配。捕获前后重新核对目标配置，变化时拒绝旧帧。
- 申请DC/位图后使用RAII释放；GetDIBits前恢复原位图选择，清理时先关闭内存DC再删除位图，最后释放屏幕DC。清理失败只输出通用错误步骤，不输出像素或截图内容。
- 原图物理像素使用top-down BGRA读取，原地转换为不透明RGBA。PRD上限16,777,216像素（64 MiB RGBA）；PNG输出上限65 MiB，增长分配和写入有界。没有缩放截图来伪造物理尺寸。
- 主窗口hide后等待350ms，再捕获并恢复；恢复错误单独报告，异常展开仍尝试恢复。等待时长基于本机Windows隐藏动画实测，不是所有设置下的保证；DwmFlush并不能单独终止隐藏动画。
- 前端features/capture提供显示器选择/刷新、全屏按钮与一张预览。PNG头和实际解码尺寸共同校验；解码器关闭、旧Blob URL替换/关闭时撤销；错误保留旧预览，迟到的旧图错误不能清除新图。
- main只新增allow-list-displays、allow-capture-fullscreen。没有fs插件、asset协议、任意路径读取、远程截图/上传或像素日志。CSP沿用已有本地IPC和blob图片范围。
- Screenshot是会话预览，不建立卡片、不保存PNG到素材库。未调用get_workspace_status，SQLite/storage/SQL迁移没有改动；没有重置用户AppData。

## 实际环境与命令

Windows x64，内核10.0.26200；Node 22.22.2 / npm 10.9.7，项目内Rust MSVC 1.99.0，Tauri 2.12.3，WebView2 154.0.4258.62。测试全屏窗口2560×1440、devicePixelRatio=1。

| 命令 | 实际结果 | 退出码 |
| --- | --- | --- |
| npm ls --depth=0 / cargo tree --depth 1 --locked | 已安装依赖核对成功 | 0 |
| cargo info xcap | 下载并核对发布源码/API 0.9.8；未作为应用依赖安装 | 0 |
| cargo fetch --locked --offline --target x86_64-pc-windows-msvc（通过包装脚本、指定manifest） | Windows锁文件依赖完整且可解析 | 0 |
| npm run check | TypeScript、ESLint、6文件52项Vitest、生产构建通过 | 0 |
| npm run format:rust | rustfmt通过 | 0 |
| npm run test:rust | 14项通过（新增5项、原有9项） | 0 |
| npm run check:rust | clippy all-targets、-D warnings通过 | 0 |
| npm run desktop:build | release Windows exe、no-bundle构建成功 | 0 |
| cargo run --locked --example capture_probe -- 50（通过包装脚本、指定manifest） | 主显示器50次捕获、PNG实际尺寸与资源计数通过 | 0 |
| npm run test:smoke | 生产浏览器6组检查，包括32种页面/主题/尺寸组合 | 0 |
| npm run test:smoke -- --desktop | 真实Windows应用启动/重启，20组检查通过 | 0 |
| node .tools/check-screen-capture.mjs | 11份文档、25个本地链接/围栏、命令权限、无fs授权、storage/迁移未改动与git diff --check通过 | 0 |

Rust新增测试覆盖分配前尺寸上限、DTO/ID校验、PNG宽高与逐像素保真、损坏缓冲/输出上限、并发拒绝与异常许可释放。前端新增19项覆盖真实命令名称/参数、浏览器防误调用、显示器/PNG响应校验、解码器关闭、选择/插拔回退、重复请求、失败重试和Blob生命周期；mock仅用于单元测试，不用于native验收。

windows 0.62.2、png 0.18.1复用锁文件已有版本，Cargo.lock只增加应用根依赖关系，未升级其他包。npm依赖/锁文件没有因本轮捕获新增或升级。使用已安装依赖的Windows锁定解析和实际构建验证，未进行全量冷缓存重建。

## 真实显示器与内容检查

| 显示器 | 枚举的物理尺寸 | 原点 | 本轮检查 |
| --- | --- | --- | --- |
| 主显示器 DISPLAY2 | 2560×1440 | (0,0) | 指定该ID捕获、50次循环、真实内容与应用预览 |
| 次显示器 DISPLAY1 | 2560×1600 | (-2560,0) | 仅枚举，未验收其图像捕获 |

capture_probe保留一个活跃测试DC来确认GetGuiResources计数器有效；50次前后GDI对象均为1，没有持续增长。只输出尺寸与计数，不保存屏幕图像。这不能证明所有驱动或全部资源类型均无泄漏。

原生UI验收在真实Windows桌面打开可见的全屏色块窗口。色块没有传给捕获服务；Rust直接读取显示器，前端对返回图像解码后核对九处像素，包含主窗口隐藏前位置内的采样点。色块布局改变后再次捕获必须得到新画面，尺寸与主显示器一致。启动和重启均通过；最后四次测量为696/667/662/634ms，是本次交互样本，不是性能p95或通用保证。

实际IPC验证INVALID_DISPLAY、DISPLAY_GONE和两个同时请求的一成功/一BUSY；错误后主窗口仍可操作并重新截图。实际Blob URL替换、旧URL失效、关闭预览、重启无旧截图通过。观察到的命令限定版本/主题/通信/显示器/捕获，没有数据库诊断调用；localStorage/sessionStorage没有写入。原生图像组件正常渲染，未发现renderer错误。

测试WebView2资料目录为artifacts/screen-capture-webview-profile，测试结束关闭应用与色块浏览器。证据为artifacts/screen-capture-desktop-smoke.json、screen-capture-preview-smoke.json、screen-capture-desktop-preview.png及restart预览。旧init/main-ui/M1证据保留。截图证据只用于本地测试，不作为科研素材保存。

Windows exe：src-tauri/target/release/researchclip.exe，10,572,800 bytes；SHA-256 B3F994C95A8CA46154A0023C42ED416AC07A9AC5B2FC70CD828687A2202732D3。

## 修复过程与未验证项

- 初次锁文件命令缺少manifest，随后--locked拒绝新增根依赖；改用指定manifest的offline解析，锁文件仅补根依赖。
- 当前windows-rs使用Error::from_thread，GDI资源计数位于System::Threading；编译发现旧名称/错误命名空间后按安装源码修正。未将编译失败记为通过。
- clippy发现可合并条件和固定大小分块用法，修复后严格检查通过。
- 初次测试浏览器未覆盖全屏，像素检查失败；改用实际窗口全屏设置，并在捕获前断言窗口覆盖显示器。没有把部分屏幕色块当成完整验收。
- 首轮五点采样漏掉Windows隐藏淡出残影；查看预览后改为350ms等待并增加四个窗口内部采样点。最后九点检查和实际预览均无ResearchClip残影。
- IPC观察可能提前捕获合法getVersion请求；按既有权限补齐允许列表，仍保留对其他命令/数据库调用的拒绝断言。对故意访问已撤销Blob产生的资源错误，只豁免该精确URL，不忽略其他renderer错误。
- 没有未解决的已测主显示器阻塞。不同动画设置、Windows 10、其他GPU/驱动、4K及16MP边界真机、混合DPI、旋转、多屏图像、插拔、HDR、锁屏/安全桌面、远程桌面和macOS仍未验证。不能宣称这些全部支持。
- GDI/驱动调用没有安全强制取消的硬超时，受保护画面可能失败或出现黑色区域，不尝试绕过系统保护。当前不标记截图为已校准sRGB/HDR。

## 手动验收与下一步

运行npm run desktop:dev，在“截图素材”选择目标显示器并点击“全屏截图”；检查尺寸、屏幕内容、重复捕获、关闭预览及主窗口恢复。若有其他硬件，分别检查各显示器、100/125/150/200%缩放、旋转与HDR；对断开显示器使用刷新列表并重试。锁屏/安全桌面/受保护内容的错误或黑色区域需单独记录，不能以普通桌面结果代替。

下一步需独立请求：扩展硬件矩阵或区域选择/热键；持久化必须另行设计迁移和文件一致性。本轮完成全屏捕获预览后停止。

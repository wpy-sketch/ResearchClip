# A07：Windows 智能窗口截图

## 支持范围与边界

第一版选择普通、可见、未最小化的顶层桌面窗口。不识别网页 DOM，不捕获 ResearchClip 自身窗口，不绕过受保护内容。窗口可以被其他普通窗口部分遮挡；兼容性必须以实际应用验收为准，不承诺所有 GPU、HDR、远程桌面、权限级别或应用均可用。

需要 Windows 10 1903（18362）或更新版本、可用的 D3D11 硬件设备。`GraphicsCaptureSession.IsSupported` 与实际创建/捕获均检查；不兼容、拒绝访问、超时、窗口关闭/最小化/边界改变均返回可理解的错误。没有桌面裁剪或 PrintWindow 的隐式降级。

## 三层职责

### 窗口边界与用户选择

`services/window_capture/windows.rs` 使用 EnumWindows 的 Z 顺序、IsWindowVisible、IsIconic、DWM cloaked/extended frame bounds 和进程身份筛选。全部 ResearchClip 进程内窗口被排除。最多512个可见窗口；溢出报错，不截断后悄悄选择错误目标。可读的非零 display affinity 拒绝；查询不可用时仍由 WGC 与 Windows 系统保护决定权限，不以查询失败推导允许绕过保护。

`WindowCaptureService` 仅持有一个活动选择会话；不把 HWND 传给 WebView，也不接受前端传来的任意 HWND。鼠标位置来自原生 GetCursorPos。鼠标移动选择最上层候选，Tab/Shift+Tab 轮换未标记保护的候选；不需要 UI Automation。

### 实际图像捕获

通过 `IGraphicsCaptureItemInterop.CreateForWindow` 创建 WGC item，`CreateFreeThreaded` 提供不依赖 DispatcherQueue 的2帧缓冲。帧获取以20毫秒间隔有界轮询；安装版windows-rs把S_OK加空帧投影为HRESULT 0的Error::empty，仅该情况继续等待，真实API错误立即返回。图像读取、D3D staging、PNG 编码均在后台进行。内容尺寸与捕获 item 比较；行跨度显式校验并转换 BGRA→RGBA，不用 DWM 边界裁切 WGC 像素。

像素上限沿用16,777,216，GPU/帧等待最多3秒，轮询支持取消。帧、映射、session、pool、WinRT device按所有权释放。实测本机WGC在完整Close/Release之后仍每次保留两个ALPC连接，根因未归为Windows或驱动的确定缺陷。最终采用同一EXE的短生命周期捕获worker，主服务以私有stdin/stdout传递最多8KiB身份请求和有界PNG，不新增外部依赖、服务或监听端口。worker在Tauri初始化之前执行，不读取配置或SQLite；Windows Job限制512MiB且父进程退出时终止，6秒外层超时、取消及所有失败路径都会终止并等待回收。原生WGC帧等待仍限制3秒。20次实际生产路径测试检查主进程句柄及ALPC保持稳定；不隐瞒原同进程路径的资源失败。

### 完成后处理

`commands/window_capture.rs` 复用全局 CapturePermit 和 HiddenWindow；选择层销毁后才读取窗口图像。捕获完成写入既有 CaptureWorkflow，模式为 `window`。前端复用 PNG预览、剪贴板、系统保存对话框、入库与 Konva 编辑器，不新增第二套图像或持久化模型。

## 坐标与生命周期

所有枚举/鼠标操作在可恢复的线程级 PMv2 DPI context 中执行。DWM 边界为物理像素，允许负坐标；前端根据完整虚拟桌面物理矩形和当前 CSS viewport 比例映射，不重复乘 DPI。选择层覆盖虚拟桌面，单边最大16384、面积最多67,108,864像素；超限明确拒绝。

主窗口暂时隐藏。透明、无边框、置顶且不显示任务栏的 `window-picker-*` 窗口只允许两个特定业务命令（状态与选择动作），没有通用窗口、文件、SQL或网络访问权。页面就绪后才显示并检查焦点；失焦、窗口位置/尺寸变化、显示器拓扑变化、Esc、退出或隐藏操作取消。就绪最多15秒，总选择最多120秒。已提交选择不因正常销毁事件被误判取消；退出仍可取消实际捕获。

原图、SQLite schema6、Annotation schema1、既有迁移、同步协议均不改变。新依赖：无；仅启用已安装 MIT/Apache-2.0 `windows` 0.62.2 的 WGC/D3D 功能。

## 核实的 API 来源

- [Microsoft：窗口截图概览、IsSupported 与 ContentSize](https://learn.microsoft.com/en-us/windows/apps/develop/media-authoring-processing/screen-capture)
- [Microsoft：CreateForWindow 的 Windows 10 1903 要求](https://learn.microsoft.com/en-us/windows/win32/api/windows.graphics.capture.interop/nf-windows-graphics-capture-interop-igraphicscaptureiteminterop-createforwindow)
- [Microsoft：CreateFreeThreaded 与事件线程](https://learn.microsoft.com/en-us/uwp/api/windows.graphics.capture.direct3d11captureframepool.createfreethreaded)
- [Microsoft：GetWindowRect、DPI 与 DWM 边界区别](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getwindowrect)

实际使用的方法、类型和特性名同时核对本项目锁定的 windows 0.62.2 源码；并用真实 Windows 原生探针核验，结果以 [v3.0.1验收](qa/RELEASE-3.0.1.md) 为准。安全桌面、提权窗口、混合DPI/多屏与特定第三方应用需人工测试，不能由坐标单元测试代替。

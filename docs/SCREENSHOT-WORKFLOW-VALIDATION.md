# 截图工作流验收

2026-10-13。本轮包含设置、全局快捷键、真实图片剪贴板、系统 PNG 保存及编辑器交接；后续用户追加基础编辑器与标注，见 [编辑器验收](EDITOR-VALIDATION.md)。

## 实现和权限

- 官方 Rust 插件：global-shortcut 2.4.1、clipboard-manager 2.4.2、dialog 2.8.2，Tauri 2.12.3，均已核对安装源码、公开 API 和 MIT/Apache-2.0 许可证；没有引入 JS 通用插件操作权限。
- ScreenCaptureService 继续负责原生帧和并发 RAII 门控，按钮与快捷键共用 run_region_capture。只处理 Pressed，忙时丢弃重复请求。
- SettingsService 持有明确有界状态，schemaVersion=1 JSON 同目录暂存/sync/原子提交；损坏/未来版本保留文件并拒绝写入。新快捷键先注册，失败保留旧键；后续注销/文件写入失败按已验证路径回滚。主窗口关闭在工作线程注销全部本应用快捷键，避免阻塞 UI 线程。
- CaptureWorkflow 只持有最新一张受管 PNG，物理尺寸/PNG/RGBA 上限保持 16,777,216 像素和约 65 MiB。captureId 为时间、PID 和检查溢出的序号，不接受任意路径。discard 的迟到回调不能删除新结果。
- 全屏和区域返回 CaptureInfo，前端通过 capture_result_png 获取受控字节并校验实际解码尺寸。region_result_png 保留为受限兼容入口。成功框选不再自动写临时文件。
- copy_capture_image 解码真实 RGBA 后调用官方 write_image；不是文本路径。错误映射为 CLIPBOARD_UNAVAILABLE，保留预览。
- choose_save_directory 与 save_capture_png 由 Rust 打开系统对话框。保存路径只来自对话框；PNG 暂存并 persist_noclobber，任何同名目标不覆盖。取消不新增文件或改设置。
- main capability 仅增加所需自定义命令、事件 listen/unlisten。overlay 仍严格只有原来的五个所属会话命令；不授予 fs、通用剪贴板读取、JS 快捷键注册、窗口创建/关闭权限。
- 原始 Blob 交接编辑器后独立拥有 URL，替换/取消截图预览不会改写正在编辑的原图。截图页复制/保存仍是原始截图；编辑器新增独立合成导出/复制，见编辑器验收。

API 来源：[全局快捷键](https://docs.rs/tauri-plugin-global-shortcut/2.4.1/tauri_plugin_global_shortcut/)、[图片剪贴板](https://docs.rs/tauri-plugin-clipboard-manager/2.4.2/tauri_plugin_clipboard_manager/struct.Clipboard.html)、[系统文件对话框](https://docs.rs/tauri-plugin-dialog/2.8.2/tauri_plugin_dialog/struct.FileDialogBuilder.html)。Windows 物理捕获仍是已有 GDI，不改成 CSS 或远程服务。

## 本轮实际结果

- npm ci --offline：退出 0，214 包安装，审计 0 漏洞。
- npm run check：96 项/12 文件通过，TypeScript、ESLint、Vite 生产构建通过。编辑器拆为延迟加载 chunk，未提高 chunk 警告门槛。
- npm run test:rust：28 项通过。旧区域临时路径保存测试已被统一资源/PNG 导出测试替代，所以测试总数从中间的 29 变为 28；未删除错误、边界或坐标断言。
- npm run check:rust 与 format:rust：实际通过。Rust 单测覆盖未知设置版本/损坏保留、冲突和写入回滚、PNG 实际像素、旧 ID、迟到取消、禁止覆盖和暂存清理，以及已有裁剪/迁移。
- Windows debug 与 release 无安装包构建实际生成 exe。最终无警告构建复验和文档检查结果以末尾记录为准。
- node scripts/editor-smoke.mjs ui-check3：7 组浏览器编辑器交互通过；只验证编辑器，图片为 fixture。
- 原生多轮普通启动实际验证了偏好写入、字体/字号/主题、Windows 快捷键冲突保留旧设置、修改后旧键释放、系统目录/保存对话框打开和取消。
- plugins-fixture4 与 plugins-save1：普通 Tauri debug exe 通过两个明确的测试环境标志加载固定 PNG；官方插件将图片写入真实 Windows 剪贴板，另一个 Edge 图片接收页通过 Clipboard API 粘贴 image/png，320×200 全部 RGBA 像素一致。此证据是系统剪贴板图片传输，不是原生屏幕捕获。画图/Word/GIMP 的物理 Ctrl+V 尚未人工验证。

## 未通过与诊断

1. 当前独立 capture_probe -- 1 退出 1，普通桌面主屏 2560×1440 的 GDI 返回 PERMISSION_DENIED；native-integration1 明确记录 Pressed 回调已到达，随后同样错误、没有覆盖窗口。原因未确定，不能凭用户已确认解锁就假设应当成功，也不能用以前的通过成绩替代。
2. native-check1/8 的 SendInput 被进程前台校验拒绝；native-check2 发送后没出现选择层。本轮未通过物理后台快捷键到真实区域截图的整条链路。WM_HOTKEY 模式是单独消息集成验证，不伪装物理按键。
3. plugins-fixture4 的真实剪贴板持锁场景未在预算内出现预期错误，报告保留失败。错误映射/预览保留在前端与 Rust 数据测试中验证，但真实竞争场景待人工复验；带 --skip-clipboard-lock 的报告明确列 skipped。
4. 系统 UI Automation 只枚举到文件浏览区域，未能定位文件名/保存控件，因此插件另存为的完成落盘尚不能仅据自动化宣称通过。保存纯逻辑的原子提交/PNG/同名保护已单测通过，真实另存为按后续实际记录更新。
5. 单独实验可执行文件曾遇到 Windows STATUS_ENTRYPOINT_NOT_FOUND，未进入插件测试；该实验已移除，未作为生产失败或剪贴板成功证据。本轮拥有的故障进程已按 PID 与完整路径核对后结束，未关闭用户 ResearchClip 实例。
6. 一次原生测试启动自动审批超时，按工具要求重试后得到执行；超时不是安全风险判定。

所有失败/成功/跳过报告保留于 artifacts，未清空真实 AppData、重置数据库、覆盖用户 PNG 或改变屏幕采集方案。

## 复验方法

```powershell
npm run desktop:dev
# 设置中检查快捷键，再切换其他应用按 Ctrl+Shift+2
# 截图完成后分别测试复制、保存 PNG、打开编辑器与取消

npm run tauri -- build --debug --no-bundle
node scripts/with-rust.mjs cargo build --manifest-path src-tauri/Cargo.toml --locked --example workflow_input
npm run test:smoke:workflow -- manual-native-1
```

自动原生工作流使用独立设置/浏览器资料和新报告目录，输入仅向验证过的测试进程/窗口发送。--fixture 仅在 debug 构建且同时设置两个测试环境标志时加载固定非屏幕 PNG，报告 nativeCaptureVerified=false，不能作为截图真机验收。普通应用启动和 release 永不生成测试截图。

人工矩阵：Windows 普通桌面和目标屏访问、后台/最小化快捷键、Esc/失焦/连续会话、100/125/150/200% DPI 与负原点、多屏插拔；画图/Word/GIMP 实际 Ctrl+V；剪贴板占用/不可用；目录更改与重启；另存为取消、同名、只读目录和磁盘满；关闭后其他进程可注册同一快捷键。macOS 不在本轮验收范围。

## 后续实际补充（2026-10-13）

- plugins-save2实际系统另存为生成默认命名PNG，但自动化假定名称为native.png，因此报告仍保留失败。随后png_export_probe核对真正生成的PNG：320×200、全部受控RGBA像素一致，退出0；不称整份报告通过。
- native-settings-graceful实际5组偏好/快捷键/目录取消/重启检查通过，无退出错误；此前Chrome_WidgetWin_0问题通过先注销、再正常关闭主窗口解决。最新settings-history-final --settings-only复验5组通过，errors=[]。
- 后续编辑器103项前端/32项Rust、Windows debug/release构建通过；native-final验证真实项目保存/打开/重启、PNG导出与合成图片剪贴板，详见 [编辑器验收](EDITOR-VALIDATION.md)。GDI屏幕权限和真实剪贴板竞争的未通过项保留，不据fixture改为通过。

## 基础画布请求中的真实主屏复验

capture_probe -- 1与canvas-real-screen-3本轮成功；全屏9个真实屏幕标记、区域裁剪/冻结/取消/连续会话、实际截图到编辑器平移和预览释放后保留、重启复验通过，报告errors=[]。此前PERMISSION_DENIED当前未复现，原因未确定；仍未验证所有副屏/DPI/物理快捷键/剪贴板竞争配置。最新证据与测试失败修复见 [基础画布复验](EDITOR-CANVAS-VALIDATION.md)。

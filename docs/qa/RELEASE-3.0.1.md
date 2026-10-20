# v3.0.1 / A07 窗口截图验收

执行：2026-10-19至20，Asia/Hong_Kong。Windows 11 10.0.26200 x64；Node22.22.2/npm10.9.7、Rust1.99 MSVC、Tauri2.12.3、windows0.62.2。记录实际执行，不把代码、Mock、坐标测试或正式版本标签当作未运行硬件矩阵通过。

## 1. 实现状态与架构

**已实现**：窗口级枚举/鼠标命中/高亮、Tab与反向Tab、单击WGC捕获、Esc/失焦取消、最小化/关闭/保护拒绝、实际内容尺寸、既有复制/系统保存/入库/编辑器管线。原始图片、Annotation schema1、SQLite schema6和既有迁移不改变。无新增第三方依赖；仅启用已锁定windows crate的WGC/D3D功能。

- `services/window_capture/mod.rs`：单会话、稳定身份、候选轮换、负坐标虚拟桌面与预算。
- `services/window_capture/windows.rs`：DWM物理边界、PMv2坐标、窗口身份复核、WGC/D3D staging/行跨度与PNG像素。
- `services/window_capture/worker.rs`、`main.rs`：同一EXE受控worker，8KiB严格请求、16,777,216像素/有界PNG、512MiB Job、6秒外层超时、终止与等待；worker不初始化Tauri或SQLite。
- `commands/window_capture.rs`：main-only发起、独立透明置顶选择窗口、焦点/拓扑/生命周期、共享CapturePermit及CaptureWorkflow。
- `WindowPicker.tsx`、`window-picker.ts`：独立入口，按物理桌面到CSS的同一变换定位；顺序轮询和清理事件。
- `CaptureControls/store/shared API`：窗口模式进入既有截图工作流；专属capability仅2个选择命令，主窗口增加1个有界业务命令。

完整契约与官方API来源见[设计](../WINDOW-CAPTURE.md)。未实现任意DOM/子控件识别、macOS窗口捕获、受保护内容绕过或桌面裁剪降级。

## 2. 自动检查与实际命令

| 状态 | 命令/检查 | 本轮证据与结果 |
| --- | --- | --- |
| PASS | `npm run check` | 类型、ESLint、52文件348个Vitest、Vite生产构建，退出0；体积警告保留 |
| PASS | `npm run check:rust` | 全targets Clippy，`-D warnings`，退出0 |
| PASS | `npm run test:rust`（原生权限） | 167通过、0失败、7显式忽略的独立WebDAV服务器测试，退出0 |
| PASS | `npm run tauri -- build --debug --no-bundle` | 实际Windows debug EXE与嵌入前端，退出0 |
| PASS | `cargo build --locked --manifest-path src-tauri/Cargo.toml --example window_capture_probe`（经with-rust包装） | 原生探针实际编译，退出0 |
| PASS | `node scripts/window-capture-native-smoke.mjs a07-final3` | 14个真实原生检查记录，退出0；独立设置/SQLite目录 |
| PASS | 最终干净安装、格式/文档、回归与NSIS | npm ci 303包；fmt/文档；前端348/Rust167/扩展5；原生回归5组、布局8组、指南9组；NSIS实际安装/忙碌拒绝/默认卸载与10份既有资料摘要，退出0 |
| NOT_RUN | 7个WebDAV外部测试用例 | 本轮未启动专属HTTP/TLS测试服务器；同步实现未修改，旧版本结果不冒充本轮 |

单元测试覆盖物理坐标、负原点、100%/125%/150%/200%数学变换、Tab轮换/保护跳过、BGRA行跨度与alpha、大小边界、会话身份/取消、worker请求校验、无帧HRESULT与真实错误区分、前端成功/取消/失败/重复触发。

## 3. 已执行的Windows原生验收

`artifacts/window-a07-final3/report.json`：

| 状态 | 用例 | 实际观察 |
| --- | --- | --- |
| PASS | 实际Edge窗口 | WGC802×731，已知网页左右色块258354/253186像素；普通可见Edge窗口通过，不等于所有网页或浏览器功能通过 |
| PASS | 普通GDI窗口与部分遮挡 | WGC626×473，已知两块像素保留；同一位置桌面像素为遮挡物，和WGC目标像素不同 |
| PASS | 负坐标副屏 | 目标原点(-2448,101)、96DPI，WGC619×469，已知色块校验通过；不等于125%/150%物理缩放通过 |
| PASS | 最小化/关闭目标 | 拒绝捕获；未复用错误句柄生成其他窗口图像 |
| PASS | 系统捕获保护 | 专用fixture设置WDA_EXCLUDEFROMCAPTURE，明确PermissionDenied；未尝试绕过 |
| PASS | 捕获中取消 | 40ms后取消，worker停止并等待回收 |
| PASS | 20次生产路径捕获 | 主进程句柄127→127，ALPC1→1，其余观测类型数量不变 |
| PASS | 真实选窗UI | OS鼠标移动与单击，标题/边界命中；CDP键盘Tab/Shift+Tab切换，Esc销毁独立窗口并恢复主窗口 |
| PASS | PNG与剪贴板 | 真实系统保存对话框，PNG像素尺寸一致；Windows CF_DIB/CF_DIBV5实读626×473图片头，非路径 |
| PASS | 编辑器/连续生命周期 | 捕获进入既有Konva画布；返回后3次选窗/取消保留上一张预览，未遗留选择窗口；应用正常退出0 |

原生fixture为专门创建的非科研色块窗口，用于可复现像素验收；不是用模拟返回值替代Windows截图。除上述真实Edge外，其他第三方软件兼容性需另列实测范围，不能由fixture覆盖全部应用。

## 4. 初次失败、修复与复验

1. **P1，已缓解并复验**：最初同进程WGC每20次句柄增加40，进一步按类型定位为ALPC连接增长。重现：启动受控色块窗口并连续20次CreateForWindow捕获。Close/Release、复用D3D与factory、显式表面清理及等待均未使其回收；不确定归因到系统或驱动的具体缺陷。最终以同EXE短生命周期worker隔离，Windows Job限制资源，取消/超时/父退出回收。相同生产路径20次复验127→127。未放宽原有上限断言，也未隐藏失败日志。
2. **P2，已修复**：枚举把隐藏窗口计入512上限，导致本机无法开始；改为只累计可见窗口，仍对真实溢出报错。
3. **P2，已修复**：初版轮询把空帧误认为失败；核对锁定windows-rs投影，S_OK+null为Error::empty/HRESULT0；仅该情况轮询，真实错误直接返回，增加自动测试。
4. **测试设施，已修复**：后台启动fixture默认隐藏、运行中的EXE链接锁、Node被信号终止时exitCode仍null造成等待超时；显式显示自己的fixture、结束后才重建、检查signalCode。探针IsWindow参数、JSON Value构造和Clippy布局告警均修复后重跑。
5. **环境限制**：沙箱Rust凭据往返首次FAIL（保存被拒绝），原生权限全套重跑PASS；沙箱`npm ci --no-audit --no-fund`首次FAIL（registry/cache EACCES），原生权限重跑PASS（303包）。系统保存对话框创建过程中有短暂控件未就绪重试，最终文件验收成功。

## 5. 未执行与人工清单

| 状态 | Windows人工操作 | 完成核对 |
| --- | --- | --- |
| NOT_RUN | 主副屏分别设125%/150%，混合DPI，跨屏移动后重新选择 | 高亮贴合实际窗口，PNG尺寸/内容正确，变化时取消或要求重选 |
| NOT_RUN | 快速移动鼠标、Tab、多次开始/取消，目标在提交时移动/关闭 | 不捕获错误窗口、无遗留遮罩、旧预览保留 |
| NOT_RUN | Chrome/Edge、Office、Obsidian、Electron、视频/游戏等实际常用应用 | 对每个应用独立核对遮挡、完整图像和失败提示，不推广为全部支持 |
| NOT_RUN | 提权窗口、安全桌面、DRM、远程桌面、HDR | 遵守权限，失败明确；不绕过保护，不把黑图解释为成功保护绕过 |
| NOT_RUN | 复制后实际粘贴到画图/Word/其他图像应用 | 已自动验证DIB存在；跨应用粘贴交互仍人工核对 |
| NOT_RUN | 选择层打开时最小化/切换窗口/改屏幕布局/从托盘退出 | 安全取消，窗口恢复，退出后无worker和选择层 |
| NOT_RUN | macOS、Windows10旧硬件矩阵 | 没有执行环境，不宣称通过 |

## 6. 最终构建、安装与发布门槛

原生核心流程、资源增长缓解与对应检查已通过。最终回归及NSIS安装/卸载PASS。最初NSIS打包因os error32未写入UNK→NSS标记，严格安装检查FAIL；实际文件没有其他未知差异。仅卸载本轮已确认的测试安装后重建，标记写入与完整摘要检查PASS，安装/忙碌拒绝/默认卸载PASS。10份既有用户资料摘要与失败测试之前一致，未改断言。优化EXE的worker实际输出626×473，两色各137574像素，退出0。GitHub标签与匿名下载在随后发布回执中单独记录。保留兼容WebClipper v1.0.2与可选MathOCR安装脚本；不上传用户数据、缓存、工具链或模型，不修改既有Release。

旧QA-001/002/008、OCR数学误识别和坚果云认证未验收限制继续保留。未新增数据库迁移或独立备份恢复功能。

## 7. 最终检查证据与修改文件

[机器可读检查摘要](evidence/v301-checks.json)。所有上表最终命令退出0；早期失败和复验分别保留，7个WebDAV服务器用例仍NOT_RUN。Vite大包警告、未签名和Windows硬件矩阵限制未被隐藏。

- `CHANGELOG.md`
- `README.md`
- `docs/ARCHITECTURE.md`
- `docs/ROADMAP.md`
- `docs/USER_GUIDE.md`
- `package-lock.json`
- `package.json`
- `scripts/check-docs.mjs`
- `scripts/guide-native-smoke.mjs`
- `scripts/guide-reader-smoke.mjs`
- `src-tauri/Cargo.lock`
- `src-tauri/Cargo.toml`
- `src-tauri/build.rs`
- `src-tauri/capabilities/main.json`
- `src-tauri/src/capture/mod.rs`
- `src-tauri/src/commands/mod.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/src/main.rs`
- `src-tauri/src/services/desktop.rs`
- `src-tauri/src/services/mod.rs`
- `src-tauri/tauri.conf.json`
- `src/app/open-editor.ts`
- `src/features/capture/CaptureControls.tsx`
- `src/features/capture/CapturePanel.tsx`
- `src/features/capture/store.test.ts`
- `src/features/capture/store.ts`
- `src/features/guide/GuideView.tsx`
- `src/features/library/LibraryView.tsx`
- `src/main.tsx`
- `src/shared/api/capture.ts`
- `src/shared/types/capture.ts`
- `src/styles.css`
- `docs/RELEASE-NOTES-3.0.1.md`
- `docs/WINDOW-CAPTURE.md`
- `docs/images/v301-window-capture.svg`
- `docs/qa/RELEASE-3.0.1.md`
- `docs/qa/evidence/v301-checks.json`
- `scripts/window-capture-native-smoke.mjs`
- `src-tauri/capabilities/window-picker.json`
- `src-tauri/examples/window_capture_probe.rs`
- `src-tauri/src/commands/window_capture.rs`
- `src-tauri/src/services/window_capture/mod.rs`
- `src-tauri/src/services/window_capture/windows.rs`
- `src-tauri/src/services/window_capture/worker.rs`
- `src/features/capture/WindowPicker.tsx`
- `src/features/capture/window-picker.test.ts`
- `src/features/capture/window-picker.ts`

## 8. 正式发布

普通v3.0.1、六附件匿名下载及18个既有发布保留均PASS，见[发布回执](PUBLICATION-3.0.1.md)。源码标签固定，不以回执提交更新程序附件。

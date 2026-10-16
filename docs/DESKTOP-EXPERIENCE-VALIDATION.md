# 桌面设置、托盘与生命周期验收

2026-10-15，Windows / PowerShell。本轮仅完善用户请求的桌面体验，延续现有截图、编辑器、SQLite v2与设置JSON；没有新增账号、云同步、自动更新或数据库迁移。使用方法见 [README](../README.md#托盘与退出)，技术契约见 [架构第13节](ARCHITECTURE.md#13-当前桌面设置与生命周期2026-10-15)。既有未提交修改与用户资料保留。

## 修改文件与行为

- Rust新增services/desktop.rs和commands/desktop.rs；lib.rs接入单实例、单托盘、关闭/退出事件，workflow.rs复用现有截图并让后台失败恢复窗口。build.rs、capabilities/main.json仅增加三个main业务命令，overlay权限不变。
- services/settings.rs扩展旧schema1配置，增加system主题、annotationColor、annotationWidth和closeToTray；默认深色、#ef6262、3px、关闭退出。旧文件读取不重写；配置仍以临时文件、sync和原子替换提交，未知版本/损坏文件拒绝覆盖。双快捷键保持已有注册事务和失败回滚。目录不可用明确提示且不删配置，editor.rs保存前也校验。
- 前端新增features/desktop的退出协调、事件监听与测试，shared/api/desktop为受限IPC；settings/theme、store、SettingsView和app/use-appearance支持三态主题和监听清理。App挂载全局退出提示。编辑器store/types、EditorView、app/library-actions记录实际保存点，默认样式仅作用于之后创建的对象，不改已有图元。
- 新增scripts/desktop-native-smoke.mjs、examples/desktop_probe.rs及npm测试入口；check-docs校验新增命令权限。README、ARCHITECTURE、ROADMAP和本报告同步当前行为。核心单测增加14项前端、7项Rust。

设置默认截图目录仍只决定系统另存为起始位置，不搬迁受管资料或自动保存。颜色/线宽点击“保存标注默认值”，其他设置直接保存。关闭默认退出；启用“关闭窗口时隐藏到托盘”才隐藏。最小化、托盘恢复及第二实例复用同一个主窗口。托盘退出始终请求真正退出。

正常退出先等待笔记保存，失败取消退出、保留草稿；未保存标注或活动绘制需要明确放弃或返回保存。成功保存只确认当时的文档，不清除其后的新修改。PNG导出不保存可编辑数据。Rust等待捕获/文件操作许可最多3秒，忙时提示重试；成功后暂停快捷键、注销、先关闭WebView，再移除托盘和单实例资源。未入库的截图预览仍是临时数据；强制结束进程和断电不承诺恢复。

## 依赖与环境

已检查官方 [Tauri托盘文档](https://v2.tauri.app/learn/system-tray/)、[单实例文档](https://v2.tauri.app/plugin/single-instance/)和安装源码。tauri2.12.3启用tray-icon，原锁文件已有tray-icon0.25.1；新增官方tauri-plugin-single-instance精确锁定2.5.3，MIT/Apache-2.0，声明Rust>=1.90、Tauri^2.12，与当前Rust1.99兼容。cargo info及锁文件构建实际成功。相比自写Windows互斥量/IPC或增加JS窗口权限，复用官方插件，作为首个插件注册。没有新增npm依赖或通用tray/process/fs/global-shortcut权限。

实际环境：Node22.22.2/npm10.9.7、Rust1.99 MSVC、Tauri2.12.3、Windows x64 10.0.26200、WebView2；CPU Intel Core Ultra 7 356H、16逻辑核心、内存33,879,642,112字节（Node os实际读取）。CIM硬件查询因访问限制失败，未取得本轮GPU/物理显示器DPI清单，因此不据此声称显示器硬件矩阵通过。

## 实际检查

以下为本轮实际运行，退出码均为0。

| 命令 | 实际结果 |
| --- | --- |
| npm run check | 24文件、171项测试通过；TypeScript、ESLint零警告、Vite生产构建通过 |
| npm run test:rust | 87项通过，包含旧配置兼容、默认样式校验/重载、目录失效、退出暂停注册、桌面阶段重复事件测试 |
| npm run format:rust | 格式检查通过 |
| npm run check:rust | Clippy所有目标通过，-D warnings |
| npm run tauri -- build --debug --no-bundle | Windows debug构建通过，原生验收使用该程序 |
| npm run test:smoke:experience:native -- desktop-20261015-final | 10组真实Windows桌面流程通过，errors为空；captureOutcome=CANCELLED |
| npm run test:smoke:knowledge:native -- desktop-regression-20261015 | 9组原生笔记/标签/搜索/编辑器往返/保存失败恢复/重启通过，errors为空 |
| npm run test:smoke:markdown:native -- desktop-export-regression-20261015 | 7组真实系统目录对话框/导出/部分失败/路径权限回归通过，errors为空 |
| npm run test:smoke -- --run-id desktop-ui-20261015 | 6组、32种页面/主题/内容视口通过，errors为空；4种overlay mock DPR仅验证渲染 |
| CARGO_TARGET_DIR=src-tauri/target/desktop-release；npm run desktop:build | 新目标目录完整Windows release构建通过，Rust编译1分34秒；非安装包/签名验收 |
| npm run check:docs | 22份文档、67个本地链接、围栏及命令权限/迁移一致性通过 |

生产产物为src-tauri/target/desktop-release/release/researchclip.exe；可直接运行，或npm run desktop:dev。请先保存并从托盘退出旧版，再启动新产物，否则单实例会恢复正在运行的旧版。实际功能原生验收使用本轮debug程序；release完成了干净目标目录的编译，没有使用用户真实配置/资料进行自动化写入。

原生主证据为artifacts/desktop-desktop-20261015-final/report.json及settings.png；已查看设置截图确认三态主题布局。测试助手只控制传入PID拥有的主窗口、托盘和前台原生菜单，使用固定合成键盘菜单访问键，未操纵其他应用。左键托盘回调通过已安装tray-icon源码核对的Win32消息进入真实回调；菜单使用真实原生弹出菜单，不以JS伪造菜单结果。

10组覆盖：旧配置补默认值且不重置；system偏好保存/深浅媒体变化；关闭隐藏及三次最小化/恢复保持同一HWND和一个托盘；第二真实进程激活第一进程后退出；托盘设置菜单；另一真实进程占用快捷键后的注册失败/原文件与绑定保持/释放后重试；删除仅本测试空默认目录后的明确提示；默认样式不改选中旧对象且下个对象采用新值；未保存标注退出取消后可继续编辑；真实SQLite触发器阻止笔记保存时退出被取消，恢复后保存退出；重启恢复全部设置与笔记；退出后原生窗口/托盘移除且OS热键可重新注册。

真实托盘区域命令进入既有GDI冻结画面和覆盖窗口，向本测试拥有的选择层发送Esc后返回CANCELLED。它证明原生入口和取消生命周期，不代表完成一次成功区域截图。持久化/编辑器检查使用明确320×200受控PNG，不能称为屏幕捕获。跟随系统以CDP媒体模拟验证监听，不等同物理Windows设置切换。所有测试使用独立artifacts配置、资料和WebView目录；debug单实例标识隔离测试命名空间。没有重置真实AppData或结束用户实例。

## 已发现并修复

早期原生1的checkbox.check()期待保存前立即更新，改为真实点击、等待Rust配置保存并核对最终选中状态。原生2按菜单位置导航误选相邻的全屏截图项，触发了真实截图；改为唯一原生菜单访问键和固定输入，并检查前台窗口归属，未放宽业务断言。原生3期待快捷键输入别名，而插件实际规范化为shift+control+alt+F11；修正预期后原文件/绑定检查通过。

原生4前8组完成，但stderr出现Chrome_WidgetWin_0注销错误1412，且后台弹出菜单未取得前台。修复产品退出顺序：先关闭活跃WebView，再由正常Exit事件销毁托盘/插件；测试在导航原生菜单前恢复自己的主窗口并继续严格核对前台PID。后续5和final完整通过，未过滤stderr或隐藏错误。前端首次lint提示Fast Refresh组件文件混合导出hook，拆分后严格lint通过。早期一次检查调用在自动审批等待时超时，按工具允许重试后执行成功，无待批准操作。

失败报告保留在artifacts/desktop-desktop-20261015-*；包括误选全屏产生的本地测试截图，未上传。退出资源单测及原生检查不以强杀进程制造成功。

## 已知边界与下一步

本轮没有未解决的基本编译或桌面流程阻塞。仍需人工验证物理Windows深浅主题切换及标题栏、托盘隐藏图标区域/Explorer重启、默认Ctrl+Shift+1/2的物理触发、完整成功截图及多屏/混合DPI、只读/磁盘满/剪贴板竞争、系统关机等场景。macOS与Windows其他版本、安装包/签名、长时间资源和1000卡性能矩阵未验收。release构建不等同完整发布验收；不向真实资料库进行自动化写入。下一步仅建议人工验收上述条件；本轮完成后停止。

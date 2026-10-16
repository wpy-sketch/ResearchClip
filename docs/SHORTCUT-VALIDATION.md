# 区域与全屏独立快捷键验收

延续用户此前请求，2026-10-14收尾。使用已有官方tauri-plugin-global-shortcut2.4.1，不新增依赖或前端通用注册权限；API核对见 [官方文档](https://v2.tauri.app/plugin/global-shortcut/)。源码契约见 [架构](ARCHITECTURE.md)。

## 行为

保留“区域截图快捷键”（默认Ctrl+Shift+2），新增独立“全屏截图快捷键”（默认Ctrl+Shift+1）。按解析ID检测相同组合及等价写法，外部OS占用显示错误并保留原绑定和设置。原captureShortcut字段不改名或动作；旧版本1JSON缺fullscreenShortcut时补默认而不读取即重写。若旧区域占用默认1，保留区域并报告全屏冲突。

一次Rust handler按Pressed与动作映射分发，共用捕获门控。主屏全屏与按钮复用捕获/隐藏/恢复/预览，区域复用冻结层。注册/注销/文件提交失败回滚；回滚失败暂停动作并提示重启。关闭注销两组键，交换已注册键只更新映射。

修改文件：Rust services/settings.rs、commands/capture.rs、commands/workflow.rs、lib.rs；前端shared/api/settings.ts、settings/store.ts与测试、SettingsView.tsx、App.tsx；examples/workflow_input.rs及scripts/shortcut-smoke.mjs、workflow-smoke.mjs。capability、锁文件及SQLite迁移保持现状。

## 本次通过项

- npm run check：112项、类型/ESLint/Vite构建，退出0。
- Rust format/test/clippy和Windows debug/release构建退出0；Rust39项包含7项双快捷键测试：旧文件不重写、独立修改/重载、等价/外部冲突、启动全屏冲突保留区域、旧默认冲突、部分失败回滚与暂停、已注册键交换。
- npm run test:smoke:workflow -- independent-hotkeys-oct14 --settings-only：退出0，7组，errors为空。实际Windows应用采用隔离设置：区域改3，全屏相同3被拒绝；独立helper占用4制造真实OS冲突，修改失败保留两键；释放后全屏改4，1释放、3/4被应用占用，设置文件保留两项；实际关闭后3/4可被helper注册；重启两键恢复。另检查外观持久化和系统目录对话框取消。

这些结果证明OS注册和注销，不证明物理按键触发或当前原生捕获成功。

## 失败与历史部分结果

保留失败报告，不删除断言：

- 上轮artifacts/shortcuts-dual-hotkeys-1退出1：前台PID保护拒绝向其他应用发送按键，未注入快捷键。
- 上轮shortcuts-dual-hotkeys-message退出1，显式synthetic WM_HOTKEY，5组部分通过。默认1和改后4捕获实际主屏2560×1440，四个实时屏幕色块像素匹配，正常预览且无区域层；相同键/真实外部占用失败保留旧键。后续区域3层在焦点就绪前关闭，断言失败，未验证关闭/重启，不记为全通过。
- 上轮追加诊断与截图回归因自动审批额度耗尽未执行；本轮取得执行授权后重试诊断，没有绕过审批。
- 本次 node scripts/shortcut-smoke.mjs dual-hotkeys-oct14-diagnostic --message-trigger退出1。旧区域3保留、新默认1注册、不重写旧文件通过；触发后capture-workflow返回failed / PERMISSION_DENIED，Windows拒绝屏幕读取，未得到截图。workflowEvents保存started与failed；errors数组无JS异常不等于功能通过。未进入区域焦点检查，上轮区域层关闭原因仍未确认。

真实捕获和物理按键验收受Windows桌面访问/前台输入条件阻塞。用户此前已确认桌面解锁；未改变系统权限策略，未用CSS或固定图片替代原生截图。旧成功记录不能说明所有桌面会话稳定。

## 手动复验

运行 npm run desktop:dev，在设置分别指定两组不冲突键。切到其他应用，按全屏键确认主屏预览；区域键框选，检查Esc、主界面恢复与连续截图；关闭重启复验。若失败保留错误码，不重置设置或数据库。

隔离复验先构建debug应用、workflow_input及raise_capture_fixture两个example。用新ID执行 npm run test:smoke:workflow -- new-id --settings-only（仅注册生命周期），或 node scripts/shortcut-smoke.mjs new-id --message-trigger（显式合成消息，不能代替物理键验收）。测试只操作自建PID、固定窗口和独立artifacts目录。

下一步依赖当前普通Windows桌面的屏幕访问与物理键盘/前台条件，复验两组真实触发和区域焦点；多显示器及实际DPI另行验收。本次安全的独立检查完成后停止。

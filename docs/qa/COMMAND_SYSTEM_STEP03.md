# Step03 验收记录

环境Windows x64、Node22、React19、Tauri2/WebView2，SQLite12。测试只用新建artifacts隔离目录与生成PNG，不触碰真实AppData。

## 本轮范围

已实现CommandRegistry、类型化EventBus、ResourceResolver、PermissionService、可取消TaskManager及ErrorReporter。7个封闭示例契约，2个实际业务适配，5个未来模块明确不可用。无新增依赖/迁移；原文档/截图服务继续权威写入。架构与限制见[命令系统](../architecture/COMMAND_SYSTEM.md)。

## 实际检查

- npm run check：退出0，68文件484测试通过，类型/lint/生产构建通过。新增28个命令/事件前端用例覆盖权限句柄伪造/撤销、收据重放与异参冲突、资源锁、取消/提交、后端响应不匹配、事件循环/冻结/泄漏预算、失败监听和诊断隐私。
- npm run test:rust：退出0，236通过/7忽略；新增6个协议/窗口/只读解析测试。npm run format:rust与npm run check:rust退出0。
- node scripts/commands-native-smoke.mjs v412-delivery：退出0，8组Windows原生回归，实际IPC拒绝、未来模块不可用、运行任务阻止退出、取消、跨类型同ID解析、焦点切换后的稳定目标插图、故障保留原文和显式重试。
- node scripts/workspace-native-smoke.mjs v412-delivery2：退出0，7组，分栏/保存/脏关闭/隐藏/拖动/窗口恢复/进程重启/图片撤销/离线指南通过。指南32张图片实际解码。
- node scripts/resources-native-smoke.mjs v412-delivery：退出0，6组，隔离v11→12迁移、旧截图/批注/Markdown/URI/重启/原图哈希完整性通过。
- guide-reader-v412-delivery：退出0，14组浏览器检查；20章122例32图，主题、字体、480–1440宽度、75–200% CSS缩放、三面板边界和125%拖动通过。CSS模拟不代表物理DPI。
- npm run tauri -- build --debug --no-bundle：退出0，以上原生测试顺序运行，记录见artifacts/dock-evaluation/delivery-*.log。
- 取消/焦点故障注入只在测试实例延迟原生IPC的fetch传输，然后释放并实际调用Rust；没有假响应、放宽权限或产品调试桥。初版尝试覆盖Tauri只读invoke没有生效，测试失败记录保留，已改用标准fetch传输边界。
- 历史附件归档含31版226附件，每个ZIP成员均核对GitHub SHA-256；分223卷，原始公开附件总字节4117989169，归档字节3728704762。Windows223卷实际重组及整体SHA匹配通过；已有不同目标/越界路径均退出1并保留哨兵，相同目标幂等校验退出0。发布传输另见发布报告。

正式安装包检查另见[发布报告](RELEASE-4.1.2.md)。不将本轮测试称为用户人工验收。

## Windows人工验收（未执行）

1. 通过命令面板打开最近文档，再次打开同一ID确认聚焦已有草稿；文件不存在时入口不可用。
2. 打开两份练习稿，向已登记文档插入已入库图，核对目标、撤销与显式保存；未保存文档和未保存图片合成仍使用原流程。
3. 查询阶段可取消时查看状态；提交阶段等待完成，不反复点击。失败后核对结果，再新建请求重试。
4. 打开任务与诊断，检查窄/短窗口、两主题、12–20px及物理多DPI，确认旧成功通知不会冒充新失败。
5. 模块状态中确认PDF、标准批注、代码和Agent不可用；不启动Zotero/HarmonyOS/Agent权限流程。
6. 下载全部历史分卷，先核对摘要、重组并自行解压到新目录；旧程序不是v4.1.2。按各组件原说明配置，可复用已有运行时。

## 已知限制与未验证

仅两处UI接入新命令层，旧截图/OCR工作流不重写；总计1024请求/运行、4并行任务、128事件监听。事件不支持异步回调，取消不能中断已开始的文件提交。诊断/任务/收据不跨进程保存。逻辑模块权限不是JS沙箱，没有对Agent开放IPC。

未验证用户真实资料、多显示器物理DPI、macOS、Zotero/批注写回、HarmonyOS压感、未实现PDF/代码/Agent第三方接口。历史advanced-native退出超时未宣称通过。额外workflow-smoke原生截图脚本本轮因前台窗口所有权保护退出1，未向其他应用发送按键；该检查与调试构建误并行造成一次EXE占用构建失败，测试实例结束后顺序重建成功。保留失败记录，不把额外截图/物理硬件回归称为通过。

主要修改：src/features/commands/platform/*、既有命令面板/InsertButton/App接入；Rust命令模块/注册/两项最小capability；指南/架构/发布脚本。新增PDF→业务、Markdown导出PDF和独立数据备份/恢复均不在本轮范围。

# v3.1.0 WebDAV设置交互验收

执行：2026-10-20，Asia/Hong_Kong。Windows11 10.0.26200 x64，Node22.22.2/npm10.9.7、Rust1.99 MSVC、Tauri2.12.3。仅使用隔离测试资料与专用凭据。PASS只代表本报告明确执行的范围，不把Mock、标签或旧记录当作原生认证结果。

## 实现状态与入口

已实现：`features/sync/{api,form,store,SyncSettings}`分别负责经校验IPC、字段规则、独立读写/停止状态与表单；`commands/sync.rs`负责异步本机任务；`services/sync/runtime.rs`负责凭据可用性与非阻塞取消。沿用既有资源同步协议，无SQLite迁移、标注格式或权限变化，无新增依赖。指南保持14章，新增三级排错与按钮条件表，两张原生测试截图取代旧浏览器表单示意，当前23幅。

## 初次失败与可复现问题

1. **P1，单元复现并修复**：`action`等待`refresh`，延迟状态返回会使busy不释放；同一busy门控使stop不调用IPC。新增两项前端回归初次2失败/5通过，修复后分别验证操作释放、停止和启动后重送取消。
2. **P1，Rust复现并修复**：持有job锁时调用stop，200ms内未返回。原测试FAIL；改为AtomicBool先设置、try_lock仅更新提示，复验通过。启动/冲突IPC移出UI线程。
3. **P2，原生复现并修复**：空表单保存错误在视野外，用户缺少反馈。隔离Windows原生基线观察false，修复后true。已保存连接的凭据失效原先被吞成false，现明确提示重输；改变连接不再误导为可复用旧凭据。
4. **原始现场限制**：基线Windows新表单可编辑，保存按钮未禁用。因此不将“所有按钮失效”截图归因全部确定为同一问题。原先尝试改写Tauri不可写invoke，延迟注入没有生效；该阶段延迟/回执结论无效，真实保存/凭据观察仍独立有效。最终改为可验证的fetch传输拦截，断言确实拦到状态和启动回执，再执行测试。前端故障注入/单测与真实原生结果分别记录，不以Mock代替坚果云或原系统现场。
5. **测试设施失败保留**：首次原生脚本误将save_project的void回执当成Project，范围断言超时；修正为list_projects读取，不改业务以迁就测试。不可写invoke使故障断言超时，修正后受控失败/延迟均实测；清理该失败测试唯一已知QA凭据。首次Rust沙箱凭据写入失败，原生权限重跑通过。替换指南图期间构建遇到文件暂不可用，图片归位后完整check重跑成功。版本修改时发现误替换依赖同号，检查diff后立即恢复，锁文件最终只更新根包版本，未更换依赖。

## 已执行检查

| 状态 | 实际命令 | 结果 |
| --- | --- | --- |
| PASS | `npm run check` | 类型、ESLint、53文件364个Vitest、Vite生产构建，退出0；最终bundle再次类型/构建 |
| PASS | `npm run format:rust` / `npm run check:rust` | fmt及all-targets Clippy `-D warnings`，退出0 |
| PASS | `npm run test:rust`（Windows原生权限） | 168通过、0失败、7独立服务器用例在下一行单独运行，退出0 |
| PASS | `$env:RESEARCHCLIP_QA_DAV_BASE='http://127.0.0.1:1461/'; node scripts/with-rust.mjs cargo test --manifest-path src-tauri/Cargo.toml --locked -- --ignored --test-threads=1` | 独立WsgiDAV4.3.3原子条件写入/内容ETag配置1461、不可信TLS1462，7通过，退出0；不是默认WsgiDAV或坚果云认证结论 |
| PASS | `npm --prefix artifacts/qa-v310/npm-clean ci --registry=https://registry.npmjs.org --no-audit --no-fund` | 独立目录干净安装303包，退出0 |
| PASS | `npm --prefix browser-extension test` / `npm --prefix browser-extension run build` | 5通过及独立构建，退出0；发布附件仍保留已验原版ZIP与扩展ID |
| PASS | `npm run tauri -- build --debug --no-bundle` | Windows v3.1.0 debug EXE及嵌入前端，退出0 |
| PASS | `node scripts/sync-settings-native-smoke.mjs v310-accepted2 --guide` | 5组真实/注入分开记录，退出0；见下表 |
| PASS | `node scripts/guide-native-smoke.mjs v310-final --verify-only` | 5组原生回归，23张本地图片解码、系统Markdown保存、SQLite/标注/卡片/对比，退出0 |
| PASS | `node scripts/window-capture-native-smoke.mjs v310-final` | 14项实际WGC/窗口/PNG/剪贴板/编辑器/20次资源检查，退出0 |
| PASS | `node scripts/material-ui-smoke.mjs v310-final` / `node scripts/guide-reader-smoke.mjs v310-final` / `node scripts/sync-ui-smoke.mjs v310-final` | Edge实际渲染8/9/5组，深浅主题、480–1440宽度、目录搜索/图片放大、Markdown草稿保留；不是原生服务连接验收，退出0 |

独立服务器启动命令：`artifacts/qa-sync/venv/Scripts/python.exe artifacts/qa-v310/dav-server.py --serialize-writes`与`--tls`。脚本为`scripts/qa-webdav-server.py`的隔离路径副本，使用旧测试证书，不改系统信任；测试根/服务器记录新建在`artifacts/qa-v310/dav-fixture`，只关闭本轮创建的服务器。TLS负向用例产生预期握手拒绝日志。

| 状态 | 设置验收 | 实际证据性质 |
| --- | --- | --- |
| PASS | 空表单错误当前可见，保存可重试 | 原生WebView2实际字段与焦点/视口 |
| PASS | 保存失败保留账号/密码/目录，按钮释放 | 明确标记的UI传输错误注入，未声称OS实际写入失败 |
| PASS | 后台状态读取延迟时保存已完成、密码清空 | 实际Rust/Windows凭据/SQLite保存；另行确认拦截状态传输 |
| PASS | 启动回执等待时可停止并恢复保存 | 实际本机TLS延迟任务与取消；另行确认前端回执被拦截 |
| PASS | 凭据删除后提示并可修复；范围保存 | 只删除/重新创建已知测试凭据，读取真实SQLite项目 |
| PASS | 关闭/重启恢复，不自动联网 | 实际EXE退出0与重启；配置/范围/凭据可用，job idle，SQLite无测试账号/密码 |

最终[机器可读摘要](evidence/v310-checks.json)已记录上述实际结果。全部相关命令最终退出0；初次失败未隐藏。最后统一错误颜色为现有主题角色后，重复深浅色六宽度检查和正式打包/安装验收。

| 状态 | 最终发布门槛 | 结果 |
| --- | --- | --- |
| PASS | `npm run desktop:bundle` | Windows优化EXE与NSIS，退出0，无UNK→NSS标记警告 |
| PASS | `D:/anaconda3/python.exe artifacts/qa-v310-final/release-worker-probe.py` | 实际优化EXE捕获626×473，绿/红各137574像素，退出0；未初始化Tauri/用户数据库；Pillow旧getdata用法弃用提示保留，不影响断言 |
| PASS | `scripts/installer-smoke.ps1 -RunId v310-accepted -TargetDir src-tauri/target` | 实际Unicode/空格目录安装、版本/图标/厂商/完整payload核对；忙碌安装与卸载均退出2正确拒绝；默认卸载保留10份既有资料摘要及无关文件，无测试安装残留 |
| PASS | `npm run check:docs` | 本地链接/围栏、版本/锁文件一致、capability和既有迁移检查，退出0；这是静态检查，不代替运行验收 |

没有在真实资料库启动最终生产GUI；原生交互采用同源码的隔离debug EXE，生产worker、安装包和静态/构建验证另行执行。坚果云真实认证与两台实体设备仍未验证。当前未复现新增P0/P1发布阻断项，建议发布有明确边界的修复版本，目标服务商仍需按下列清单验收。GitHub上传在随后发布回执中单独记录，不预先标为成功。

## 人工验收（NOT_RUN）

1. 下载新EXE/安装包，保存并退出旧实例后打开，设置→跨设备同步；输入自己的HTTPS地址、用户名、应用授权密码，保存后应看到成功提示。
2. 修改一项账号或范围，不先保存时测试/同步禁用且原因可见；保存后勾选探针再测试。不要在聊天提供密码。
3. 在坚果云账号执行真实认证、目录与安全条件写入探针；失败按提示处理，不禁用TLS校验。实际服务商认证目前BLOCKED（无本机配置凭据）。
4. 两台真实设备测试少量项目、标注历史及Markdown双向/冲突；未在真实用户资料执行破坏性同步。
5. 125%/150%混合DPI、提权/安全桌面、HDR、RDP、其他截图目标/粘贴软件、macOS矩阵另行验收。

## 已知限制与下一步

既有QA-001/002/008、未签名产物、前端大包提示与公式OCR错误风险继续保留。网络停止等待当前请求最多约30秒；重启不自动联网。没有降低条件写入安全门槛，没有上传SQLite、真实资料或凭据；建议先用测试项目在目标服务商执行上述人工验收。

## 修改文件

- `CHANGELOG.md`
- `README.md`
- `docs/ARCHITECTURE.md`
- `docs/RELEASE-NOTES-3.1.0.md`
- `docs/ROADMAP.md`
- `docs/USER_GUIDE.md`
- `docs/images/v310-sync-actions.png`
- `docs/images/v310-sync-settings.png`
- `docs/qa/RELEASE-3.1.0.md`
- `docs/qa/TEST_REPORT.md`
- `docs/qa/evidence/v310-checks.json`
- `package-lock.json`
- `package.json`
- `scripts/guide-native-smoke.mjs`
- `scripts/guide-reader-smoke.mjs`
- `scripts/sync-settings-native-smoke.mjs`
- `src-tauri/Cargo.lock`
- `src-tauri/Cargo.toml`
- `src-tauri/src/commands/sync.rs`
- `src-tauri/src/services/sync/runtime.rs`
- `src-tauri/tauri.conf.json`
- `src/features/guide/GuideView.tsx`
- `src/features/sync/SyncSettings.tsx`
- `src/features/sync/api.test.ts`
- `src/features/sync/api.ts`
- `src/features/sync/form.test.ts`
- `src/features/sync/form.ts`
- `src/features/sync/store.test.ts`
- `src/features/sync/store.ts`
- `src/styles.css`

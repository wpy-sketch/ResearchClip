# ResearchClip v3.1.8 / A12 验收

本轮范围：9项命令、Ctrl+Shift+P、7种受限动作、3个预设、自定义有限组合、状态/取消/失败重试/校对和配置恢复；更新界面、18章110示例指南及完整配套Release。架构与数据契约见[命令与工作流](../COMMAND-WORKFLOWS.md)。复用既有服务/设置，无新依赖或SQLite迁移，schema保持10。

## 实现与修改范围

新增features/commands（model、runner、runtime、commands、CommandCenter及测试）；App/Toolbar/SettingsView接入口，styles增加命令与步骤布局。Rust settings增加有界workflows配置与保留快捷键检测；workflow_input验收辅助程序只增加固定palette组合，继续校验目标PID。指南和发布工具按3.1.8维护。

## 实际检查记录

环境：Windows x64、Node22.22.2、Rust1.99、WebView2；全部资料为隔离目录中的练习图片/配置。原生工作流使用真实Windows OCR及已有本地Python/ONNX，截图仅框选测试自建Edge窗口。

| 命令 | 实际结果 |
| --- | --- |
| `npm run check` | 退出0；62文件420项测试，TypeScript、ESLint和Vite生产构建通过；命令执行器/适配器17项纳入其中 |
| `npm run format:rust` / `npm run check:rust` | 均退出0；Rust格式、全部targets严格Clippy |
| `npm run test:rust`（RUST_TEST_THREADS=1） | 退出0；203通过、0失败、7忽略；7项需独立WebDAV/TLS环境，本轮未运行 |
| 隔离离线 `npm ci` | 退出0；303包、0漏洞；锁文件除根版本号外与v3.1.7一致，无新增依赖 |
| `python scripts/test-release-companions.py` | 退出0；3项附件哈希/ZIP安全测试 |
| `node scripts/links-worker-smoke.mjs` | 退出0；2组生产Worker检查 |
| `node scripts/commands-native-smoke.mjs a12-context` | 退出0；16组真实Windows交互：搜索/SendInput快捷键、3预设、OCR/公式校对、取消/重试、写入失败、最近文档、配置重启及卡片上下文 |
| `node scripts/anchors-native-smoke.mjs v318-regression` | 退出0；11组A11锚点、定位、草稿保留、导出、失效及重启回归 |
| `node scripts/guide-native-smoke.mjs v318-final --verify-only` | 退出0；6组旧截图/标注/卡片/对比/Markdown和原生指南检查 |
| `node scripts/guide-reader-smoke.mjs v318-final` | 退出0；11组目录/搜索/焦点/18章110示例31图，明暗主题7宽度及600px高度 |
| `node scripts/clipper-native-smoke.mjs v318-packaged --packaged-host` | 退出0；7组，实际已发布Host→本版IPC→SQLite/文件，未修改真实浏览器注册，不宣称本轮验证浏览器activeTab |
| `npm run tauri -- build --debug --no-bundle` / `npm run desktop:bundle` | 最终均退出0；实际静态前端EXE与NSIS构建 |
| `& .\scripts\installer-smoke.ps1 -RunId v318-final -TargetDir src-tauri/target` | 退出0；4组安装/忙碌拦截/卸载检查，13个既有用户数据文件哈希未变，已清理本次测试安装 |

最终安装包SHA256：`a8aa80d108f9a758512a7fb458490766243d6034ff8fd0b68c6cc1d8924de5e6`。机器可读记录见[检查摘要](evidence/v318-checks.json)。`npm run check:docs`检查本地链接/围栏、版本/权限/schema一致性；该检查不替代运行验收。

16组原生工作流同时验证：重复触发受单运行约束、没有Markdown时截图前停止、校对前未插入且编辑后清除确认；真实模型缺失后只重试公式步骤，未再选择图片；外部损坏测试配置时保存失败，文件与表单均保留，恢复测试副本后再成功保存；重启不自动执行。文档切换/忙碌插入/取消期间失败保留另有适配器单元测试；不把Mock称为原生测试。

### 失败与修正记录

- 首次完整前端在沙箱临时模块文件缺失导致42个suite加载失败；正常权限最终420项通过。Rust沙箱设置原子写入10项失败，正常权限最终203项通过；附件单测沙箱临时ZIP写入2项失败，正常权限3项通过。未删断言或业务校验。
- 原生早期定位器把背景select选项计入命令结果，改为限定命令listbox；另一次前台不属于测试PID，辅助程序拒绝发送键盘，改为先调用既有恢复主窗口接口再核验PID。浏览器标题尚未注册时等候枚举就绪。
- 真实OCR把品牌名中的l认作I，保留此误识别事实；测试核对其余固定文字并人工修订，不宣称识别正确率。
- 自查修正卡片详情命令可能回退旧预览的问题；新增适配器测试及原生卡片OCR用例，最终16组通过。取消期间真实错误保留到结果检查，不静默吞掉。
- 初次NSIS在沙箱报系统DLL重定位错误623，正常权限完整构建退出0；保留构建失败记录，未降低打包要求。

## Windows人工验收与已知边界

本轮未单独进行用户人工操作验收；以上为实际Windows自动化。macOS、多显示器/高DPI全部组合、其他软件抢占快捷键与全部第三方WebDAV/编辑器尚未完成人工验收。既有未签名与大chunk提示保留。Windows文字OCR取消需等待既有引擎返回；工作流默认整图和已有语言，精确区域/语言选择使用原面板。工作流配置恢复，未完成执行不跨重启恢复。v3.1.7曾记录一次偶发Markdown文件操作错误，文件/草稿未丢失；本轮多次原生保存未复现，未将其宣称为已修复。

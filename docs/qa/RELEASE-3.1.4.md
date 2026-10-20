# ResearchClip v3.1.4 验收

环境：2026-10-21，Windows x64；现有Node、Rust、Tauri2/WebView2。所有运行测试使用隔离资料。

## 范围

修改Sidebar、GuideView、styles、USER_GUIDE、版本清单与浏览器冒烟脚本。沿用模块化单体和既有状态服务。无新增依赖、数据库迁移、PDF或独立备份系统。

## 本轮实际检查

- PASS `npm run check`：退出0；56文件、381测试通过，类型、ESLint及Vite构建通过。保留既有大chunk警告。
- PASS `npm run check:docs`：退出0；103文档、597本地链接；版本与权限一致。
- PASS `node scripts/guide-reader-smoke.mjs v314-final`：退出0；9组浏览器交互检查，15章27图，7种宽度和深浅色，侧栏顺序断言通过。
- PASS `npm run format:rust`：退出0。
- PASS `cargo build --manifest-path src-tauri/Cargo.toml --locked`：通过；原生验收使用隔离debug程序。
- PASS `npm run desktop:bundle`：退出0；最终v3.1.4优化EXE与NSIS安装包生成。
- 原生初次FAIL：缺少开发服务器，WebView2显示连接拒绝；第二次同样原因，截图证据保留；启动Vite1420后重新验证。
- GitHub初次直连FAIL：Schannel与OpenSSL TLS握手失败；使用本机已有7897代理且验证证书后，远端main读取成功。

- PASS `node scripts/guide-native-smoke.mjs v314-stable --verify-only`：退出0；真实Windows WebView2、SQLite、原生Save/Open，5组通过；素材/标注/卡片/对比/Markdown、15章27图及1180/760/480验证。前次运行期间Vite热更新导致关闭文档定位失败，保持文件稳定后复验通过。

构建和自动化不代替人工硬件矩阵。Rust存储逻辑未变，本轮未重复全量Rust测试。

## 未执行项与风险

Windows人工完整工作流、物理高DPI/多显示器和macOS未执行。安装包未签名。两个原有未跟踪发布脚本不包含在本版提交中。

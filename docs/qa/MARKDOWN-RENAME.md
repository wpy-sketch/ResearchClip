# Markdown 文件命名验收

日期：2026-10-17，Windows11 x64 / Node22.22.2 / npm10.9.7 / Rust1.99 MSVC / Tauri2.12.3。范围仅自定义初始文件名、文件名右键菜单和安全重命名，沿用现有Markdown工作流，不修改数据库/原始截图。前两阶段记录保留。

## 使用

新建空白Markdown自动显示untitled.md，图文编排显示未命名文档.md。右键文件名→“重命名文件”，输入名称确认；也可单击文件名或使用键盘菜单入口。没有.md时自动补齐，文件名与正文标题独立。

未保存时只改内存名称，不创建文件；首次保存/另存对话框采用建议名称，实际选择后以真实文件名为准。已保存时只在同一目录原位改名，不覆盖其他文件、不修改正文或assets；有脏草稿仍显示未保存，后续Ctrl+S写入新路径。取消、失败或外部文件被修改时保留文档与草稿。

## 实际检查

| 命令 | 状态与结果 |
| --- | --- |
| `npm run check`首次 | FAIL/1，ESLint no-control-regex；改为字符码检查保留控制字符拒绝，不禁用规则 |
| `npm run check`修正后 | PASS/0，29文件214项；TypeScript、ESLint、Vitest及Vite生产构建 |
| `npm run format:rust` | PASS/0，fmt --check |
| `npm run check:rust` | PASS/0，Clippy --all-targets -- -D warnings |
| `npm run test:rust` | PASS/0，97项，0失败/忽略；含3项新增实际临时文件/文件名测试 |
| `node scripts/with-rust.mjs tauri build --debug --no-bundle` | PASS/0，实际运行Windows WebView2 |
| `node scripts/document-rename-native-smoke.mjs rename-20261017-a` | PASS/0，5组，errors=[]；[原始结果副本](evidence/markdown-rename.json) |
| `node scripts/split-native-smoke.mjs rename-20261017-split` | PASS/0，原有动态分栏11组；[结果副本](evidence/markdown-rename-split.json) |
| `node scripts/document-native-smoke.mjs rename-20261017-blocks` | PASS/0，原有图文编排/保存/重启8组；[结果副本](evidence/markdown-rename-blocks.json) |
| `npm run check:docs`、`git diff --check` | PASS/0，29文档161条本地链接；文档权限/版本检查一致，diff无空白错误 |
| 独立target/document-rename的`npm run desktop:build` | PASS/0，独立目录构建1分53秒，保留旧版本 |

原生测试使用独立artifacts目录和明确的测试PNG；SQLite、Windows重命名和系统文件对话框为真实操作，不能据此声称已重新验证真实屏幕捕获、高DPI或macOS。

PASS证据：新文档改名不生成文件；中文/空格名称与自动扩展名；重命名不改正文标题；真实已保存.md消失于旧路径、出现于新路径；未保存草稿及资产hash不变；Ctrl+S保存新路径；真实同名目标与外部修改均拒绝，两个文件/草稿保留；非法路径及CON等名称禁止提交；取消/菜单Escape不修改资料；重开加载实际名称和图片引用。Rust额外直接调用无覆盖MoveFileW helper检验目标已存在时不替换。

## 限制和人工验收

- Windows仅大小写改名按同名拒绝，可先改中间名称再改回目标大小写。macOS已保存文件原位重命名NOT_IMPLEMENTED，明确报错，可用另存；无macOS运行证据。
- 手动使用中文输入法命名、键盘Shift+F10菜单、不同DPI/主题、只读/文件占用/网络盘的失败路径均NOT_RUN；自动填充中文不等同中文输入法验收。
- 手工步骤：新建→右键文件名改名→Ctrl+S检查建议名称；插图保存后再次改名→检查旧文件消失、assets不变→修改正文并保存→关闭重开。再尝试同名/非法名称与取消，确认不覆盖文件、不丢草稿。
- 历史托盘/退出及旧目录导出自动化阻塞沿用原报告，本轮未修复；没有发布新GitHub Release或安装器。

## 本轮文件

新增：src/features/document/DocumentFileName.tsx、file-name.ts、file-name.test.ts，scripts/document-rename-native-smoke.mjs，本文及三份evidence JSON。

修改：src/features/document/MarkdownPanel.tsx、api.ts、store.ts、store.test.ts；src/styles.css；src-tauri/src/services/markdown_document.rs、commands/document.rs、src/lib.rs、build.rs、capabilities/main.json、Cargo.toml；scripts/check-docs.mjs；README.md、docs/ARCHITECTURE.md。Cargo只启用已有windows库的文件系统feature，未新增依赖包/版本，无Cargo.lock或数据库迁移变更。保留了此前用户未提交工作。

## 最终记录

Windows生产构建产物：src-tauri/target/document-rename/release/researchclip.exe；15,634,944字节；SHA256=4DE78777C89F2FBAA3439A04F8E16EF3E2DEF987DA6CDAA06BF5E9C00AFDAF6D。未签名、未发布新版安装器/Release；请退出旧普通实例再运行。功能原生验收使用隔离debug版，不声称已用用户真实资料验证生产程序。

最终文档检查PASS/0，29文档161链接；git diff --check及Cargo.lock/迁移无变化检查PASS/0。此前214项前端、97项Rust、原生重命名5组和旧流程19组均PASS，无未解决的本轮自动化失败。早期lint失败保留，未执行平台/人工项没有标PASS。阶段完成后停止。

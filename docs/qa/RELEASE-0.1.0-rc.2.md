# v0.1.0-rc.2 发布核对

日期：2026-10-17，Asia/Hong_Kong。用户授权上传新版本。当前版本为Windows预发布，程序内版本仍0.1.0。此次只发布源码和最新EXE，不公开39GB缓存归档，不包含新NSIS安装器，也不覆盖rc.1。

## 本轮实际检查

| 命令 | 结果 |
| --- | --- |
| `npm run check`首次 | FAIL/1：新增重命名原生验收脚本originals变量未使用；补齐原图hash断言，不禁用lint |
| `npm run check`修正后 | PASS/0：29测试文件214项，TypeScript、ESLint、Vitest、Vite构建 |
| `npm run format:rust` | PASS/0 |
| `npm run check:rust` | PASS/0，all-targets -D warnings |
| `npm run test:rust` | PASS/0，97项、无失败/忽略 |
| `node scripts/document-rename-native-smoke.mjs publish-rc2-final` | PASS/0，5组，errors=[]；新增原图hash断言，见 [结果](evidence/rc2-rename-prepublish.json) |
| `git diff --cached --check`首次 | FAIL/2，三个此前未跟踪文档的末尾多余空行，已清理并重跑；不改正文 |
| `npm run check:docs`、`git diff --check` | PASS/0，31文档170条本地链接，权限/版本一致，无diff空白错误 |

复用了此前完成的Windows生产构建，核心业务源码没有新增修改：src-tauri/target/document-rename/release/researchclip.exe，15,634,944字节，SHA256=4DE78777C89F2FBAA3439A04F8E16EF3E2DEF987DA6CDAA06BF5E9C00AFDAF6D。此次修改原生测试脚本及发布文档，不改业务行为。原生测试是隔离debug应用与明确测试PNG，不代替生产版/真实资料或屏幕硬件全流程。

## 提交范围和安全边界

包括全部未被.gitignore排除的项目源码、资源、配置、两个锁文件、文档和QA证据。排除node_modules、.tools、src-tauri/target、dist、artifacts、真实/测试运行数据库、浏览器会话与完整归档；本地文件不删除。Git凭据仅由Credential Manager读取并在进程内用于GitHub，不写入文档或日志。常见私钥/令牌格式扫描只作为预检查，不能冒充全面安全审计。

已核对远程main为592ba6f6c638ecca2508e70551b355e4da36b07d，尚无rc.2标签；使用普通快进推送，禁止force。发布先创建草稿、上传EXE及SHA256SUMS/SOURCE，再公开为prerelease；旧rc.1保留。

## 已知问题

历史退出句柄/托盘问题、完整DPI/多屏、中文输入法、长稳/性能、生产版全流程和macOS仍未完成；不能以此次检查宣布完整正式发布验收通过。Windows仅大小写重命名限制、外部Markdown附件及公式排版限制见 [发布说明](../RELEASE-NOTES-0.1.0-rc.2.md)。

## 发布结果

尚待实际推送/上传后记录，不提前标成功。

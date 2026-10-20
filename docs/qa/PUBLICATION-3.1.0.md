# ResearchClip v3.1.0 正式发布回执

执行：2026-10-20，Asia/Hong_Kong。实际 GitHub API、Git推送和匿名下载证据；不把发布标签当作服务商/硬件矩阵通过。

- PASS：[正式Release](https://github.com/wpy-sketch/ResearchClip/releases/tag/v3.1.0)；draft=false、prerelease=false、Latest=true。
- PASS：源码提交`8309b37057b73b705cf33e127183ad66423a7ba7`，不可变标签v3.1.0；原子推送main与tag，远端API核对提交一致。发布回执另行提交，不移动源码标签。
- PASS：6个附件均由GitHub返回uploaded及匹配digest，随后**无Authorization/Cookie**匿名下载HTTP200，字节数及SHA256逐一一致。
- PASS：发布前19个旧Release的ID/标签/预发布状态/附件ID/大小/digest发布后保持一致，包括已完成的v3.0.1。没有覆盖旧内容。
- PASS：保留原版WebClipper v1.0.2 ZIP及原扩展ID；MathOCR附件为可选安装脚本，不含模型/Python运行时。
- 未上传真实资料、凭据、缓存、工具链、模型或测试工作区。

## 实际发布命令

1. `git commit -m "fix: restore responsive WebDAV settings for v3.1.0"`，退出0。
2. `node artifacts/publish-v310-final/prepare.mjs`，干净源码及已验安装包SHA核对，退出0。
3. `git tag -a v3.1.0 -m "ResearchClip v3.1.0 stable: WebDAV settings reliability"`；`git push --atomic origin HEAD:refs/heads/main refs/tags/v3.1.0`，退出0。
4. `D:/anaconda3/python.exe artifacts/publish-v310-final/publish_release.py`，创建草稿→上传→正式发布→匿名下载→旧Release核对，退出0。凭据只在进程内存使用；TLS验证启用，带身份请求不跟随重定向。

## 附件

| 文件 | 字节 | SHA256 |
| --- | --- | --- |
| ResearchClip-v3.1.0-windows-x64.exe | 24689664 | `2ec7d129cb732b19326197d954241238fa8fce0cf327d9410889a65c67e500eb` |
| ResearchClip_3.1.0_x64-setup.exe | 7929800 | `ca92000d83d9afcebf5cc18482ef1473d7565b422a7cbcbcbe164dccad101af1` |
| ResearchClip-WebClipper-v1.0.2-windows-x64.zip | 541479 | `1e456e8cc198f1b366afc47614cc5ad3dbf14410ad5a52aad21cf61d551cd4ce` |
| ResearchClip-MathOCR-Setup-v1.0.4.zip | 10948 | `c263e29d81ec31ede7199f2902ddba443114828c89bd48194c4549cf9209693d` |
| SOURCE.txt | 1938 | `25e030b6c08be61ed3d2638885c119a5d2b0cd36445ed7170c79a9cffea72468` |
| SHA256SUMS.txt | 495 | `224f3a3b6c21d582be5a0efd73efa9a3a614e97c57c6582a806322e98ccfeed8` |

[公开核验摘要](evidence/v310-publication.json) · [实际测试与修改文件](RELEASE-3.1.0.md) · [操作指南](../USER_GUIDE.md)。本机安装包：`src-tauri/target/release/bundle/nsis/ResearchClip_3.1.0_x64-setup.exe`。

## 已知边界

前端364、Rust168+7隔离集成、扩展5、原生设置/截图/Markdown回归、实际安装卸载及10份既有资料摘要通过，详见专项报告。坚果云真实账号认证和两台实体设备仍未验收；物理125%/150%混合DPI、提权/HDR/RDP/完整第三方矩阵及macOS仍需人工测试。最终生产GUI未在真实用户资料库启动，隔离debug交互、优化捕获worker和NSIS安装是不同证据。产物未签名、前端大包提示、旧QA-001/002/008与OCR准确性限制保留。

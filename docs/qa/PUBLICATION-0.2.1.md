# v0.2.1 GitHub 发布回执

PASS：[ResearchClip v0.2.1](https://github.com/wpy-sketch/ResearchClip/releases/tag/v0.2.1)，Release ID 416295372；发布时间 2026-10-18T07:09:49Z，香港时间 2026-10-18T15:09:49+08:00。draft=false、prerelease=false、Latest=true。

源码提交：`a65dbcfdb9db1f27f350e2f85b082e795a9ccbdc`，带注释标签v0.2.1指向同一提交；main已推送。本回执作为单独文档提交，不移动已发布标签。

| 附件 | 字节 | SHA256 |
| --- | --- | --- |
| ResearchClip-v0.2.1-windows-x64.exe | 19106816 | `e23cbfa89b887d47c45d606ce91585e04c9f13eeb3e8dce35e7a625f2c393029` |
| SOURCE.txt | 1478 | `845797ea34e00d337414ebaabb2cc5ae1986e1468f28fd132a20d7d4467fa998` |
| SHA256SUMS.txt | 181 | `1e9a26e929093ecda4ba1d17d10df0c5f869014fbde77244097ce766153da62e` |

[直接下载Windows x64 EXE](https://github.com/wpy-sketch/ResearchClip/releases/download/v0.2.1/ResearchClip-v0.2.1-windows-x64.exe)。本地可启动程序：`src-tauri/target/release/researchclip.exe`；版本0.2.1，未签名，需要WebView2。本轮没有新NSIS安装包。

实际执行：`git -c http.proxy=http://127.0.0.1:7897 push --atomic origin HEAD:refs/heads/main refs/tags/v0.2.1`退出0；受控API先核对main/标签源码提交及9个旧Release，再创建草稿、上传3附件、核验服务器SHA256，发布普通Latest。3个公开下载URL均以**不带认证**方式返回HTTP200，字节数与SHA256和本地全部相同。此前9个Release的ID、预发布状态和附件ID/大小/摘要保留；未覆盖旧发布。

[公开元数据/下载校验证据](evidence/v021-publication.json)不含认证信息。Git仅49个源码、文档、锁文件和脱敏测试证据；无删除文件。运行缓存、工具链、浏览器目录、测试工作区和用户资料未上传。

[完整验收报告](RELEASE-0.2.1.md)：299项前端、125项Rust、67组隔离Windows原生及8组浏览器检查最终PASS，生产构建与版本校验通过。最初失败与拖放不稳定保留。第三方Markdown应用逐个打开、完整捕获/DPI/多屏、生产真实资料目录、安装卸载及macOS仍NOT_RUN，历史QA-001/002继续跟踪。普通Release状态不表示这些未运行项已通过。操作见[带图使用指南](../USER_GUIDE.md)。

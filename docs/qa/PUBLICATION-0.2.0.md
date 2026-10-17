# v0.2.0 GitHub 发布回执

PASS：[ResearchClip v0.2.0](https://github.com/wpy-sketch/ResearchClip/releases/tag/v0.2.0)，Release ID 415864695；发布时间2026-10-17T15:00:45Z，香港时间2026-10-17T23:00:45+08:00。draft=false、prerelease=false、Latest=true。

源码提交：`6389b7109203a4b8386f8f53aba6c192a7b6cc9d`，带注释v0.2.0标签已核对指向该提交；main已推送。此回执作为单独文档提交，不移动已发布源码标签。

| 附件 | 字节 | SHA256 |
| --- | --- | --- |
| ResearchClip-v0.2.0-windows-x64.exe | 17945600 | `b76afddd222eb2ea62c2958b77e8872c80be83750a2928a6fad95a5234fcff2e` |
| SOURCE.txt | 1633 | `8e371e9174a2658b64f67cc7daa55cc22a4e410059e80f4a149d19d1cb22c088` |
| SHA256SUMS.txt | 181 | `5cf5dabc2f9c6657f1f54a509bd4c8b9aac637f68c414619cfe855ee74de5b53` |

[下载Windows x64 EXE](https://github.com/wpy-sketch/ResearchClip/releases/download/v0.2.0/ResearchClip-v0.2.0-windows-x64.exe)。需要WebView2，未签名。本地程序：`src-tauri/target/release/researchclip.exe`。本轮没有新NSIS安装包。

实际操作：`git -c http.proxy=http://127.0.0.1:7897 push --atomic origin HEAD:refs/heads/main refs/tags/v0.2.0`退出0；受控API创建草稿、依次上传3附件并核验服务器SHA256，再发布为普通Latest。通过不带认证的公开下载URL下载3附件，均HTTP200、大小与SHA256相同。之前8个Release的ID、预发布标识和附件ID/大小/摘要均保留。

[公开元数据与下载证据](evidence/v020-publication.json)不含认证信息。仓库只上传源码、文档、测试与锁文件；缓存、工具链、浏览器配置、测试图片资料和真实用户数据未公开。

数据库变更/失败修复/75个开发修改文件/实际检查和未执行人工清单见[0.2.0验收报告](RELEASE-0.2.0.md)，操作见[完整指南](../USER_GUIDE.md)。SQLite v3自动备份升级保留原图路径和既有资料；旧版不能写v3。完整生产/硬件/长稳/安装卸载等人工矩阵未全面验收，普通发布标识不表示这些项目已通过。

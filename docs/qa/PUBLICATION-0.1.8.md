# v0.1.8 GitHub 发布回执

PASS：2026-10-17T14:16:21Z（香港22:16:21）发布 [ResearchClip v0.1.8](https://github.com/wpy-sketch/ResearchClip/releases/tag/v0.1.8)，Release ID 415841214；draft=false、prerelease=false、Latest=true。没有覆盖旧版；原有7个Release的ID、预发布标识及附件ID/大小/摘要均与发布前一致。

源码提交：`ee8552496c7d233692d64565e41b93e276a77587`；v0.1.8带注释标签指向该提交，源码已推送main。发布回执另行提交，不移动源码标签。

附件：

| 文件 | 字节 | SHA256 |
| --- | --- | --- |
| ResearchClip-v0.1.8-windows-x64.exe | 17254400 | `f5475732e108856d1a9e6406160f81cfc2494037862f727e900d5f8f39c05c94` |
| SOURCE.txt | 1292 | `e9ee48151e46502c97285f111104479bc2b90718cd6c07d5ed625d34fdbb6b38` |
| SHA256SUMS.txt | 181 | `8ac5201e5fc7d8d9315b70224c8e0b1563bd57a9dd6e03d4f72c8a70bce41306` |

[下载Windows x64 EXE](https://github.com/wpy-sketch/ResearchClip/releases/download/v0.1.8/ResearchClip-v0.1.8-windows-x64.exe)。需要WebView2，未签名，无本轮新NSIS。项目本地构建位于 `src-tauri/target/release/researchclip.exe`。

实际执行：`git push --atomic origin HEAD:refs/heads/main refs/tags/v0.1.8` 退出0；受控GitHub API创建草稿、上传3附件、核验服务器SHA256后转为普通Latest，旧版快照和源码标签再核验。匿名公开URL逐个下载并比较本地大小及SHA256，3项均PASS。EXE首次直连curl56、校验文件首次直连curl28，改用本机代理后匿名下载成功；保留网络失败尝试，不把直连当作通过。

[完整发布证据](evidence/v018-publication.json)只含公开元数据及校验结果，不包含令牌。源码/文档/锁文件已上传，缓存、工具链、测试资料和用户数据没有公开。

实际功能检查、58个开发修改文件、失败修复和未执行人工矩阵见[验收报告](RELEASE-0.1.8.md)，使用见[完整指南](../USER_GUIDE.md)。本轮无新未解决自动化FAIL；完整生产用户流程、安装/卸载、高DPI/多屏、长稳和macOS仍NOT_RUN，历史QA-001/002不据此关闭。

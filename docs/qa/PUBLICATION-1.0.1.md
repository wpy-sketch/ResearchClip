# v1.0.1 GitHub发布回执

PASS：[ResearchClip v1.0.1](https://github.com/wpy-sketch/ResearchClip/releases/tag/v1.0.1)，Release ID 416461690，实际发布时间2026-10-18T13:19:07Z，draft=false、prerelease=false、Latest=true。

源码与带注释标签固定在`c04678f95e39e29d31555bdb2821d648d8d9273c`；随后仅补文档回执及编号更正，不移动标签、不替换附件。

| 附件 | 字节 | SHA256 |
| --- | --- | --- |
| ResearchClip-v1.0.1-windows-x64.exe | 20113920 | `eed8912073448f3d41a4489b137e672a57921319dfd8edcc2fc41eed1795c164` |
| ResearchClip_1.0.1_x64-setup.exe | 6396745 | `b6ebd9876dd69ebbf15d5830aee2fe0265f52f98d0c5c5534ca7f0b97717b2af` |
| SOURCE.txt | 1718 | `6e0e5726129c2fd5d71e0d6749153eabc29013b4bc085a8219256eff9aeed8b2` |
| SHA256SUMS.txt | 278 | `e0c3d9134b2cf8f8dc44d29e4313d477efa7aae04358787275fddc502f26bc34` |

[独立EXE](https://github.com/wpy-sketch/ResearchClip/releases/download/v1.0.1/ResearchClip-v1.0.1-windows-x64.exe) · [NSIS安装包](https://github.com/wpy-sketch/ResearchClip/releases/download/v1.0.1/ResearchClip_1.0.1_x64-setup.exe)。未签名，需要WebView2；本地路径分别为`src-tauri/target/release/researchclip.exe`与`src-tauri/target/release/bundle/nsis/ResearchClip_1.0.1_x64-setup.exe`。

## 实际命令与核验

1. git fetch origin main --tags退出0；远程main与任务起始24b1f74一致，无远程改动被覆盖。提交50份源码/文档/锁文件/可公开测试摘要及自有指南图。用户资料、缓存、工具链、测试工作区未上传。
2. git push --atomic origin HEAD:refs/heads/main refs/tags/v1.0.1:refs/tags/v1.0.1退出0；新建标签，未force或改旧标签。
3. python artifacts/publish-v101/publish_release.py退出0；凭据只在内存/curl标准输入。核对main及标签，创建草稿并上传4附件，核对服务器size/digest后公开普通Latest。一次只读API传输TLS失败，GET重试成功；未重复创建/覆盖发布。
4. 四份附件均匿名HTTP200下载，大小与SHA256完全匹配。11个既有Release的ID、预发布属性、附件ID/名称/大小/digest逐一保持不变。[公开证据](evidence/v101-publication.json)不含凭据。
5. [实际验收](RELEASE-1.0.1.md)保留初期FAIL、最终306/132项测试、3/10屏实际证据及未运行矩阵。正式发布不替代实屏125%/150%、Chrome、macOS或长期稳定性验收。

## 文档编号更正与已知问题

本轮安装间歇问题统一编号为**QA-008/P2 OPEN**。初次v1.0.1标签和不可变SOURCE.txt误复用了历史已关闭QA-003的编号；这两个问题不同。主分支报告/证据和本回执已更正，不改动标签或附件校验。最终两个全新目录默认卸载PASS且7份资料摘要不变，不等于根因修复；QA-001/002历史问题仍保留。

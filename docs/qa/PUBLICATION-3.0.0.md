# v3.0.0 正式发布回执

PASS：[ResearchClip v3.0.0](https://github.com/wpy-sketch/ResearchClip/releases/tag/v3.0.0)，Release 417245961，2026-10-19T12:26:05Z。普通Latest，draft=false、prerelease=false；六个附件均匿名下载HTTP200、大小和SHA256一致，17个既有Release的ID/属性/附件不变。

最终源码注释标签固定于`31eb050629d8c44d5821a3d95013513dd98a47dc`。发布前补充复查发现截图完成预览被收起，停止公开并在隔离Windows复现后修复；再次通过338前端检查、原生5组及最终构建/安装。最初4c5dcbf只是本轮未公开候选，API确认草稿无附件后，使用精确旧SHA限定更新本轮标签至最终提交；没有覆盖任何既有正式版或其他标签。GitHub在公开草稿时一度关联为临时untagged标签；核验发现后只更正本次Release的tag_name到既有正确v3.0.0标签，未替换任何附件。随后再次核对canonical v3.0.0链接、Latest、17个旧版及六份匿名下载，均通过。回执为随后文档提交，不再移动标签或替换附件。

实际git源码提交、原子推送、python artifacts/publish-v300-final/publish_release.py及verify-canonical.py最终退出0。发布脚本首轮缺少新状态文件而未执行网络修改，初始化后重试；Schannel/代理和直连发生TLS握手失败，草稿未公开且未上传旧EXE。改用证书校验开启的Python TLS客户端完成上传；认证API禁止重定向，公开下载不带认证头或Cookie。原始失败日志保留，未禁用证书验证。

| 附件 | 字节 | SHA256 |
| --- | --- | --- |
| ResearchClip-v3.0.0-windows-x64.exe | 24257536 | `981062605955823549469ad08cfa3c13e303f33da6564d64d1eae7dd1c11c7b3` |
| ResearchClip_3.0.0_x64-setup.exe | 7732417 | `5c47a82b6faf08c8d4fadce7355edbc27b6d3734be946ac187d8756e24130555` |
| ResearchClip-WebClipper-v1.0.2-windows-x64.zip | 541479 | `1e456e8cc198f1b366afc47614cc5ad3dbf14410ad5a52aad21cf61d551cd4ce` |
| ResearchClip-MathOCR-Setup-v1.0.4.zip | 10948 | `c263e29d81ec31ede7199f2902ddba443114828c89bd48194c4549cf9209693d` |
| SOURCE.txt | 2079 | `12c706511e4a475989eada7afc1888543c363610faadf8907d698e9633d3d4ec` |
| SHA256SUMS.txt | 495 | `33f975e872af6518cbcb5c36ef74c287f53127234d6cb2e153d687bbe7bcf06e` |

原版WebClipper v1.0.2 ZIP/扩展ID不变，随此Release再次提供；可选MathOCR Setup保留v1.0.4，不附模型/Python。没有上传科研资料、凭据、缓存、工具链或测试工作区。

[完整测试/修改文件/人工清单](RELEASE-3.0.0.md) · [公开附件与下载核验证据](evidence/v300-publication.json)。338前端、158 Rust、7真实本地HTTP/TLS、5扩展测试；5组Windows同步配置/IPC/系统凭据/重启、5组素材/编辑/卡片/Markdown保存/21图指南原生回归；本地Edge布局8/指南9/同步UI5组通过。最终实际隔离安装/运行中拒绝/默认卸载通过，10个既有资料文件摘要不变。

**坚果云认证与条件写入兼容性仍BLOCKED**：需要用户在本机设置输入应用授权密码并通过安全探针；指定ResearchClip目录，不访问Zotero。物理双设备、完整DPI/多屏/托盘/浏览器工具栏和macOS矩阵NOT_RUN；历史QA-001/002/008保留。默认WsgiDAV曾实际拒绝为不安全，不将隔离参考配置的通过当作服务商通过。

程序未签名，需WebView2。EXE：`src-tauri/target/release/researchclip.exe`；NSIS：`src-tauri/target/release/bundle/nsis/ResearchClip_3.0.0_x64-setup.exe`。

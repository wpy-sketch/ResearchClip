# v2.0.0 正式发布回执

PASS：[ResearchClip v2.0.0](https://github.com/wpy-sketch/ResearchClip/releases/tag/v2.0.0)，Release 417000822，2026-10-19T07:10:14Z。普通Latest，draft=false、prerelease=false。源码注释标签固定于`37a3ce89f9e57d03f3f5d27b98db4ebf74e7948d`；回执为后续文档提交，不移动标签或替换附件。

实际git fetch/commit/annotated tag/atomic push与python artifacts/publish-v200/publish_release.py均退出0；没有force。六附件先草稿上传并逐份核对GitHub digest，再正式发布。六份均匿名HTTP200下载，大小和SHA256匹配。16个旧Release的ID/属性/附件保持不变。

原浏览器扩展v1.0.2 ZIP完整保留，扩展ID不变；可选MathOCR Setup保留v1.0.4原版。不上传Python、模型、工具缓存、测试工作区、凭据或真实科研资料。

| 附件 | 字节 | SHA256 |
| --- | --- | --- |
| ResearchClip-v2.0.0-windows-x64.exe | 21859328 | `d3ae676038390f6a64ae7b6ff57d88a57364bb6e0ebf813d9b8a65c24aaf0355` |
| ResearchClip_2.0.0_x64-setup.exe | 7054346 | `d64bbfe1a4cfd65c7f7fc88210013e6dcec80b3fdcb9b20335024a4df31c6367` |
| ResearchClip-WebClipper-v1.0.2-windows-x64.zip | 541479 | `1e456e8cc198f1b366afc47614cc5ad3dbf14410ad5a52aad21cf61d551cd4ce` |
| ResearchClip-MathOCR-Setup-v1.0.4.zip | 10948 | `c263e29d81ec31ede7199f2902ddba443114828c89bd48194c4549cf9209693d` |
| SOURCE.txt | 2105 | `5a595be17ea53facd19951a95ea0fccc2738b9b618aa44f785425c91384327f1` |
| SHA256SUMS.txt | 495 | `2b9ef7b52a01ed6418e671e32441faf80b3e6b3c15bef255840410081798e4d7` |

[实际测试、修改文件和人工清单](RELEASE-2.0.0.md) · [公开核验证据](evidence/v200-publication.json)。329前端、149 Rust、5扩展测试通过；9组指南与8组布局前端冒烟、隔离Windows生产安装/忙碌拒绝/卸载通过，9份已有资料摘要不变。新原生UI、高DPI、多屏、托盘和完整浏览器工具栏流程本轮NOT_RUN；macOS未验证。历史QA-001/002/008与公式识别误差保留；A06仍为设计提案，没有可用WebDAV。

程序未签名，需WebView2。本地EXE：`src-tauri/target/release/researchclip.exe`；NSIS：`src-tauri/target/release/bundle/nsis/ResearchClip_2.0.0_x64-setup.exe`。

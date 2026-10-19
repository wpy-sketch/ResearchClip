# v1.0.3 发布回执

PASS：[ResearchClip v1.0.3](https://github.com/wpy-sketch/ResearchClip/releases/tag/v1.0.3)，Release 416830450，2026-10-19T01:53:34Z，普通Latest，draft=false、prerelease=false。源码注释标签固定于`b4bc8bf65030d0f857850d96d1ed0b43fa26c8ae`。后续仅补回执，不移动标签或替换附件。

实际git fetch/commit/annotated tag/atomic push及python artifacts/publish-v103/publish_release.py均退出0，无force。4附件先草稿上传并核对服务端digest，再公开发布；4份均匿名HTTP200下载，大小和SHA256匹配。13个旧Release的ID/属性/附件保持不变。

| 附件 | 字节 | SHA256 |
| --- | --- | --- |
| ResearchClip-v1.0.3-windows-x64.exe | 20697088 | `d3490e9377b790bb2568243326cfe04ec328e62dc533d247021d8ca1920b3052` |
| ResearchClip_1.0.3_x64-setup.exe | 6607173 | `5c4b39e79c9c8386f06ceb0c7f9187a2fa570f3aaf43362944b44b8767512c2c` |
| SOURCE.txt | 1343 | `d698c9f45c1f7ac64ccba7e5848f559a288279379a2dca5af5bd2759ebb30665` |
| SHA256SUMS.txt | 278 | `6b48cda4c59d134915bc0f34015e2c281cf6bf23e67f7af74e6f4830ac76f22c` |

本地程序：`src-tauri/target/release/researchclip.exe`；安装包：`src-tauri/target/release/bundle/nsis/ResearchClip_1.0.3_x64-setup.exe`。未签名，需WebView2。

[实际验收与未运行矩阵](RELEASE-1.0.3.md) · [公开核验证据](evidence/v103-publication.json)。312前端/140 Rust测试、20查询/30真实OCR图与隔离安装通过；新UI、其他语言/系统/长期资源仍未验证，历史QA-001/002/008保留。

# v1.0.4 发布回执

PASS：[ResearchClip v1.0.4](https://github.com/wpy-sketch/ResearchClip/releases/tag/v1.0.4)，Release 416850762，2026-10-19T02:47:54Z，普通Latest，draft=false、prerelease=false。源码注释标签固定于`c2656ab84dac42f336e707ac6758f9d924a3ba30`。后续仅补回执，不移动标签或替换附件。

实际git fetch/commit/annotated tag/atomic push及python artifacts/publish-v104/publish_release.py均退出0，无force。5附件先草稿上传并核对服务端digest，再公开发布；5份均匿名HTTP200下载，大小和SHA256匹配。14个旧Release的ID/属性/附件保持不变。

| 附件 | 字节 | SHA256 |
| --- | --- | --- |
| ResearchClip-v1.0.4-windows-x64.exe | 21219840 | `77420012a1d0eb5882479f4d8ea1463846d4ee0e67b6d4cee78027745ede7d03` |
| ResearchClip_1.0.4_x64-setup.exe | 6711385 | `b4271364c5ede834564b0eba0879192e4ccc867166f18f2b87fc638757e61261` |
| ResearchClip-MathOCR-Setup-v1.0.4.zip | 10948 | `c263e29d81ec31ede7199f2902ddba443114828c89bd48194c4549cf9209693d` |
| SOURCE.txt | 2010 | `1b4773b9514e2d666b94b3206d78afdac541a7d6c285a90c11f3969c146b8df9` |
| SHA256SUMS.txt | 382 | `956e31b316149c63ceaae94d125d23ebb9e7c950756aa34de6da5cb1e4ff9cdd` |

本地程序：`src-tauri/target/release/researchclip.exe`；安装包：`src-tauri/target/release/bundle/nsis/ResearchClip_1.0.4_x64-setup.exe`。未签名，需WebView2。

[实际验收与未运行矩阵](RELEASE-1.0.4.md) · [公开核验证据](evidence/v104-publication.json)。320前端/143 Rust测试、11次实际本地公式推理及隔离安装通过；三类公式仍有内容识别错误，结果必须人工核对；新UI、其他语言/系统/长期资源仍未验证，历史QA-001/002/008保留。

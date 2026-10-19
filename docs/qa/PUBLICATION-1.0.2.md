# v1.0.2 发布回执

PASS：[ResearchClip v1.0.2](https://github.com/wpy-sketch/ResearchClip/releases/tag/v1.0.2)，Release 416818385，2026-10-19T01:25:56Z，普通Latest，draft=false、prerelease=false。

源码与注释标签固定于`499ca464ab70cd9dd02f537540f80f72dd2a25b6`，随后仅补文档回执。git fetch、commit、新标签及atomic push退出0，无force。python artifacts/publish-v102/publish_release.py退出0。先草稿上传、核对服务器size/digest再发布；5份附件匿名HTTP200下载均SHA256匹配。12个既有Release的ID/属性/附件全部保持不变。凭据未写入公开资料。

| 附件 | 字节 | SHA256 |
| --- | --- | --- |
| ResearchClip-v1.0.2-windows-x64.exe | 20295680 | `1069e6535c91bf2910eef19e496c8a008424658a3f9c7839062ecb4dc2ccdd78` |
| ResearchClip_1.0.2_x64-setup.exe | 6538665 | `967a027af119ebefb60aa90885167f7c33298a12c2d81950557655cb3e29cc61` |
| ResearchClip-WebClipper-v1.0.2-windows-x64.zip | 541479 | `1e456e8cc198f1b366afc47614cc5ad3dbf14410ad5a52aad21cf61d551cd4ce` |
| SOURCE.txt | 1429 | `e509a47c57fe6720e8e04c1a41041a6cd88f40796ef0c2547ebd2f3950d638bd` |
| SHA256SUMS.txt | 391 | `998a4645397a79c77a32b8c86eec8fac927cc7cee380ddd4f847b68d38892176` |

本地独立EXE：`src-tauri/target/release/researchclip.exe`；NSIS：`src-tauri/target/release/bundle/nsis/ResearchClip_1.0.2_x64-setup.exe`；扩展ZIP：`artifacts/v102-final/ResearchClip-WebClipper-v1.0.2-windows-x64.zip`。未签名，需WebView2。

[实际验收及未通过项](RELEASE-1.0.2.md) · [公开校验摘要](evidence/v102-publication.json)。浏览器工具栏/activeTab端到端仍BLOCKED，不能据普通Release标识标为通过。QA-001/002/008历史问题保留。

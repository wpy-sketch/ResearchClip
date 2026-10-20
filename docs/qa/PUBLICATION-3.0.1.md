# v3.0.1 正式发布回执

PASS：[ResearchClip v3.0.1](https://github.com/wpy-sketch/ResearchClip/releases/tag/v3.0.1)，Release 417896354，2026-10-20T00:59:19Z。发布时为普通Latest，draft=false、prerelease=false。六附件匿名HTTP200、大小和SHA256全部匹配；18个既有Release的ID/属性/附件保持。

注释标签固定源码 `6bc863a4a9f673ed3b26eb416f94126e308bd672`。先完成最终构建、原生及安装复验，再创建新标签和草稿；未移动标签、覆盖旧版或替换已发布附件。实际git源码提交/原子推送及python artifacts/publish-v301-final/publish_release.py退出0。回执作为后续文档提交，不改变固定源码标签。

| 附件 | 字节 | SHA256 |
| --- | --- | --- |
| ResearchClip-v3.0.1-windows-x64.exe | 24515584 | `3526bf520b788c3949a58db99e951abfdcde1403a2fb456b2307ba78b9d04d08` |
| ResearchClip_3.0.1_x64-setup.exe | 7777970 | `cca9094477285fb9de6abe818150c016699e763e6ff0bdaf941b1bcc9c1f9578` |
| ResearchClip-WebClipper-v1.0.2-windows-x64.zip | 541479 | `1e456e8cc198f1b366afc47614cc5ad3dbf14410ad5a52aad21cf61d551cd4ce` |
| ResearchClip-MathOCR-Setup-v1.0.4.zip | 10948 | `c263e29d81ec31ede7199f2902ddba443114828c89bd48194c4549cf9209693d` |
| SOURCE.txt | 1484 | `b19fb88a0b84bd7c4d3918c08b450c676ffbe4a2098bb36b5b861da025eb78fc` |
| SHA256SUMS.txt | 495 | `35c525b301ecf99d6cccbde97ace6a09dacd01696623d4c95a00f467d2dcea97` |

原版兼容WebClipper v1.0.2和扩展ID保持，可选MathOCR Setup v1.0.4不附模型/Python。未上传用户资料、缓存、凭据或测试工作区。产物未签名，需要WebView2。

[实际检查、修复、人工矩阵](RELEASE-3.0.1.md) · [匿名下载核验](evidence/v301-publication.json)。前端348、Rust167、扩展5；原生14项含真实Edge/遮挡/负坐标屏幕/保护/取消/DIB/PNG/编辑器和20次句柄127→127。最终NSIS安装、运行中拒绝、默认卸载及10份既有资料摘要通过。7个独立WebDAV服务器测试本轮未运行，125%/150%混合DPI/提权/HDR/RDP/macOS等矩阵未验收。已知历史QA和坚果云认证限制保留，不以正式标签替代这些验收。

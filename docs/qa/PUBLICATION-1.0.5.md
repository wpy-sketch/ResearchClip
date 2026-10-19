# v1.0.5 发布回执

PASS：[ResearchClip v1.0.5](https://github.com/wpy-sketch/ResearchClip/releases/tag/v1.0.5)，Release 416972653，2026-10-19T06:27:59Z，普通Latest，draft=false、prerelease=false。源码注释标签固定于`7e3e5cc0ab4a025e8e214ae4f637194cfa10aa0e`。后续仅补回执，不移动标签或替换附件。

实际git fetch/commit/annotated tag/atomic push及python artifacts/publish-v105/publish_release.py均退出0，无force。6附件先草稿上传并核对服务端digest，再公开发布；6份均匿名HTTP200下载，大小和SHA256匹配。15个旧Release的ID/属性/附件保持不变。浏览器插件保持v1.0.2原版ZIP字节及ID；可选MathOCR-Setup保持v1.0.4原版，无模型/运行时捆绑。

| 附件 | 字节 | SHA256 |
| --- | --- | --- |
| ResearchClip-v1.0.5-windows-x64.exe | 21552128 | `26a459221a7477ebfa23a5c899ebd652c9ddea21dc683f5aa726b3b364930265` |
| ResearchClip_1.0.5_x64-setup.exe | 6748091 | `e4ab9561024ddf2b16a2d8c84bbaa7c487662a4d86e63f8ff849ce2dcc8cde82` |
| ResearchClip-WebClipper-v1.0.2-windows-x64.zip | 541479 | `1e456e8cc198f1b366afc47614cc5ad3dbf14410ad5a52aad21cf61d551cd4ce` |
| ResearchClip-MathOCR-Setup-v1.0.4.zip | 10948 | `c263e29d81ec31ede7199f2902ddba443114828c89bd48194c4549cf9209693d` |
| SOURCE.txt | 1954 | `db6f3ba05b0fec5e9cb477d5d515b7d843e3bf8e10b21a0cc6349623b01b955e` |
| SHA256SUMS.txt | 495 | `8499948fc6b5c8bb791ac9f490801bea8f5eb86a89b2b19f403b7b0298597992` |

本地程序：`src-tauri/target/release/researchclip.exe`；安装包：`src-tauri/target/release/bundle/nsis/ResearchClip_1.0.5_x64-setup.exe`。未签名，需WebView2。

[实际验收、修改53文件及人工清单](RELEASE-1.0.5.md) · [公开核验证据](evidence/v105-publication.json)。325前端/149 Rust/5扩展测试、两独立进程持久化与隔离安装通过；8份既有用户资料哈希不变。新Windows历史界面、实际PNG视觉比对、DPI与长期资源仍未验证；旧QA-001/002/008保留。无新依赖或独立全库备份功能。

# v1.0.0 GitHub 发布回执

PASS：[ResearchClip v1.0.0 正式版](https://github.com/wpy-sketch/ResearchClip/releases/tag/v1.0.0)，Release ID 416426112。实际发布时间2026-10-18T12:03:55Z（香港时间20:03:55）；draft=false、prerelease=false、Latest=true。

固定源码及带注释标签：`982c3fa34ed43756264e6ab19e3a7c5ef495aa1c`。源码发布后仅补充文档回执和用户要求的Pro规范，不移动此标签。

| 附件 | 字节 | SHA256 |
| --- | --- | --- |
| ResearchClip-v1.0.0-windows-x64.exe | 19842560 | `d32064b0ae05d3d4b3fc575dec41beb8dd56d877198737014ceb4c8e0a901716` |
| ResearchClip_1.0.0_x64-setup.exe | 6222645 | `88bb12d721874b00271919fa51c1767e96e4704a102f5785a8a973bf630d0502` |
| SOURCE.txt | 1304 | `48ec5f25bdb6e13cea3b3ebd38bcdabeecdeb3ff4ab8957bddab977b37ccd2c2` |
| SHA256SUMS.txt | 281 | `8bb54ee0b724895c50469fa626f876f67b3fd949b8998b45a7813889a1bb54fb` |

[独立EXE](https://github.com/wpy-sketch/ResearchClip/releases/download/v1.0.0/ResearchClip-v1.0.0-windows-x64.exe) · [NSIS安装包](https://github.com/wpy-sketch/ResearchClip/releases/download/v1.0.0/ResearchClip_1.0.0_x64-setup.exe)。未签名，需要WebView2。本地分别位于`src-tauri/target/release/researchclip.exe`和`src-tauri/target/release/bundle/nsis/ResearchClip_1.0.0_x64-setup.exe`。

## 实际过程与校验

1. 推送前发现远程新增`5a481a9 Update README.md`，保留其“使用Codex进行构建”署名。`git fetch origin main`成功；`git merge --no-edit origin/main`出现README冲突（退出1），手动保留署名及新版介绍后文档检查通过，合并提交982c3fa。仅校正本轮尚未上传的本地新标签；没有改写已发布标签或强制推送。
2. `git -c http.proxy=http://127.0.0.1:7897 push --atomic origin HEAD:refs/heads/main refs/tags/v1.0.0:refs/tags/v1.0.0`退出0；远程main合并保留双方历史，新建v1.0.0。
3. `python artifacts/publish-v100/publish_release.py`退出0；既有Git Credential Manager凭据仅用于进程内存和curl标准输入。先核对main/标签源码与10个旧Release，创建草稿，上传4附件，核对服务器size/digest，再公开普通Latest。
4. 四个公开附件均匿名HTTP200下载，字节数和SHA256与本地完全相同。10个旧Release的ID、prerelease及附件ID/名称/大小/digest均保持不变。
5. Git只提交36份源码/指南/图片/锁文件/测试证据及合并README；缓存、工具链、真实科研资料、剪贴板及临时测试工作区未上传。[公开回执JSON](evidence/v100-publication.json)不含凭据。

[实际验收](RELEASE-1.0.0.md)：300项前端、125项Rust、88组原生专项、8组浏览器、主屏3帧、最终安装卸载4组通过；素材库退出窗口句柄告警QA-001/P2仍FAIL，完整硬件/生产人工矩阵未完成。正式Release不更改这些检查状态。

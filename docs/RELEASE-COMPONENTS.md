# v4.1.9 下载、配套组件与历史附件

桌面版本4.1.9，SQLite升级schema15，沿用迁移前校验/一致快照/事务；旧版不能写新库。使用顺序：安装当前桌面程序，按需要安装原版本伴侣组件；如需集中保存全部历史发布文件，再下载历史归档分卷。指南包含27章、177项练习和44幅本地图示。

| 任务 | 本Release附件 | 使用方法 |
| --- | --- | --- |
| 安装桌面应用 | ResearchClip_4.1.9_x64-setup.exe | 保存并退出旧实例再安装，需要Windows WebView2；未签名 |
| 直接运行 | ResearchClip-v4.1.9-windows-x64.zip | 解压后运行researchclip.exe，保留licenses；仍使用本机资料目录 |
| Chrome/Edge网页剪藏 | ResearchClip-WebClipper-v1.0.2-windows-x64.zip | 扩展、Native Host、注册脚本及许可原样沿用；按包内README主动安装并在桌面启用连接 |
| 可选公式识别 | ResearchClip-MathOCR-Setup-v1.0.4.zip | 原安装脚本/锁定依赖/worker/模型清单；Python与模型等外部依赖按原README安装，不冒充已安装 |
| 离线指南 | ResearchClip-Guide-v4.1.9.zip | 打开docs/USER_GUIDE.md，保留images/examples；包含伴侣组件说明、模型清单和许可，源码链接指向精确Git标签 |
| Qwen资源索引 | MODEL-RESOURCES.txt、OFFLINE-RESOURCE-INDEX.json、QWEN-MODEL-MANIFEST.json | 与此前附件字节一致；已有运行时可复用，或从本版历史归档v3.1.2目录获得原分卷 |
| 离线图文语义检索 | 指南ZIP内semantic-search/README.md、models.json与许可 | 按固定模型清单主动下载/离线导入，与公式OCR复用现有Python运行时；不上传图片 |
| 全部历史附件 | ResearchClip-History-through-v4.1.1.zip.001起的223卷 | 先按下面步骤重组；包含此前31个Release的全部226附件，按版本分目录 |
| v4.1.2/v4.1.3/v4.1.4/v4.1.5/v4.1.6/v4.1.7/v4.1.8 的全部附件 | 原便携包、安装包、指南与运输文件；SOURCE/SHA在对应版本的Release-Metadata.zip中 | 原名原字节保留；逐项定位见V419-HISTORY-INVENTORY.json和RELEASE-HISTORY-v4.1.9.txt |
| 历史归档校验/重组 | HISTORY-CONTENTS.json、HISTORY-TRANSPORT.json、Assemble-Release-History.ps1、HISTORY-README.txt | 原附件URL/ID/大小/SHA均保留；只重组发布ZIP，不执行旧程序，不涉及应用资料恢复 |
| 本版文件校验 | SHA256SUMS.txt、SOURCE.txt | 逐项校验本版发布文件及源码提交；校验文件自身不包含其自身摘要 |

## 只安装当前程序和所需组件

不需要下载3.73GB历史归档才能使用基础截图、标注和Markdown。伴侣安装ZIP已直接附上；可选模型/系统运行时仍按各组件原说明主动配置，不宣称本版安装器捆绑Windows、WebView2、Python或所有可下载模型。新命令层没有增加外部API、上传或账号流程。

WebClipper1.0.2、MathOCR Setup1.0.4及Qwen清单原样沿用，不改其版本/协议/许可。浏览器说明见[WebClipper](../browser-extension/README.md)，图文模型见[语义检索](SEMANTIC-SEARCH.md)。

## 重组全部历史附件

1. 在一个新的下载目录中放入全部223个ResearchClip-History-through-v4.1.1.zip.NNN分卷、HISTORY-TRANSPORT.json与Assemble-Release-History.ps1。先依据SHA256SUMS.txt核对下载；不能跳过缺卷或损坏分卷。
2. 在该目录执行PowerShell -NoProfile -File .\Assemble-Release-History.ps1。脚本逐卷核验，再CreateNew写入唯一临时文件；整包SHA匹配后提交为ZIP。已有不同内容目标会拒绝覆盖，已有相同ZIP只校验，不执行任何归档内容。
3. 自行把ZIP解压到新的空目录。每个旧版本各有目录，SOURCE.txt、SHA256SUMS.txt等重名附件保留原名和原字节；HISTORY-CONTENTS.json逐项对应原发布记录。
4. v3.1.2目录保留原25卷与必要运输片、原重组脚本和清单；按其中README先还原运输卷，再组装旧离线包。旧包的桌面EXE仍为v3.1.2；日常应用使用本Release的4.1.9。

历史附件原始合计4117989169字节；当前ZIP为3728704762字节，分223个不超过16MiB的运输卷。每个归档成员已按GitHub SHA-256校验，不以清单链接代替附件内容。原31个Release、原附件和标签保持原样。

## v4.1.9 如何完整继承附件

本版实际273个附件，覆盖此前37版1738个原发布附件：通过v4.1.1的226个文件仍在相同223卷归档内；v4.1.2的237个、v4.1.3的243个、v4.1.4的249个、v4.1.5的255个、v4.1.6的261个和v4.1.7的267个附件全部收录。v4.1.7中265个无冲突附件直接附上，SOURCE.txt与SHA256SUMS.txt按原名原字节放入ResearchClip-v4.1.7-Release-Metadata.zip的v4.1.7目录；更早元数据ZIP继续原样提供。V419-HISTORY-INVENTORY.json逐项给出原版本、文件名、大小、摘要及当前定位。不以旧链接代替内容，也不把数GB归档再次嵌套打包。

旧HISTORY-TRANSPORT.json和Assemble脚本保留desktopVersion4.1.2，因为它们描述的始终是通过v4.1.1的同一归档，摘要和223卷完全相同。旧HISTORY-README也原样保留；本版补充说明以RELEASE-HISTORY-v4.1.9.txt为准。当前顶层SOURCE.txt和SHA256SUMS.txt属于v4.1.9。不要把旧版安装包当作当前应用安装。

## 升级后核对

检查版本4.1.9、旧截图/Markdown、工作区、插图与诊断。保存草稿后退出；布局不等于正文保存。真实资料、多显示器DPI及可选第三方运行时的人工验证仍须按[引用与文献验收](qa/CITATIONS_STEP09.md)执行。没有Markdown导出PDF或独立资料备份/恢复产品。

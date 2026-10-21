# v3.1.7 下载与配套组件

本版沿用既有业务模块，新增A11图片局部锚点，SQLite schema9→10使用事务迁移与升级前快照，无新增依赖。截图、标注与历史、素材管理、项目、Markdown、OCR、文献、同步、相似检索及双向链接仍在同一应用内。操作和94项练习见[使用指南](USER_GUIDE.md)；配套文件版本独立于桌面版本。

| 需要完成的任务 | 本Release附件 | 安装/使用方法 |
| --- | --- | --- |
| 安装桌面应用 | `ResearchClip_3.1.7_x64-setup.exe` | 保存并退出旧实例再安装；需要WebView2 |
| 直接运行桌面应用 | `ResearchClip-v3.1.7-windows-x64.exe` | 不等于资料便携模式，仍使用本机应用资料目录 |
| Chrome/Edge网页剪藏 | `ResearchClip-WebClipper-v1.0.2-windows-x64.zip` | 包含扩展dist、Native Messaging Host、注册脚本和许可证；依照包内README安装，桌面设置每次会话明确启用连接 |
| 可选数学公式识别 | `ResearchClip-MathOCR-Setup-v1.0.4.zip` | 包含专用环境安装脚本、锁定依赖、worker及模型清单；不包含Python解释器或171MiB权重，依照包内README主动安装/下载 |
| 离线阅读和练习 | `ResearchClip-Guide-v3.1.7.zip` | 解压后打开docs/USER_GUIDE.md，保留同级images和examples目录；有30幅本地图示与虚构练习稿/BibTeX |
| 可选Qwen模型和CPU运行时 | `MODEL-RESOURCES.txt`、`OFFLINE-RESOURCE-INDEX.json`、`QWEN-MODEL-MANIFEST.json` | 已装用户继续复用；首次使用从索引中列出的v3.1.2附件获取，轻量安装器不含数GB权重 |
| 校验与来源 | `SHA256SUMS.txt`、`SOURCE.txt` | 核对下载文件摘要，SOURCE给出本版源码提交及配套清单 |

浏览器插件、MathOCR脚本和Qwen模型清单与v3.1.3附件逐字节相同，保留原版本、协议ID、许可证和SHA256，不冒充新开发的组件。插件安装说明见[WebClipper README](../browser-extension/README.md)。

## Qwen离线资源的完整取得流程

1. 打开 [v3.1.2完整离线资源](https://github.com/wpy-sketch/ResearchClip/releases/tag/v3.1.2)。本版`OFFLINE-RESOURCE-INDEX.json`逐一记录所需既有附件的下载URL、字节数与GitHub摘要。
2. `.001`至`.020`及`.025`是原始卷。`.021`至`.024`各拆成8个`.partNN`运输片；先获取全部32个片段、`Restore-Transport-Volumes.ps1`、`transport-manifest.json`、`TRANSPORT-SHA256SUMS.txt`和`TRANSPORT-README.txt`，按该README还原四卷。不要把part文件直接交给ZIP解压器。
3. 获取`Assemble-Offline.ps1`、`offline-manifest.json`、`README-OFFLINE.txt`及其校验文件，按原README校验全部25卷并合并解压。旧卷和运输片摘要保持原值；缺卷/摘要不符时停止，不跳过校验。
4. 只取解压后的`similarity-ai`目录，与本版应用按第4.4节配置。旧包中的桌面EXE仍为v3.1.2；使用本Release的3.1.7应用。模型推理在本机执行，输出仍需人工判断。

本次不重传相同的大模型，全部历史Release及其附件保留。发布前后对历史Release ID、标签、源提交、附件ID/名称/大小/摘要逐项比较；这项元数据检查不冒充对所有历史数GB附件重新下载。新Release每个附件单独匿名下载并核对本地摘要，结果写入本版验收记录。

## 升级后核对

确认版本为3.1.7、原素材与文档可读；练习打开截图编辑器、文献和知识图谱。浏览器扩展仍用原ID，不要覆盖另一个目录的Host注册。用户已有模型、模板、设置和资料不会被新Release附件替换。

完整人工硬件矩阵和第三方网站兼容性不是发布文件校验的一部分，实际覆盖范围见[本版验收](qa/RELEASE-3.1.7.md)。

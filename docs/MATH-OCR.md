# A04 数学公式图片转LaTeX：方案与边界

核对日期2026-10-19。本轮仅新增本地公式识别，复用已有OCR选区坐标、PNG裁剪、KaTeX、Markdown光标和笔记服务；不新增数据库表，不实现PDF、云端OCR或其他高级功能。

## 方案比较

| 方案 | 实现/许可 | 体积及部署 | CPU/GPU与本轮验证 |
| --- | --- | --- | --- |
| [pix2tex / LaTeX-OCR](https://github.com/lukas-blecher/LaTeX-OCR) | Python/PyTorch，代码MIT；模型/训练数据权利须另核对 | 依赖PyTorch与checkpoint；默认构造会自动下载，不能直接套用本产品的显式下载策略。精确权重体积见官方发布元数据，未下载 | 上游有CPU/GPU配置；本轮没有运行PyTorch版，不编造Windows速度/RAM或兼容性 |
| [UniMERNet](https://github.com/opendatalab/UniMERNet) | Python/PyTorch，代码Apache2；[tiny模型卡](https://huggingface.co/wanderkid/unimernet_tiny)标Apache2 | 官方README列base约1.3GB/small773MB/tiny441MB，另需tokenizer和Python依赖；依赖与集成链较重 | 面向真实公式的训练方案，但本轮未下载/测试CPU、GPU、Windows，不把论文指标当本机成绩 |
| [RapidLaTeXOCR ONNX](https://github.com/RapidAI/RapidLaTeXOCR) | pix2tex导出三模型；GitHub及0.0.9 wheel所附LICENSE为MIT；PyPI元数据标Apache2存在不一致 | 官方v0.0.0三ONNX+tokenizer共178,952,787字节；本轮经用户明确允许后下载并固定SHA256。导出权重较旧，不声称上游仍高频维护 | 已验证Windows x64/Python3.12/ONNX CPU。单公式启动+推理约1.2–1.5秒；一个自有STIX样例启动器与子进程合计RSS采样约256MiB，不代表整个应用RAM或长期峰值 |

选择第三条作为可验证的首版路线：不安装旧Rapid Python包（其NumPy<2/OpenCV依赖与现代环境冲突），只参考其MIT预处理与encoder/decoder约定，改用当前固定版本ONNX Runtime1.31.0、NumPy2.5.4、Pillow13.0.0、tokenizers0.23.3。Python3.12.14的干净环境真实安装并导入通过，完整传递依赖见[锁文件](../math-ocr/requirements-lock.txt)。这不是宣称新训练模型准确率优于其他方案；实际样本存在严重符号错误，必须人工校对。GPU未集成，也不把CPU实测外推为GPU成绩。

代码许可说明、元数据不一致与独立权重授权/签名缺失见[运行时说明](../math-ocr/README.md)。本版发布源码/安装工具，不把模型权重或Python库重新打包到应用或GitHub附件；需要商业再分发时不能仅凭源代码MIT推断所有权利。

## 受控进程与文件边界

1. Rust MathOcrService在应用配置目录维护独立runtime.json、models-v1与单任务状态；损坏/未来配置显示错误，不阻止其他功能启动，不重置配置或SQLite。
2. 主窗口六个受限命令：状态、配置运行时、显式下载、离线导入、启动识别、取消。WebView不提供任意路径、URL、Shell、模型代码或SQL；Python路径只能由系统文件选择器取得。
3. Rust先验证PNG、原图像素矩形、419万像素/4096单边上限；原图不改，受控临时PNG传给固定内置Python代码。stdout最多64KiB，源码8192字符/token512；stderr不输出内容或完整路径。
4. 固定CPUExecutionProvider、两条intra-op/一条inter-op线程；推理路径拒绝socket连接和子进程启动，不下载模型。Python -I忽略用户site/PYTHONPATH，模型摘要每次核验；ONNX没有启用自定义算子或从模型导入Python代码。用户配置的运行时与同账号可修改的库属于本地信任边界，不把Job Object称安全沙箱。
5. 子进程60秒超时，下载300秒；取消结束子进程并等待回收。Windows Job Object启用KILL_ON_JOB_CLOSE及每进程2GiB提交内存限制。模型下载的.part只存在于本任务临时目录；退出前取消并等待清理，超时则明确取消退出。突然崩溃仍可能留下临时文件，不以扫描删除未知用户文件掩盖。
6. 模型下载只GET清单固定HTTPS地址，重定向限定GitHub/其资源域，尺寸/SHA256/禁止覆盖。离线导入只读四个已知文件，复制有界、校验后原子提交；已存在的相同文件复用，不同文件拒绝覆盖。模型缺失/不一致/无内容/损坏/超时均明确提示，可重试。

## 科研内容安全

KaTeX沿用trust=false、strict/error、长度及展开预算；模型文字不作为命令执行。结果始终带verified=false。源码可编辑、复制、实时预览；插入Markdown或追加截图笔记前必须人工勾选校对，更改源码会取消确认。笔记用既有CAS/草稿保护追加，已有公式和原图不会自动替换。标准.md保存源LaTeX，不转换为图片。

**已发现的误识别必须保留**：STIX测试中，上下标x变成χ且k+1分组改变，求和等号变成不等号，范数等号变箭头且v变成ν；无衬线测试错误更多。KaTeX全部能渲染仍不代表公式正确。首版仅提供本地识别与人工校对工作流，不提供数学正确性保证。

## 真实证据与安装

[本轮验收](qa/RELEASE-1.0.4.md)包含首次失败、143 Rust/320前端测试、11+11公式原生流程、真实下载与取消、超时/输出边界、配置/笔记/.md重开、句柄97→97及尚未运行的Windows界面步骤。硬件为Intel Core Ultra7 356H、约31.55GiB物理内存；RSS每25ms采样，CPU速度依赖环境。

[安装与离线部署](../math-ocr/README.md) · [图文使用指南](USER_GUIDE.md)。本阶段没有新增npm依赖或数据库迁移；新增可选Python依赖锁及Windows API编译特性，不增加WebView通用访问权限。

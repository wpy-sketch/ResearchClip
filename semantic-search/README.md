# 离线语义模型与运行时

此组件为可选实验功能。桌面应用默认不下载、不索引，不发送图片到网络。首次使用请阅读 [模型评估与安全边界](../docs/SEMANTIC-SEARCH.md)。

1. 使用公式OCR已有的专用 Python 3.12+ 运行时，或依据 [公式OCR安装说明](../math-ocr/README.md) 创建专用运行时；本模块仅复用已有 `numpy`、`pillow`、`onnxruntime`、`tokenizers`，版本见 [锁文件](../math-ocr/requirements-lock.txt)。不需要 PyTorch、Transformers 或 sentence-transformers Python 包。
2. 在素材库切换“图文语义 · 实验”，展开“模型与索引管理”，选择该 `python.exe`。此选择与公式OCR共用。
3. 明确确认后由应用下载约228 MB固定模型。离线部署时，按 [models.json](models.json) 的固定URL下载八份文件，以清单中的 `name` 命名，核对大小及SHA256；再用应用的文件夹选择器导入。应用拒绝不同内容的已有文件，导入不执行代码。
4. 点击“开启 / 更新语义索引”；输入中英文画面描述。后续新图需主动更新索引；无自动后台扫描或网络请求。

首版仅CPU、2个ONNX推理线程、单任务，最多2000张，结果前60项。查询和索引都可取消。删除索引仅删除派生向量，不删原图、笔记、OCR、文档或模型。重建逐项替换，中途取消保留已完成数据。

模型组合固定为OpenAI CLIP ViT-B/32的Xenova量化ONNX图像编码器（MIT来源模型）与Sentence Transformers官方对齐的多语言文本编码器及投影（Apache-2.0）；这是同一对齐嵌入空间，不能替换成其他CLIP、中文CLIP或Qwen向量。量化转换与版本、预处理和投影共同定义 `clip-b32-multilingual-int8-v1`。来源、许可证与实测限制见上方评估文档。

本目录的worker是应用内置固定代码；WebView不能指定脚本、模型地址、执行命令或任意输入文件路径。CLI单元测试使用隔离临时目录，不应对用户资料库运行实验脚本。

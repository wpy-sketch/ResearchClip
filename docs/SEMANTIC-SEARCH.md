# A13 离线图文语义搜索

## 功能与模块

素材库“普通搜索 / 图文语义 · 实验”切换；自然语言查询独立于标题、笔记、标签、OCR全文。`features/semantic` → main窗口限定的5个IPC命令 → `EmbeddingService` 单个有界后台任务 → 固定Python/ONNX worker → schema11派生索引。复用现有 `LibraryService`、截图读取、缩略图、知识卡片、主题、原生目录选择器和公式OCR运行时；没有新增npm/Cargo/Python依赖，没有云端向量库或上传。

用户主动开启/更新索引；启动或重启不会自动下载或扫描。下载另需明确确认，离线导入不联网。索引/查询/导入/下载互斥、可取消、失败显示原因并保留已提交记录；重试更新跳过相同原图。普通搜索不依赖模型。新截图需手动更新索引。

## 模型核查与选择

| 项目 | 已核查结论 |
| --- | --- |
| 图像编码器 | `Xenova/clip-vit-base-patch32`，固定提交 `d15189d7028b43f1d3e65039190477f6af591c2a`，`vision_model_quantized.onnx`，89,117,001字节；转换自OpenAI CLIP ViT-B/32 |
| 文本编码器 | `sentence-transformers/clip-ViT-B-32-multilingual-v1`，固定提交 `58edf8cada9e398793dca955574a48cbb7f18be2`，官方 `model_quint8_avx2.onnx`，135,377,779字节；mean pooling后通过同提交 `2_Dense` 的768→512投影 |
| 配对依据 | 官方文本模型卡明确要求原版CLIP ViT-B/32图像编码器，蒸馏对齐其空间；支持包括zh-cn/zh-tw的50多种语言。不是将两个独立CLIP的向量混用 |
| 许可证 | [OpenAI代码及权重MIT](https://github.com/openai/CLIP#license)；[MIT正文](https://github.com/openai/CLIP/blob/main/LICENSE)。[Xenova转换来源](https://huggingface.co/Xenova/clip-vit-base-patch32)未单独标新的许可证，按上游MIT保留来源；[官方文本模型卡](https://huggingface.co/sentence-transformers/clip-ViT-B-32-multilingual-v1)标Apache-2.0。应用不捆绑/改写权重，下载固定公开文件，离线分发须附原许可 |
| 推理与API | [ONNX Runtime Python](https://onnxruntime.ai/docs/api/python/api_summary.html) `InferenceSession` / `SessionOptions`，已在安装版本1.31.0验证实际输入输出；tokenizers0.23.3本地JSON加载；无trust_remote_code、pickle、Torch或远程模型Hub调用 |
| 预处理 | RGB、短边224、bicubic中心裁剪、CLIP固定均值方差；文本上限128token、attention-mask mean pooling、无偏置线性投影、L2归一化512维 |
| 文件/内存 | 八份文件合计228,030,489字节；首次Windows CPU探针峰值工作集301,867,008字节。运行时和解码另占空间，不是模型文件大小即内存上限 |
| CPU | Windows x64，量化AVX2文本图；当前CPU实测可用，老旧不支持相应指令的CPU可能失败；不将其他机器性能作保证 |
| GPU | ONNX Runtime有CUDA/DirectML等可选Execution Provider，但本版固定CPU，不安装GPU运行时、不开放GPU开关；GPU兼容性与性能NOT_RUN |
| 替代方案 | OpenAI原版英文CLIP中文能力不符合目标；Chinese-CLIP需另行检验英文能力；Qwen视觉对比助手不是共享图文嵌入模型；本版不引入这些模型的额外依赖 |

所有文件URL、长度、SHA256见 [固定清单](../semantic-search/models.json)。预处理、分词、投影、量化或文件变化必须改变模型ID或版本并重建；不能复用旧向量。

## 数据与持久化

schema11事务迁移只增加 `semantic_embeddings`：`screenshot_id`（稳定ID、外键删除级联）、`embedding_model_id`、`embedding_version`、`image_hash`（原图SHA256）、`embedding_data`（512个小端f32，2048字节）、`source_stamp`、`updated_at`。既有001–010迁移不修改；复用升级前校验/备份、未来版本拒绝写入、事务回滚。

索引读取受管原图，在推理前计算摘要，提交前再次核对摘要与文件stamp，冲突拒绝写入。查询按模型ID和版本隔离，只比较当前原图stamp匹配的记录；失效计数明确展示，用户主动重建修复。重命名不改变ID；标注不改变原图向量。原始截图不可变，本版语义搜索不索引合成标注或文档图片。重启从SQLite恢复向量，任务进度仅本会话保留，重新更新安全跳过已完成记录。

小数据集线性余弦扫描，最多2000向量；分数降序、稳定ID同分排序、前60项。可按当前项目/未分类和收藏范围限制结果。分数不是概率或准确率。

## 隐私、取消与边界

除用户确认的模型下载分支外，worker无网络请求。输入图片仅来自受管截图读取后生成的临时文件；前端不能提供路径、SQL、脚本、模型URL或系统命令。导入必须原生文件夹选择，所有模型按清单全量大小/SHA256验证，不加载未校验权重，不覆盖异内容已有文件。

复用既有Python隔离参数`-I`、清理Python环境变量、隐藏窗口、输出64KiB限制、Windows Job Object关闭杀子进程及2GiB进程内存上限。单个任务，2个ONNX intra线程、1个inter线程、禁用spinning，单次推理90秒、整次下载1800秒上限。取消终止工作进程；正在提交的单个SQLite写入先完成，后续不继续。退出发出取消，Job Object随进程句柄释放。索引失败不删除旧记录，不触碰原图。

临时输入随任务清理；系统异常终止可能遗留受管临时文件，本版不扫描或删除未知文件。普通搜索、原始资料不依赖索引、模型或Python。极端长宽比导致缩放像素预算超限会失败，避免巨量分配；需要用户另行导入合理裁剪图。

## 实测限制

测试机器：Intel Core Ultra 7 356H，16核/16逻辑处理器，物理内存33,879,642,112字节（约31.55GiB）；Windows11 build26200。机器有RTX5060 Laptop及Intel集显，但本轮仅CPUExecutionProvider，未使用或测试GPU推理。

固定六类小样本12条查询首轮Top1为8/12，中英文各4/6。中文追逃目标排名2、软件截图排名3；英文flowchart排名3、软件截图排名2。首轮热编码：每图24.09–36.14ms、每文本3.62–6.53ms；图模型加载634.62ms、文本297.25ms、文件摘要验证164.49ms。应用为资源释放采用每项独立受控进程，实际端到端耗时还包括进程启动、全量摘要、落盘和读取，不能把热推理时间当作产品响应时间。

样本包括三个自主绘制的受控图示、已有STIX公式图、实际Windows界面截图、scikit-learn的CC-BY `china.jpg`示例。它是便利样本，不是标准基准；自然风景样本含建筑。完整排名、环境和应用端到端数据见 [本版验收](qa/RELEASE-3.1.9.md)。不保证理解所有公式、证明或数学图像；建议结合普通搜索、标签与人工核对。


风景测试图片来源：scikit-learn `china.jpg`，按其[官方归属说明](https://github.com/scikit-learn/scikit-learn/blob/main/sklearn/datasets/images/README.txt)署名danielbuechele、CC-BY-2.0；仅在隔离本地测试中使用，未将照片附入应用或发布包。

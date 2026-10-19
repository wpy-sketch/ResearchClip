# 本地数学公式识别运行时

主程序不自动安装Python、不自动下载模型，不上传图片。第一版只验证 **Windows x64 + Python 3.12 + CPUExecutionProvider**。需要先安装可选本地运行时；未安装不会影响截图、标注、普通OCR或Markdown。

## 1. 安装推理依赖

准备官方Windows x64 Python 3.12与本文件夹（可从仓库或Release的MathOCR-Setup ZIP取得）。使用新的目录，脚本不删除或覆盖已有环境：

```powershell
pwsh -NoProfile -File .\install-runtime.ps1 `
  -Python 'C:\Path\Python312\python.exe' `
  -Destination 'C:\Path\ResearchClip-MathRuntime'
```

脚本从官方PyPI安装`requirements-lock.txt`固定版本，包含ONNX Runtime、NumPy、Pillow、tokenizers及其运行依赖；不安装CUDA、PyTorch、OpenCV或RapidLaTeXOCR旧Python包。首次需联网下载运行依赖，体积由对应Windows wheels决定，模型另计。失败会保留目录与错误，不能以清空已有环境来绕过问题。

在ResearchClip「设置 → 本地公式识别 → 配置Python运行时」选择新环境的`Scripts/python.exe`。应用只运行编译内置的固定Python代码与固定参数，不提供通用Shell或脚本执行入口。

## 2. 主动下载模型

在设置点击「下载模型 · 171 MiB」，阅读来源/体积，再确认。官方来源为[RapidAI/RapidLaTeXOCR v0.0.0](https://github.com/RapidAI/RapidLaTeXOCR/releases/tag/v0.0.0)：

| 文件 | 字节 |
| --- | ---: |
| encoder.onnx | 89,008,136 |
| decoder.onnx | 50,952,726 |
| image_resizer.onnx | 38,967,751 |
| tokenizer.json | 24,174 |
| 合计 | 178,952,787 |

`models.json`记录固定HTTPS来源、尺寸与SHA256。摘要在本轮授权下载后固定，不是上游签名；不能把TLS下载与本地摘要当成发布者签名。每次推理前核验，损坏或缺失时失败，不自动替换未知文件。下载可取消，未完成临时文件释放；已校验完成文件保留供重试。

模型保存在应用配置目录`math-ocr-v1/models-v1`；运行时路径保存在同目录`runtime.json`。配置损坏/未知版本会显示错误并保留，不清空截图数据库。

## 3. 完全离线部署

在联网机器下载同版本依赖wheels并安装到独立环境，再复制完整环境需确保Python安装路径兼容；更稳妥是在目标机器用相同版本Python和离线wheel目录建立环境。不要只复制一个python.exe。可先显式下载模型到一个新文件夹：

```powershell
python .\download-models.py --output 'D:\MathModels' --accept-source-size
```

该辅助脚本只下载四份模型，不读取图像；错误JSON表示下载失败，不能仅凭进程启动判定完成。把四份文件复制到目标机器，在设置选择「导入离线模型」。只读来源文件，验证尺寸与摘要后复制到应用目录，不覆盖已有不同文件。模型和已安装依赖齐备后，识别过程可断网运行。

## 4. 使用与限制

1. 截图预览、知识卡片或图片编辑器点击「公式识别」。编辑器以不可变原图识别，避免标注遮挡。
2. 整图或框选单个清晰公式；选区使用原图像素，最多419万像素/单边4096。
3. 开始后可取消，60秒超时。单任务、两条ONNX计算线程、512 token上限；Windows Job Object限制子进程2 GiB提交内存并在父进程退出时结束子进程。这是资源边界，不是进程安全沙箱。
4. 编辑LaTeX并查看现有KaTeX安全预览。模型输出永不作为代码执行，网络连接/外部进程启动在推理路径被拒绝；不启用KaTeX trust。
5. 复制可直接操作；插入Markdown或追加截图笔记前必须勾选校对确认。已有笔记不被自动替换。更改源码后需要再次确认。

**准确性限制**：旧模型会漏项、混淆变量、错认等号与箭头，甚至增加不存在的项；能渲染不代表正确。首轮无衬线测试已出现明显错误，原样保留在验收报告。矩阵/多行/范数/低清截图必须逐项核对，不作为数学正确性验证。

## 5. 依赖与许可

- 预处理/编码器解码器约定参考[RapidLaTeXOCR](https://github.com/RapidAI/RapidLaTeXOCR)，源代码与实际0.0.9 wheel所附LICENSE均为MIT，见`LICENSE-RapidLaTeXOCR`；PyPI元数据写Apache-2.0，与实际附带许可证不一致，已记录，不声称没有许可歧义。
- 模型为该项目发布的pix2tex ONNX转换；[上游LaTeX-OCR](https://github.com/lukas-blecher/LaTeX-OCR)代码MIT，模型发布页没有独立的权重许可证或签名说明。本版不把权重打进应用/Release；商业分发前需另核对权重及训练数据权利，不能只凭代码许可证推断全部权利。
- ONNX Runtime MIT、NumPy BSD及附带第三方许可、Pillow MIT-CMU、tokenizers Apache-2.0；使用固定版本，保留各安装包自带LICENSE。GPU、其他模型和macOS未集成。

方案比较和真实验收见仓库`docs/MATH-OCR.md`与`docs/qa/RELEASE-1.0.4.md`。这份README中的步骤不等同于已经执行的Windows界面验收。

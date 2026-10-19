# v1.0.4 / A04 数学公式图片转LaTeX验收

日期2026-10-19；Windows11 x64、Intel Core Ultra7 356H、33879642112字节物理RAM，Node22.22.2/npm10.9.7/Rust MSVC1.99/Tauri2.12.3；可选Python3.12.14/ONNX1.31.0 CPU。先完成并发布A02/v1.0.2与A03/v1.0.3，本轮不重建项目、不增加PDF或云服务。

## 实现与涉及文件

- services/math_ocr.rs：MathOcrService、固定受控进程、系统对话框授予的运行时、配置持久化/未来版本拒写、模型导入/摘要/禁止覆盖、PNG像素裁剪、单任务、取消/60秒超时/64KiB输出/512 token/2GiB Windows Job Object。lib.rs/desktop.rs负责注册、退出取消与资源等待。
- commands/math_ocr.rs、build.rs、main capability：六个main专用命令，无WebView通用文件/进程/HTTP权限，无任意SQL或用户URL。
- math-ocr/worker.py、models.json、requirements-lock.txt、install-runtime.ps1、download-models.py、README/许可证：离线CPU推理、主动下载、固定哈希、独立运行时交付工具。模型与环境不提交Git或打入应用。
- features/math-ocr/api/store/MathOcr：独立会话，复用OCR选区坐标、MathFormula安全渲染、Markdown光标与CardStore追加。截图预览、编辑器与知识卡片提供入口；设置提供运行时/模型配置。
- store.test.ts、document/store.test.ts、qa_math.rs仅debug、math_ocr_probe、scripts/math-ocr-fixtures.py：状态/安全/坐标/源码和真实进程测试。架构/计划/指南/版本/锁文件与两幅明确标注的操作图/自有公式图更新；不是伪造窗口截图。

## 实际命令与结果

| 命令或检查 | 状态 | 实际结果 |
| --- | --- | --- |
| npm run check | PASS | 47文件320项，TS/ESLint/Vitest/前端生产构建退出0；其中状态Mock明确仅单元层 |
| npm run format:rust | PASS | fmt --check退出0 |
| npm run test:rust | PASS | 143项，0失败/忽略，含原图坐标裁剪/零面积/越界/未知字段、无运行时/未来配置保留、损坏模型禁止落盘与既有数据库回归 |
| npm run check:rust | PASS | all-targets -D warnings退出0 |
| pwsh -File math-ocr/install-runtime.ps1 -Python <bundled3.12> -Destination artifacts/qa-v104/runtime-clean | PASS | 实际新环境安装全部22个固定依赖，原生模块导入成功，未改全局Python或自动装模型 |
| math_ocr_probe.exe <python> <models> <fixtures-stix> <newroot> | PASS | 真实Rust服务/Windows Job Object/Python/ONNX；11份输出均可编辑且verified=false；60秒机制以250ms受控sleep负例验证，超限stdout拒绝；识别取消、损坏图恢复、模型缺失/摘要不匹配/空白图、配置重开、重复导入、笔记和.md保存重开 |
| 同探针+RESEARCHCLIP_QA_DOWNLOAD_MODELS=1 | PASS | 实际官方179MB下载，下载取消/.part清理、完成文件摘要、取消后重试；相同文件复用、不修改下载来源文件 |
| 11份模型输出离线KaTeX渲染 | PASS | 11/11可渲染，仅语法/渲染检查，不代表公式内容正确 |
| 重复推理句柄 | PASS | 首轮STIX85→85；干净运行时最终97→97（前后等待结束），不是长期无泄漏保证 |
| 一个STIX公式内存/耗时 | PASS | 25ms采样启动器+其自身子进程，峰值267993088字节约255.6MiB；模型结果内计时约1.16秒，整套11图每张含启动约1.2–1.5秒；不外推GPU/整体应用内存 |
| Windows新增界面/系统对话框/剪贴板/Markdown光标实点 | NOT_RUN | 前阶段Computer Use因无法可靠确认URL停止输入；未换工具绕过。状态单测与真实文件测试不能替代UI |
| Windows EXE/NSIS/安装/文档 | PASS | npm run desktop:bundle最终退出0；隔离安装/繁忙保护/默认卸载通过；8份既有资料摘要不变；72文档433链接校验通过。发布另附回执 |

## 公式内容核对：不能把流程PASS当准确率

以下是对自有STIX图像与实际输出的源码对照；不是数学证明或Windows人工界面验收。报告仍保留mathCorrectness=NOT_VERIFIED。

| 输入类型 | 符号/结构对照 | 观察 |
| --- | --- | --- |
| V(x)=x^TPx | PASS | STIX样例字符一致，含额外排版空白 |
| 分式dV/dt<0 | PASS | 分式/不等号正确，带句末点，需按用途校对 |
| 上下标x_i²+y_(k+1) | FAIL | x识别为χ，k+1分组改变 |
| 希腊字母 | PASS | 本样例α/β/λ/γ对应 |
| 积分 | PASS | 本样例积分上下限与1/3对应；无衬线首轮曾幻增项 |
| 求和 | FAIL | 等号识别为不等号，数学陈述改变 |
| 向量范数追逃示例 | FAIL | 等号变长箭头、v变ν，不能直接使用 |
| 复杂括号 | PASS | 本例分式括号/平方对应 |
| 低分辨率V(x) | PASS | 仅这一自有低清图对应，不保证其他低清图 |
| 2×2矩阵 | PASS | 数字/行列对应，bmatrix输出为array+方括号，需检查对齐语义 |
| 两行状态/输出公式 | PASS | 两行结构/字母对应，aligned输出为array |

首轮无衬线图还出现V/P等变量误读、积分幻增项、箭头误读等，原样保留[首轮原生输出](evidence/v104-math-sans-native.json)。[STIX最终结果](evidence/v104-math-native.json)、[下载复验](evidence/v104-math-download-native.json)、[内存](evidence/v104-math-memory.json)、[KaTeX](evidence/v104-math-preview.json)均为自有图像摘要。不要以8/11的这一小组对照推断通用准确率。

本轮内容失败为**P2/识别质量限制**，在UI/指南/Release中明确提示，复制保留待校对源码；插入和保存必须主动勾选校对，不自动替换既有公式。对于要求无需人工校对的科研场景不满足验收，不据此宣称数学识别可靠。

## 首轮失败及处理

1. CreateJobObjectW缺Win32_Security编译特性，核对安装版windows-rs后补齐；只增加Rust编译特性，没有扩大WebView权限，复编译通过。
2. 前端测试误用Node Buffer而项目使用浏览器TS类型，改用atob/Uint8Array，不改TS配置压制错误；完整检查通过。
3. 第一次内存采样只统计venv启动器4.7MB，不是模型RSS；重新统计其自有进程树得到约256MiB，旧值不作为交付内存指标。
4. pix2tex /releases/latest元数据404，列表核实前三版均预发布且无附件；未编造checkpoint大小或下载更多权重。
5. 本轮公式准确性失败如上，没有调换断言或静默纠正输出。模型已按用户明确授权下载，SHA256为本轮固定摘要，不是上游签名。

## Windows人工验收（NOT_RUN）

1. 设置中选择已准备的python.exe；缺依赖/选错文件/取消系统对话框应明确失败并保留旧配置。确认模型来源/体积后下载，断网/中断/取消/重试；离线导入不改来源或覆盖不同文件。
2. 截图预览、素材库卡片、编辑器分别打开公式识别；整图和局部框选、125/150%DPI、零面积/超限/图片加载失败。原图摘要前后不变，标注状态不受影响。
3. 对分式/脚标/希腊字母/积分/求和/矩阵/多行/范数/括号/低清图逐项校对；故意输入错误或危险LaTeX，预览显示错误且不执行HTML/链接。
4. 编辑后确认状态应清除；未勾选不能插入/保存。复制到其他编辑器检查源码，Markdown光标插入→Ctrl+S→关闭重开验证源码，追加笔记不覆盖旧正文，连续点击保存不重复追加相同结果。
5. 识别中取消再重试、60秒超时、图片损坏/无文字/模型损坏；多次开关面板后焦点与事件清理。关闭结果时取消应保留，退出应用等待进程/临时资源释放，托盘恢复不丢失编辑草稿。
6. 真实原生UI、新版/其他Windows、GPU/macOS、手写公式、长时间内存/句柄压力均未通过；macOS/GPU适配NOT_IMPLEMENTED。

## 依赖、数据库与建议

无新增npm依赖和数据库迁移，schema4/001–004原样保留。新增可选Python锁定依赖和Windows JobObjects/Security编译特性；模型/代码许可歧义见[方案说明](../MATH-OCR.md)与[安装说明](../../math-ocr/README.md)。只提交源码、工具、文档及自有小型指南图，用户图片/配置/模型/环境/缓存不公开。

建议交付需校对的本地工作流，不能作为无需人工确认的数学识别器；保持上述内容FAIL和UI NOT_RUN，历史QA-001/002/008不关闭。

收尾界面检查修正：公式面板复用OCR宽面板和窄窗口单栏布局；图片用自动宽高与max-size保持元素坐标对应真实图像，避免object-fit留白影响局部框选。需人工补验长竖图/宽公式与窄窗口，未把静态源码检查标为运行通过。

收尾实际命令：npm run lint、npm run build（布局修正后）、npm run desktop:bundle、pwsh -File scripts/installer-smoke.ps1 -RunId v104-final -TargetDir src-tauri/target、npm run check:docs、npm run format:rust均退出0。最终原生native-delivery再次PASS，句柄97→97；下载工具复用已有有效模型PASS。MathOCR-Setup ZIP共8文件、10948字节，逐份对比源字节（README仅改为远程文档链接）通过，无权重/运行时/缓存。实际安装未启动生产程序访问用户工作区，未签名，不关闭QA-008历史问题。

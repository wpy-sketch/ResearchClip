# v3.1.2 A08 验收记录

本轮新增原图特征索引、相似检索、重复组与可选离线Qwen双图辅助。测试使用隔离工作区；未修改真实用户SQLite、原图或旧发布。[机器证据](evidence/v312-checks.json)记录本轮结果，历史记录不代替本轮验收。

## 最终实际检查

环境：Windows x64、Node22.22.2、项目Rust MSVC工具链、WebView2。无新增npm或Rust依赖；离线分发新增Qwen Apache-2.0模型、llama.cpp MIT运行时、OpenMP Apache-2.0 LLVM例外许可证材料，固定大小与SHA清单入源码，权重不入Git。

| 检查与实际命令 | 状态与结果 |
| --- | --- |
| `npm run check` | PASS，退出0；类型、ESLint、Vitest 55文件372测试、前端生产构建；保留大chunk警告 |
| `npm run format:rust` / `npm run check:rust` | PASS，退出0；fmt与全部target严格Clippy |
| `npm run test:rust`（正常Windows凭据权限） | PASS，178通过、0失败；7项独立服务器测试另跑 |
| `node scripts/with-rust.mjs cargo test --manifest-path src-tauri/Cargo.toml --locked -- --ignored --test-threads=1` | PASS，独立本地HTTP/TLS服务器7项；双客户端、ETag冲突、历史分叉、删除/修改、丢确认、重试、取消、不完整下载、证书拒绝 |
| `npm ci --prefix artifacts/a08-clean-install --ignore-scripts --no-audit --no-fund` / `npm ls --prefix artifacts/a08-clean-install --depth=0` | PASS，退出0，303包，隔离锁文件安装，无产品依赖升级 |
| `npm --prefix browser-extension run check` | PASS，5测试及独立构建；发布原兼容ZIP，扩展ID不变 |
| `npm run tauri -- build --debug --no-bundle` / `npm run desktop:bundle` | PASS，退出0，内嵌资源调试版及最终3.1.2优化EXE/NSIS |
| `node scripts/similarity-native-smoke.mjs a08-ship-rerun --ai` | PASS，7组，真实模型与取消，显式检查临时图片目录为空 |
| `node scripts/guide-native-smoke.mjs a08-layout --verify-only` | PASS，5组；真实SQLite、标注、卡片笔记/标签、双图对比、系统Markdown保存和25图加载 |
| `node scripts/window-capture-native-smoke.mjs a08-final` | PASS，14组；受控WGC/普通Edge、遮挡、负坐标副屏、最小化/权限、取消、保存、剪贴板、入编辑器及20次句柄126→126 |
| `node scripts/material-ui-smoke.mjs a08-final` / `node scripts/guide-reader-smoke.mjs a08-final` | PASS，8/9组；本地Edge布局/对比度/导航；不是原生采集证明 |
| `python scripts/package-similarity-offline.py --version 3.1.2 --out artifacts/offline-v312-ship --bundle artifacts/a08-model/bundle/similarity-ai --exe src-tauri/target/release/researchclip.exe --installer src-tauri/target/release/bundle/nsis/ResearchClip_3.1.2_x64-setup.exe` | PASS，3卷，完整ZIP 3,269,394,775字节；组装脚本后续改用.NET SHA以避开本机PS模块加载问题，分卷未改 |
| `python artifacts/a08-assemble-final.py` | PASS，实际Windows PowerShell5.1合并/解压39文件逐项SHA；再次执行拒绝覆盖，预期退出1 |
| `pwsh -NoProfile -File scripts/installer-smoke.ps1 -RunId v312-rerun -TargetDir src-tauri/target` | PASS，真实安装/忙进程保护/默认卸载；既有11份用户资料哈希保持；无遗留安装 |
| `python artifacts/qa-v312/release-worker-probe.py` | PASS，最终优化EXE不初始化用户DB；626×473原生窗口图，绿/红各137574像素 |
| `npm run check:docs` / `git diff --check` / `python -m py_compile scripts/package-similarity-offline.py` | PASS，文档链接/围栏、权限/已发布迁移不变、结构与脚本语法；非功能运行替代 |

## 已执行的原生验证

`node scripts/similarity-native-smoke.mjs a08-final --ai --guide`，退出0，`artifacts/similarity-a08-final/report.json`：7组PASS、errors空。

1. 38张真实受管PNG/SQLite索引；重复组33张、3张；不同文件名不改变SHA身份。
2. 完全重复与近似阈值、缩放图匹配、25/7条分页；结果显示距离而非概率。
3. 深浅色、1180/760/480宽度无面板溢出；进入既有双图对比。
4. 实际删除确认取消后记录与索引保持不变；相似功能没有删除/合并入口。
5. 损坏仅隔离测试原图；保留原记录、失败状态可重试；取消重建保留已完成条目，未变化条目复用。
6. 实际系统文件夹选择器+模型/运行时SHA校验；CPU Qwen比较两张测试图，中文输出、准确A/B身份；第二次真实推理取消。
7. 正常退出0后重启，schema7/特征保留，索引和模型不自动启动。

实际界面图：`docs/images/v312-similarity.png`。该测试不宣称屏幕采集、高DPI或所有硬件/图像模型准确性通过。

## 失败及修正记录

- 初次原生脚本因局部变量`document`遮蔽DOM对象失败；改名后两次实际重跑通过，没有更改业务断言来跳过。
- 最初推理探针退出0但输出为空，标FAIL；核实运行时参数，使用两个图像列表和保留生成输出的日志参数后实际重跑成功。重复`--image`只使用最后一张，`--log-disable`会一并抑制生成输出，不用于最终实现。
- 最初完整Rust测试中两项旧schema断言仍为6，失败；仅更新最新版本断言为7，保留历史升级备份版本断言，随后重跑通过。
- 编译/Clippy在开发期的类型推导/复杂类型提示已修复；最终检查结果另列。
- Qwen测试虽识别红/蓝与A1/A2，但可能因token预算截断，作为已知限制显示，不把输出认定为数学事实。
- 首次受限Rust全量运行Windows凭据写入失败；提升正常测试权限后178项通过，没有明文回退。
- 直接cargo调试构建依赖未运行的开发服务器，指南连接失败；改用Tauri内嵌资源构建复验。其后发现新增独占入口行挤低素材首排（P2），移入数量工具栏后原断言及全流程通过。
- 最终模型校验复验超过60秒等待，提升测试预算至180秒并保留失败状态文件，真实重新执行通过；推理仍有180秒超时与取消。
- 分卷脚本首次依赖Get-FileHash，当前Python子进程的PS模块加载失败；改为.NET流式SHA并实际PS5.1重跑通过。初次安装测试用PS5.1不支持脚本中的IsWindows，尚未安装；按原脚本所需PS7重跑成功。优化worker先用缺Pillow的隔离venv失败，换用已有Pillow12.3.0的本机Python后实测通过。

最终结论：本轮已执行检查无未解决FAIL，可发布v3.1.2。未执行矩阵和旧问题继续公开保留；不将38张原生样本宣称为10万张性能验收。完整修改涉及services/similarity、similarity_index、similarity_ai，storage/similarity及迁移007、受限IPC、前端similarity模块、素材入口/指南、版本与QA脚本；原始001–006迁移和用户资料保持。

## 发布传输重试与相同内容证明

初始大分卷上传实际FAIL：GitHub连接重置，发布保持draft，没有公开不完整版本。改为25个128MiB小分卷，ZIP本身、最终EXE/NSIS、模型及39个解压文件均未修改。`python artifacts/a08-assemble-small.py`退出0，实际Windows PowerShell5.1合并/解压/逐文件SHA再次PASS；重跑拒绝覆盖的预期退出1。上表3卷是初始本地验证记录，不表示最终Release附件只有3卷。源码标签`2336bd9`保持不可变；打包脚本分卷大小、文档和发布回执后续提交到main，不改变已验业务代码或二进制。实际公开附件的最终状态以发布回执为准。

## 人工验收清单（未执行项为NOT_RUN）

- [ ] 在自己的Windows机器升级后进入「相似与重复」，更新索引，检查原有素材/笔记/标注完整。
- [ ] 对真实重复、压缩版、颜色版、带局部标注的图调整阈值；用双图对比核对误报和漏检。
- [ ] 用自己的大规模素材库更新/取消/重建，检查磁盘、内存和响应性；本次原生数据量38张，不代表10万张性能已实测。
- [ ] 下载全部离线分卷并校验解压；安装版选择`similarity-ai`，确认两张图片后再点运行，取消后可重试。
- [ ] 低内存、不同CPU指令集、物理125%/150%与多屏、RDP/HDR、第三方剪贴板矩阵。
- [ ] macOS运行；当前仅Windows x64 CPU运行时。
- [ ] 真实坚果云认证与两台物理设备同步：BLOCKED（未提供安全配置的凭据）；隔离本地双客户端不替代该验收。

既有QA001/002/008及OCR/公式识别准确性限制见[总表](TEST_REPORT.md)。不得依据本轮算法测试自动删除用户资料。

# v1.0.3 / A03 OCR全文索引验收

2026-10-19，Windows11 x64/Node22.22.2/npm10.9.7/Rust MSVC1.99/Tauri2.12.3，SQLite bundled构建。本轮只实现A03；A02已发布，A04另阶段进行。

## 实现与文件

- storage/ocr.rs、migrations/004_ocr_index.sql、storage/migrations.rs：OCRContent、可回滚备份迁移、CAS纠错、失败恢复、索引重建。001–003保持不变。
- storage/search.rs、projects.rs、library.rs：FTS5 trigram及中文1–2字符LIKE、安全字面查询、来源筛选、140字符摘要，分页与全选同一条件。
- services/ocr_index.rs、commands/ocr_index.rs、lib.rs/desktop.rs、build.rs/main capability：有界单工作线程、主动启动/暂停/继续/取消/重试/退出取消，6项main专用命令。
- services/ocr.rs：修复逐次WinRT退出与缓存工厂生命周期冲突；已启动原生识别完成前保留bitmap，取消丢弃结果。
- features/ocr-index/api.ts/OcrIndex.tsx/highlight.ts/index.test.ts；library、notes、projects相关组件/状态/API/测试：全文查看/纠错、未保存确认、检索来源与摘要高亮、批量选择一致。
- qa_ocr.rs仅debug、ocr_index_probe/ocr_repeat_probe和scripts/ocr-index-fixtures.ps1：30图真实原生验收，不打包到生产。版本/锁、指南、SVG操作路线与架构/计划/验收更新。

## 实际检查

| 命令/检查 | 状态 | 结果 |
| --- | --- | --- |
| npm run check | PASS | 退出0；46文件312项，TS/ESLint/Vitest/前端生产构建 |
| npm run format:rust | PASS | cargo fmt --check退出0 |
| npm run test:rust | PASS | 退出0；140项，0失败/忽略 |
| npm run check:rust | PASS | 修复一条collapsible_if后退出0，all-targets -D warnings |
| ocr_repeat_probe.exe | PASS | 真实Windows连续20次能力查询，无访问冲突 |
| ocr_index_probe.exe fixtures native-third | PASS | 实际Windows.Media.Ocr、真实SQLite/FTS、30张自有1400×300PNG；中文10/英文10/混合10，全部非空且测试关键词30/30命中，约3.07秒（含暂停/取消/恢复及文件操作，不是通用速度承诺） |
| 队列/索引/重开 | PASS | 真实工作线程暂停/继续/取消；只补未完成；重建后记录保持；释放工作区锁后重新打开30条仍在 |
| FTS5/tokenizer/迁移/更新/删除 | PASS | 实际SQLite ENABLE_FTS5=1，创建trigram；中文1/2/3+、英文、混合、符号/非法语法字面、来源、CAS纠错、删除级联、重建、v3→4备份、旧迁移失败/未来版本拒写 |
| 生产EXE/NSIS及文档 | PASS | npm run desktop:bundle退出0；check:docs 67文档401链接；安装结果另补 |
| Windows新增界面点击/剪贴板/Markdown插入/语言硬件矩阵 | NOT_RUN | 原生UI输入受前阶段Computer Use URL安全阻挡；没有用Mock或组件存在代替 |

[30图公开摘要](evidence/v103-native-ocr.json)仅含自有夹具信息，不含用户图像或笔记。测试只用新临时工作区，无真实AppData重置。30个关键词匹配不是逐字准确率或科学内容验证。

## 首轮失败与修复

- 沙箱执行Rust时文件原子提交受限导致63项失败，前端Vitest临时SSR文件ENOENT；按真实Windows权限重跑，不删断言或改用户数据。
- 首次编译命令模块引用storage私有check_id；改为复用get_screenshot边界校验。旧项目迁移断言期待schema3；改为当前schema4，旧迁移SQL不改。严格Clippy嵌套条件修正。
- 两次30图运行进程访问冲突0xC0000005，随后最小探针证实第二次能力查询即崩溃。最初怀疑异步Cancel的判断不足，移除取消后仍复现，未掩盖。定点修复WinRT工厂缓存生命周期后，20次查询及30图实际套件通过。
- 自动审批拒绝整文件git-show覆盖，未执行；改为仅修复相应代码，保留未提交工作。

## 限制与人工验收

1. 在Windows知识卡片点击「OCR全文与纠错」识别/编辑/保存，关闭脏草稿取消应保留，重新识别需确认；打开Markdown再插入/复制并粘贴核对。NOT_RUN。
2. 主动启用批量索引后暂停/继续/取消；取消等待当前原生识别，不承诺硬超时。关闭到托盘应继续，退出后重启手动重试未完成。真实队列CLI已测，窗口交互NOT_RUN。
3. 顶部中文短词、英文、特殊字符，来源/项目/收藏/多标签组合，网格/列表摘要高亮与全选同条件，点击结果打开截图。SQL/状态测试PASS，实际视觉NOT_RUN。
4. 本机仅zh-Hans-CN，英文语言包/其他Windows、长图超单边限制、实际DPI与长期进程内存/句柄曲线NOT_RUN。不保证复杂公式或低清图片准确。
5. sourceImageRevision表示应用内不可变原图ID，不检测外部程序同尺寸篡改；OCR不绑定标注像素。派生FTS占额外磁盘；1–2字符LIKE可能在很大资料库扫描文本，未做百万级基准。
6. 历史QA-001/002/008保留；正式Release不代表未运行矩阵通过。未发现需重置数据的修复方式，无新增依赖（复用rusqlite bundled及Windows OCR）。

建议：已执行核心检查通过后可交付本轮增量；Windows新增UI按清单人工验收，A04仍单独验收。

收尾：npm run desktop:bundle退出0，EXE/NSIS成功；pwsh -File scripts/installer-smoke.ps1 -RunId v103-final -TargetDir src-tauri/target退出0，实际隔离安装/繁忙拒绝/默认卸载与7份既有用户资料摘要保持通过。npm run lint及check:docs（67文档401链接）退出0。历史QA-008未关闭。

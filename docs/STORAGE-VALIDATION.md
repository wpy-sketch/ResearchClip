# 本地数据库与图片管理验收

执行日期：2026-10-14，Windows / PowerShell，Asia/Hong_Kong。范围为Screenshot、Note、Tag、ScreenshotTag、原图/缩略图、标注JSON、版本迁移和失败恢复；不扩展搜索、Markdown、同步或完整笔记编辑界面。实体与一致性契约见 [架构第10节](ARCHITECTURE.md#10-当前sqlite与受管图片2026-10-14)。此前验收文件保留其历史结果。

## 工程与模型

复用现有rusqlite0.37 bundled SQLite（MIT）、png0.18和tempfile，无新增依赖，无前端SQL/通用FS插件。新增002_library.sql，保留001_workspace.sql和app_metadata。截图ID、笔记ID、标签ID为SQLite生成的32位小写随机hex；标注对象仍沿用既有UUID/受限ASCII ID和AnnotationDocument schemaVersion1，不复制另一套编辑状态。

| 实体 | 对外字段 | 持久化/约束 |
| --- | --- | --- |
| Screenshot | id,title,originalImagePath,thumbnailPath,annotationData,createdAt,updatedAt,isFavorite；附widthPx,heightPx,revision | screenshots；图片只保存相对引用，JSON正文入DB，id/revision条件更新 |
| Note | id,screenshotId,content,sourceUrl,createdAt,updatedAt；附revision | notes；FK级联，每图100条、正文1 MiB、来源仅http/https |
| Tag | id,name | tags；trim与ASCII大小写规范化后唯一，1–50字符、最多1000个 |
| ScreenshotTag | screenshotId,tagId | screenshot_tags；联合PK与FK，关联修改采用事务 |

数据根为Tauri app_data_dir（Windows通常 `%APPDATA%/io.researchclip.desktop`）。researchclip.sqlite3及WAL/SHM、originals/<id>.png、thumbnails/<id>.png、backups、workspace.lock皆在应用目录；用户设置的保存目录只影响外部导出，不移动受管文件。无浏览器localStorage核心资料，无种子素材或远程服务。

原图不可变、按字节保存；缩略图最长边512px，不放大，预乘alpha双线性缩放，基于原图。标注JSON单独存储，视口/手柄/undo栈不入DB；重开恢复可编辑对象。库与独立项目另存并存，保存是显式操作。

## 写入、迁移与恢复

- OS工作区锁限制一个应用实例执行同一目录存储操作，退出自动释放。业务I/O位于后台worker，连接受Mutex管理。全部业务SQL参数化，不接受SQL/任意图片路径。
- v1→v2前quick_check与结构校验；VACUUM INTO取得包含WAL提交的快照，经临时文件sync、检查、禁止覆盖提交后才迁移。事务中建表、检查并更新user_version；失败回滚。备份失败、未知未来版本或损坏库停止写入，不重建或删除用户资料。
- 创建先提交pending_screenshots意图，再将PNG与缩略图暂存sync、禁止覆盖落盘，最后事务插入截图并移除意图。SQL失败保留已提交文件；没有图片提交的本次空意图可安全撤销。重启或按钮重试能验证原图、补缺失缩略图并恢复记录；缺图/坏图/未来标注版本保留并告警，不自动删除未知孤立图。
- 确认删除事务中删记录、级联笔记/关联、记录pending_file_deletions，然后清理已无引用的受管文件。失败任务保留、界面可见、重启幂等重试；文件已删但队列确认失败报告已删除与剩余清理数量，不误报记录还在。不会递归删目录、外部导出、标签实体或备份。
- 删除清除前端旧存库绑定，不丢弃已打开草稿或预览；再次显式保存创建新资料。标题/收藏/关联导致revision变化时，旧会话不覆写；保留草稿，先另存项目再关闭重开。
- 受管引用、父目录、普通文件和重解析/符号链接均检查；包括不存在目标的链接。检查不宣称防御拥有工作区写权限的外部恶意进程的全部竞态。WebView只能凭有效资料ID读取二进制图片，不开磁盘asset范围。

## 修改文件

新增storage/files.rs、storage/library.rs、002_library.sql、commands/library.rs；修改storage/mod.rs、migrations.rs、commands/mod.rs、lib.rs、build.rs与main capability。新增shared/api/library.ts和测试、features/library/store.ts和测试、app/library-actions.ts和测试；更新LibraryView、CapturePanel、EditorView、编辑会话类型/绑定、open-editor、App及欢迎文案。新增scripts/storage-native-smoke.mjs，package.json只增检查脚本；更新smoke与check-docs，README/ARCHITECTURE/ROADMAP及本报告。锁文件/依赖声明未因本轮改变，001迁移未改写。

## 实际检查

Node22.22.2/npm10.9.7、RustMSVC1.99、Tauri2.12.3、Windows WebView2/Edge，均使用现有锁文件。以下均实际执行，退出码0：

| 命令 | 结果 |
| --- | --- |
| npm run check | 类型、ESLint、17个测试文件124项、Vite生产构建通过 |
| npm run format:rust | 格式通过 |
| npm run test:rust | 54项通过；含迁移、CRUD、文件/DB失败恢复和工作区锁 |
| npm run check:rust | --all-targets -D warnings 通过 |
| npm run tauri -- build --debug --no-bundle | Windows debug EXE通过，已实际运行原生持久化检查 |
| npm run desktop:build | Windows release EXE通过，无安装包/签名或macOS构建 |
| npm run test:smoke:storage:native -- persistence-oct14-1 | 实际Windows应用8组通过，errors为空，含两次关闭重启 |
| npm run test:smoke:storage:native -- persistence-oct14-final | 最终Windows版本8组通过，含两次退出重启和确认删除 |
| npm run test:smoke -- --run-id storage-ui-oct14-fixed | 6组含32种尺寸/主题组合通过；区域mock渲染4种DPR通过，不代表截图硬件 |
| npm run test:smoke:editor -- storage-editor-oct14 | 15组原有标注/历史/PNG像素检查通过，无破坏原图 |
| npm run check:docs | 18份文档、49个本地链接、命令/capability一致；001迁移未改写 |

原生报告位于artifacts/storage-persistence-oct14-1与storage-persistence-oct14-final，最后一次运行已包含全部本轮修复；debug/release构建均针对最终源码重新通过。明确的debug受控320×200 PNG只在两个测试标志同时设置时进入截图预览，正常启动/release不生成截图或库记录。测试库和文件在该报告settings/workspace，测试关闭重启始终使用同一目录，未重置真实AppData。

实际原生证明：通过界面存库创建真实SQLite文件、记录与渲染缩略图，PNG与源字节SHA-256相同；绘制五类图元保存更新同一记录，原图不变；标题/收藏、带SQL样式正文的笔记、标签/关联实际写入；旧revision和路径样式ID被拒绝；真实退出重启后原图、五类JSON、笔记、标签关联、收藏、revision一致；重开画布修改字号再保存；取消删除保留记录，确认删除只清理选中的记录与两图；第二次重启仍保留已更新资料和删除结果。

Rust测试包括v1/WAL内容一致备份与升级、备份/迁移失败保留旧库、未来版本与坏库拒绝；原图/缩略图/全部图元重新打开；SQL样式文本参数化、FK与级联、标签重名及关联失败事务回滚、stale revision；PNG/尺寸/标注版本校验；实际目录/文件冲突；SQL插入失败后的文件恢复；崩溃缺缩略图修复；未知版本/缺图保留告警；删除文件失败、队列确认失败及重启重试；被引用图片不清理、越界路径不读；缩略图尺寸/alpha；第二实例拒绝、关闭释放锁。

## 失败记录与限制

首轮新增Rust测试50通过/1失败：Windows只读备份句柄无法sync，改为具备写权限的临时备份句柄后升级测试通过。首轮类型检查发现新增library操作的联合类型未收窄，修复分支后通过。Clippy发现needless_question_mark和新版as_chunks用法，修复后严格检查通过。UI冒烟storage-ui-oct14退出1：旧定位器取空状态class，新的真实列表已将视图状态放到语义section；改为定位“截图素材库”region，保留grid/list断言后完整通过。没有删断言或用假结果绕过失败。

恢复场景用临时SQLite故障trigger、临时文件/目录干扰模拟，标记为自动化故障注入；不等于断电、真实磁盘满/ACL/杀毒文件竞争全部硬件验收。实际原生测试以受控PNG验证真实持久化，不验证此刻GDI桌面访问；旧屏幕权限拒绝记录保留在 [双快捷键验收](SHORTCUT-VALIDATION.md)。本轮未运行macOS、硬件DPI矩阵或性能p95；未对用户真实AppData执行恢复/回滚。

当前没有自动保存、回收站、全量备份恢复UI、完整笔记编辑/分类/搜索或Markdown。移动资料请关闭应用并整体保留DB和图片目录；迁移备份仅为SQLite一致快照，不能当作所有图片的独立完整备份。未识别孤立图片保留，需后续显式恢复策略，不自动清理。

运行 npm run desktop:dev；截图后“存入素材库”，编辑后“保存到素材库”，退出重开从卡片“编辑标注”恢复。下一步建议人工复验真实截图存库、只读/磁盘满/外部文件占用与大图库性能；其他阶段等待新请求。本轮完成后停止。

# v3.1.2 发布传输记录

## 当前状态：BLOCKED，未正式发布

- PASS：业务源码与不可变标签`v3.1.2`已原子推送，源码提交`2336bd9fc109eee8ec90249d3f88bcae7e6058f0`。
- PASS：前端372、Rust178及7项独立WebDAV集成、扩展5、原生A08/模型/指南/窗口截图、最终EXE和NSIS安装/默认卸载。详见[验收报告](RELEASE-3.1.2.md)。
- PASS：同一离线ZIP重新分成25卷，每卷至多128MiB；实际Windows PowerShell合并、解压39文件及逐项SHA通过，拒绝覆盖现有输出。见[离线分发证据](evidence/v312-offline-delivery.json)。
- 已收到16个附件的成功/匹配digest回执，包括EXE、NSIS、兼容浏览器插件及7个模型分卷；**这不代表34个目标附件全部完成**，当前远端API无法最终复验。
- FAIL：最初大分卷上传连接被重置；改小分卷及有界并发重试后，后续API请求仍发生TLS EOF/连接重置。Python、Windows原生TLS与显式现有本机代理均失败；没有关闭证书校验。
- BLOCKED：用户确认GitHub网页可访问，浏览器中草稿编辑页最终加载；API页面导航超时。自动文件上传又被ChatGPT浏览器扩展的本地文件权限拒绝，尚未上传额外文件。等待用户手动设置权限；不代改安全/代理设置，不操作认证弹窗。
- NOT_RUN：全部附件上传完成后的正式发布、Latest及prerelease=false最终核验、所有附件匿名下载SHA与20个旧Release最终复验。
- 发布受阻及小分卷验证记录已本地提交；其后向main追加推送也遇到`schannel: failed to receive handshake`，没有声称这些补充记录已同步远端。业务源码`2336bd9`及版本标签的早先推送成功与本次追加推送失败分开记录。

没有执行草稿转正式发布，不把上传尝试称为成功。旧Release不覆盖，用户资料不上传；已成功的本次附件与完整本地包保留。新建GitHub Actions打包未执行，未新增CI权限、令牌或云端推理服务。

## 可用本地产物与续传检查点

- EXE：`src-tauri/target/release/researchclip.exe`。
- NSIS：`src-tauri/target/release/bundle/nsis/ResearchClip_3.1.2_x64-setup.exe`。
- 完整离线分卷/组装脚本：`artifacts/publish-v312-small`；001至025必须全部一起使用。
- 初始发布日志：`artifacts/publish-v312-final/publish.log`。
- 小分卷清单：`artifacts/publish-v312-small/manifest.json`；成功回执为同目录的`*.upload.json`，重试日志均保留。
- 源码标签保留原提交；分卷大小、实际传输记录及最终发布回执另行提交，不移动已推送标签。

网络可用时先核对已有附件的名称/大小/digest，再补齐缺失项；不得覆盖已匹配的完成项或忽略失败。只有全部完成并复验，才把本文件的发布状态更新为PASS。

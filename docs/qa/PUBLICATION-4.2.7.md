# v4.2.7 正式发布回执

[正式Release](https://github.com/wpy-sketch/ResearchClip/releases/tag/v4.2.7)，ID424985665，正式非预发布，附件327项。源码标签冻结于5fbbb697a4d6f42481a05efcdfb0159ee23fdade。

GitHub Actions [继承任务](https://github.com/wpy-sketch/ResearchClip/actions/runs/40440995893)（run40440995893，job121981047289）成功：上一版319项无冲突附件保持原名原字节，当前8项另行上传；名称、大小和SHA256逐项核对。重名SOURCE/SHA按原名和字节保留于版本metadata ZIP，历史归档成员有完整定位。

[匿名公开下载验证](https://github.com/wpy-sketch/ResearchClip/actions/runs/40441830288)（run40441830288，job121983733151）成功。327附件均从公开URL实际下载字节，大小与SHA256匹配；下载不携带Authorization。验证工作流提交5fbbb697a4d6f42481a05efcdfb0159ee23fdade，机器回执见[公开校验](evidence/v427-public-verification.json)。

此前46个Release、4411项原附件的ID、名称、大小及摘要前后不变，历史核对PASS；未改旧标签或附件。本版包含当前便携包、NSIS、离线指南与全部历代功能组件的原附件内容，不只提供旧链接。

本轮前端741/92文件、Rust308（8默认忽略、另显式执行真实Zotero集成1项）、Windows原生155（按脚本累计，公共场景重复）、指南14、发布工具23检查通过；独立npm ci327包、无依赖版本变化。指南33章253练习68图。NSIS17,626,209字节、未签名，SHA256 d744816498b3395be5a8bd4755fc632ebd888794efc500de273787add5b15f08，实际隔离安装/卸载与保护5检查通过，结束无安装残留。

schema20以事务迁移保存标准批注意图、独立处理修订和来源刷新设置；旧截图、Markdown与缓存保留，未来版本拒绝写入。默认ResearchClip资料只读审计27文件0变化。独立Zotero10.0.6验证两客户端版本冲突、删除/修改竞争、固定key复核、三类型CRUD与原PDF摘要不变；真实授权GUI、真实只读群组/非作者权限和物理多屏DPI仍需人工验收。自动刷新默认关闭且只有读取，失败修改不静默丢弃。

完整设计、修改文件、实际失败修复与人工步骤见[开发验收](RELEASE-4.2.7.md)、[文件清单](CHANGED-FILES-4.2.7.md)和[检查证据](evidence/v427-editor-checks.json)。无新增第三方依赖、直接Zotero数据库写入、原PDF改写、私有笔迹转换、Markdown导出PDF或独立备份/恢复产品。

本回执以标签之后的独立文档提交保存，源码标签与附件保持不变。Step16完成后停止；未自动进入Step17。

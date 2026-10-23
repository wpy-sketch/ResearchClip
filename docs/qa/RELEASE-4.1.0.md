# ResearchClip v4.1.0 发布验收

目标仓库：https://github.com/wpy-sketch/ResearchClip 。发布内容为Step00/01，不含Step02。

基线和详细运行结果见[Step01报告](RESOURCE_MODEL_STEP01.md)：前端429测试、Rust230通过/7忽略、Clippy/格式检查、Windows构建与隔离旧库六组通过。额外编辑器关闭超时仍未解决，不能称全量原生回归通过。

本轮只同步应用版本到4.1.0并构建/发布，依赖版本不升级。修正全文件LFS属性为普通文本自动换行与图片binary，避免新源码被误提交为LFS指针；既有LFS对象保留，不重写历史。GitHub发布附件包含Windows安装器、独立EXE、离线指南及SHA256清单。可选模型和插件沿用既有发布链接。

真实用户库未操作；安装器人工升级、签名与多屏硬件不视为已通过。新增检查和发布回执随后记录。

实际重跑（2026-10-23）：npm run check退出0，63文件429测试；npm run test:rust退出0，230通过/7忽略；fmt/Clippy退出0；文档检查132份/774链接通过。Windows NSIS和debug构建退出0，resources-native release-v410六组通过。NSIS隔离安装/忙进程拒绝/卸载/用户数据哈希保护四组通过，测试安装已清除；签名为NotSigned。此安装测试不启动正式应用读取真实资料，不等同真实用户升级人工通过。

沙箱首轮前端缓存加载失败、Rust122通过/108失败/7忽略、NSIS启动系统DLL重定位错误623；正常Windows权限重跑以上通过。首轮失败日志保留于artifacts/release-v410，不改断言掩盖环境失败。

发布传输调整：原始EXE直连/代理上传超时，改为便携ZIP内包含相同EXE字节，安装器不变；验证ZIP完整性及解压后的EXE SHA256。只替换本轮未发布草稿的校验清单与未完成附件，不修改任何历史正式Release。

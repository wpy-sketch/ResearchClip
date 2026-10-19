# ResearchClip 网页剪藏扩展（A02 / v1.0.2）

独立 Manifest V3 扩展，Chrome / Edge Windows 优先。只在点击时读取当前页，没有常驻内容脚本、后台网页扫描、Cookie权限或HTTP服务。**完整浏览器工具栏点击/activeTab用户授权链仍需人工验收**；已有真实Edge提取、浏览器Native Messaging项目读取和Host到桌面完整保存证据，不能混为同一验收。

## 本地安装（使用Release扩展包，无需Node/Rust）

1. 下载同版本 `ResearchClip-WebClipper-v1.0.2-windows-x64.zip`，核对SHA256，解压到你准备长期保留的目录。移动目录后要先注销旧Host，再从新目录注册；不要把Host放在会自动清理的临时目录。
2. 保存并退出旧ResearchClip，运行v1.0.2。在设置→**浏览器网页剪藏**点击**启用网页剪藏连接**。这是本次应用会话的开关，重启后默认关闭。
3. 在扩展目录执行以下命令。只注册当前用户，不需要管理员；已存在其他位置的同名注册时会拒绝，不覆盖。

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\register-host.ps1
```

4. 打开 `chrome://extensions` 或 `edge://extensions`，启用开发者模式，点击**加载已解压的扩展**，选择扩展包内的 **dist** 文件夹。此步骤由用户确认执行。
5. 检查扩展ID应为 `ackihaehocfemjednfdfmnnpiemdlmjo`。来源不一致会被Host拒绝。第一版没有发布到扩展商店，不自行增添任意扩展来源。
6. 在普通网页点击扩展图标。先点**刷新项目连接**，再按下面的流程剪藏。

## 使用顺序

1. 选择**链接与来源 / 选中文字 / 主要正文 / 选择图片**之一。
2. 选中文字模式先在网页正文选择，再点扩展。不会读取输入框、密码、表单或可编辑区域。
3. 点**读取当前页并预览**；检查Markdown、标题和图片，必要时修改标题。正文提取并不保证任意网站；复杂页面可改为选中文字或链接。
4. 图片模式勾选要保存的图片，图文关联整体传输。可勾选**允许复制可访问的远程图片**：仅尝试当前网页同源HTTPS、PNG/JPEG/WebP，不带Cookie、不跟随重定向；最多4张，单图256 KiB/419万像素、总384 KiB。跨域、登录限制、非HTTPS和超限图片保留来源并写明状态，不绕过访问控制。
5. 选择ResearchClip项目或**未分类**，点击**保存到 ResearchClip**。桌面弹出通知；点击**打开 Markdown**，沿用现有右分栏和未保存确认。
6. 通信失败时保持内容不变再保存，会复用请求ID。浏览器会话内暂存待确认内容；下次打开会清理超过10分钟的记录，浏览器退出清空会话存储。改内容或重新预览会生成新剪藏。
7. **取消等待**停止网络准备/等待；已交给桌面的请求可能完成，不自动删除已保存资料。用相同请求重试确认结果。

剪藏保存在应用受管 `web-clips-v1/<请求ID>/` 内的标准.md和assets，登记到已有SQLite项目文档索引；标题和注释可在分栏中编辑，另存为会带上本地图片。远程图引用在ResearchClip中默认不加载，其他Markdown软件可能自行联网加载，打开前注意来源。

## 从源码独立构建

要求Node≥22.12、npm，以及Rust。以下命令不依赖根项目node_modules。

```powershell
cd browser-extension
npm ci
npm run check
cargo test --workspace --locked
cargo build --release --bin researchclip-native-host --locked
.\register-host.ps1 -HostPath .\target\release\researchclip-native-host.exe
```

`dist`可本地加载；`target/release/researchclip-native-host.exe`是独立进程，不链接Tauri、不开数据库。桌面应用需另行按根README构建/运行。Root快捷脚本：`npm run build:extension`、`npm run test:clipper:browser`、`npm run test:clipper:native`。

## 注销与排错

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\register-host.ps1 -Action Unregister
```

只删除明确属于本目录的4个HKCU注册项；不会删除扩展、Host、截图、Markdown或图片。再由用户在浏览器扩展管理页移除扩展。切换Host路径时请先注销旧位置，使用新目录；不覆盖已有清单。

| 提示 | 检查顺序 |
| --- | --- |
| Host not found / access forbidden | 注册脚本是否成功、解压目录是否移动、扩展ID是否匹配、浏览器是否需重启 |
| DESKTOP_UNAVAILABLE | ResearchClip是否已启动、设置里的本次会话开关是否启用、Host和桌面版本是否匹配 |
| BUSY | 等待截图/文件任务结束，再保持内容不变重试 |
| 正文为空/权限拒绝 | 从普通HTTP(S)网页由图标触发；内部页、文件页、表单及部分跨域frame不支持 |
| 图片未本地复制 | 查看文档“图片资源状态”；保留原始来源，不偷偷重试有凭据访问 |
| 同一ID内容不同 | 重新读取预览创建独立剪藏；不要删除原文档或数据库解决 |

## 依赖、通信和验收

扩展运行时：Readability0.6.0（Apache-2.0）、DOMPurify3.4.17（MPL-2.0 OR Apache-2.0）、Turndown7.2.4（MIT）；静态离线打包，不从CDN加载。Readability本身不保证清理安全，另加DOMPurify并移除表单；预览使用文本框，桌面沿用skipHtml的安全Markdown渲染器。

通信及威胁边界见[协议](PROTOCOL.md)；实际命令、失败和人工清单见[验收](../docs/qa/RELEASE-1.0.2.md)。普通文章、复杂网站、实际图标点击、同源图片下载和Chrome逐个验证，不能用Mock或直连Host冒充完整浏览器验收。

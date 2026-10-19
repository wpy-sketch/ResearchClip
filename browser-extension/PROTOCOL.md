# A02 通信与文件契约

浏览器扩展（用户点击/activeTab）→官方Native Messaging标准输入输出→独立RustHost→仅127.0.0.1随机端口的长度帧IPC→ResearchClip领域服务→标准Markdown/受管assets及原有SQLite项目索引。**没有HTTP接口，也没有由扩展或Host直接访问SQL的入口。**

## 来源、认证、限额

- 固定公钥生成稳定的本地扩展ID；Host清单只允许该origin，Host再次检查首个argv必须为完整`chrome-extension://ID/`，仅额外接受数字parent-window参数。桌面再次检查origin。
- 每次用户启用连接时使用getrandom系统CSPRNG生成256位令牌，原子写入已核对的非重解析配置目录中的`clipper-endpoint-v1.json`。Host仅从当前用户固定配置目录发现该端点；令牌不返回WebView、不打印日志，退出/停止移除自己持有的端点。
- 本地IPC不是浏览器可调用的通用协议；没有令牌的报文先拒绝，再处理业务。信任边界是当前用户配置目录的OS访问控制，**不声称防御已拥有同一用户文件权限的恶意本地进程**；本机进程可伪造argv，不把argv当密码学身份。
- 4字节小端UTF-8字节长度，单帧上限768 KiB，扩展预检700 KiB。所有结构deny_unknown_fields、版本1、32位小写hex请求ID。正文192 KiB、标题200字符、图片引用32张；本地PNG最多4张/单256 KiB/合计384 KiB，桌面再次解码和像素校验。
- Host读取标准输入10秒截止，连接2秒、读响应15秒、写3秒；扩展20秒停止等待。桌面单线程依次处理，socket读写3秒，SQLite沿用5秒busy timeout；没有无界线程或请求帧缓存。
- 操作仅ping/projects/save。报文不含本地路径、SQL、命令行、程序名或HTML执行指令。响应仅状态、项目列表或不透明文档ID，不回传配置令牌或磁盘路径。

## 文件、一致性与重试

Desktop Storage校验后使用`web-clips-v1/<requestId>/`；请求摘要和初始时间先安全落盘。使用已有DocumentFiles写入禁止覆盖的.md与不可变PNG资产，校验所有引用；然后在事务中向既有markdown_documents登记请求ID、规范路径、项目关系。schema v3不变，没有迁移需求。

重试比较同一请求ID的SHA256摘要：相同则返回已有文档，不重写用户编辑；不同则拒绝。SQL提交失败留下可核验文件，原请求重试检查内容/资产再完成登记；外部修改导致差异时保留文件并报错。项目移动不改变图片原始路径，截图原图和数据库已有记录不重置。应用内Markdown改名后，同ID重试仍要求路径处于该受管请求目录，不创建副本。

标准Markdown中的图片使用assets相对路径；未复制图片保留经过校验的HTTP(S)源和失败状态。`request.json`是内部去重元数据，不是专有文档格式；单独移动.md与assets仍可阅读。原始网页HTML不交给桌面执行。

## 核对的官方文档

- [Chrome Native Messaging帧、origin与1MiB输出限制](https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging)
- [Chrome scripting与activeTab临时权限](https://developer.chrome.com/docs/extensions/reference/api/scripting)
- [Edge Native Messaging注册位置](https://learn.microsoft.com/en-us/microsoft-edge/extensions/developer-guide/native-messaging)
- [Mozilla Readability API与安全建议](https://github.com/mozilla/readability)
- [DOMPurify](https://github.com/cure53/DOMPurify)、[Turndown](https://github.com/mixmark-io/turndown)

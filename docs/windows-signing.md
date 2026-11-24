# Windows 发布签名（SignPath，尚未启用）

现有 tag 发布流程仍默认发布未签名版；日常 push、PR 和手动运行只执行现有检查。只有在配置完成后把仓库变量 `OFFWORK_WINDOWS_SIGNING_MODE` 设置为 `signpath`，正式的 `vX.Y.Z` tag 才会要求签名。未设置或设置为 `off` 保持原行为，其他值会阻止 tag 发布。

## 维护者一次性配置

1. 先完成 [SignPath 开源签名申请](https://signpath.org/)及组织／项目设置。项目应绑定本仓库的 GitHub Actions 构建来源；配置发布签名策略、审批人和可信时间戳。不要假定申请一定通过。
2. 创建 Artifact Configuration：GitHub Actions 上传的是 ZIP，签名目标是根目录下的 `offwork.exe`。保持输出结构不变，确保返回的 ZIP 解包后仍是 `offwork.exe`。参考 [SignPath GitHub 集成](https://docs.signpath.io/trusted-build-systems/github)。
3. 在 GitHub 仓库 Settings → Secrets and variables → Actions 设置：
   - **Secret**：`SIGNPATH_API_TOKEN`，使用仅有此策略提交权的 API Token。不要提交到 Git 或发到聊天。
   - **Variables**：`SIGNPATH_ORGANIZATION_ID`、`SIGNPATH_PROJECT_SLUG`、`SIGNPATH_SIGNING_POLICY_SLUG`、`SIGNPATH_ARTIFACT_CONFIGURATION_SLUG`。
   - **Variable**：`SIGNPATH_SIGNER_SHA256`，从可信的 SignPath 证书资料取得签名者叶证书的 SHA-256 指纹（64 个十六进制字符，无空格或冒号）。证书更换时先更新此值。客户端报告的 SHA-1 Thumbprint 不是这个值。
4. 先在 SignPath 测试策略中验证归档结构、签名证书、时间戳及人工审批时限；本 PR 不请求签名、不创建测试 tag、不发布。准备就绪后才将 `OFFWORK_WINDOWS_SIGNING_MODE` 设置为 `signpath`，并按原流程推送下一个正式版本 tag。如策略需要人工审批，在工作流等待期间批准该请求。

## Foundation 申请检查清单（政策草案，尚未生效）

- 查阅 [Foundation 条件](https://signpath.org/terms)，确认开源许可、当前可下载版本、项目说明和所签二进制均符合要求。只签本项目自行构建的 `offwork.exe`，不签上游依赖二进制。
- GitHub 与 SignPath 的团队成员都启用 MFA。申请前填上实际成员或权限组：作者／提交者 **待确认**，审阅者 **待确认**，签名审批人 **待确认**。非提交者的改动由指定成员审阅，每次签名由指定审批人批准。
- 在 SignPath 的 Artifact Configuration 启用文件元数据约束：所有产品名称字段必须匹配 OFFWORK 实际构建的产品名，所有产品版本字段必须匹配本次构建版本；配置前从实际 EXE 核实字段，不签其他产品。
- 隐私说明 **待维护者审核**：逐项核对应用、依赖、遥测及网络行为后再写准确声明，或链接已发布隐私政策；现在不作无联网或无数据传输的承诺。
- 获批后再在项目首页和下载／发布页加 `Code signing policy` 栏目或链接，按 Foundation 要求标注实际签名服务和证书提供方，列出团队角色及隐私政策。申请期间不要宣称已获签名服务或把旧版标为已签名。

## 启用后的发布行为

签名输入只包含未签名的 EXE，与最终发布产物分开。正式发布前，Windows runner 用 Authenticode 检查签名、时间戳及签名者证书指纹，并再次核对版本，再对最终已签名的 EXE 生成 SHA-256。缺少配置、签名失败、无有效时间戳或证书不符都将停止发布，不会自动回退到未签名版。已有发布及附件不会被重新签名或替换。有效签名可验证发布者，不保证 SmartScreen 立即不提示。

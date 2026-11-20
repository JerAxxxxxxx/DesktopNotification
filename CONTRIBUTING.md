# 贡献指南

本项目的开发和验证目标是 Windows x64。准备 Windows 环境、运行项目和执行检查的方法见 [README](README.md#本地开发)。

## 从哪里改

- 设置页、提醒卡片和悬浮窗分别在 `src/components/`。下班计时逻辑在 `src/countdown.ts`，节假日数据和日期计算在 `src/holidays.ts`。
- Rust 调度在 `src-tauri/src/scheduler.rs`，设置模型和读写分别在 `model.rs`、`storage.rs`。窗口、托盘和前后端命令在 `lib.rs`。
- 更新节假日时，以国务院办公厅公布的放假区间为准，记录通知来源，并验证假期前一天、开始日和结束日。

## 提交反馈或改动

- 报告问题时附上应用版本、Windows 版本、复现步骤、预期与实际行为；悬浮窗问题请注明是否使用多屏和显示缩放。请先从截图或日志中删去个人信息。
- 提交改动时说明变化和验证范围。修改置顶、拖动、托盘、自动启动或提醒窗口时，尽量用 Windows 桌面复测；浏览器预览无法验证这些系统行为。
- 保留 npm 和 Cargo 锁文件。依赖、构建产物、本机临时文件和历史发布包不进入源码。发布免安装 EXE 时，附上使用说明、校验值和 [MIT 许可证](LICENSE)。

Windows 自动检查配置位于 `.github/workflows/check.yml`。发布时先统一应用版本号，在对应提交上创建 `v主版本.次版本.修订号` 标签并推送。标签触发测试、Windows x64 免安装构建和 GitHub Release 发布。附件包含 EXE、SHA-256 校验文件、使用说明和 MIT 许可证。

# DesktopNotification 项目约定

这是一个 Windows x64 下班提醒工具，使用 Tauri、Vue、TypeScript 和 Rust。产品目标是让用户一眼看到距离下班或已加班多久，并在设定时间收到提醒。

- 桌面悬浮窗应紧凑、易读。优先保留计时数字、状态标签、左键拖动和隐藏入口；不要恢复默认网页右键菜单。托盘菜单用于管理应用。
- 主界面和悬浮窗共用计时逻辑。区分下班／加班计时与提醒调度，避免暂停、推迟或停止提醒时意外改变计时。改动计时规则时，核对到点切换、跨日和休息日。
- 按改动选择验证：计时逻辑用 `npm.cmd run test:countdown`，Rust 逻辑用 `cargo test --manifest-path src-tauri/Cargo.toml --locked`，前端构建用 `npm.cmd run build`。涉及置顶、拖动、隐藏或右键交互时，优先在 Windows 桌面窗口验证；如果只做了浏览器预览，就明确说明。
- 正式交付只提供 Windows x64 免安装 EXE。用 `npm.cmd run package` 构建，产物为 `src-tauri/target/release/offwork.exe`；发布文件放在 `release/`，沿用 `Offwork-<版本>-x64-portable.exe` 命名，并更新校验值和使用说明。交付时直接给出免安装文件链接；说明中保留 WebView2 Runtime 依赖。

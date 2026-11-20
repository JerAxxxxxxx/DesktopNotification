# 下班啦 · OFFWORK

一个独立运行的 Windows 下班提醒工具。使用 Tauri 2、Vue 3、TypeScript 和 Rust 开发，全部设置保存在本机。

支持下班和加班计时、置顶悬浮窗、托盘提醒与 2027 年节假日倒计时。

## 使用方式

1. 启动应用，设置下班时间、停止提醒时间、重复间隔和提醒日，然后点击「保存设置」。
2. 点击「测试提醒」，预览使用已保存设置的桌面提醒卡片。在预览中点击暂停或稍后提醒只会关闭卡片。正式提醒到点时会接替预览。
3. 到点后会在右下角显示卡片。可以选择「今天不再提醒」或按设置间隔稍后提醒。
4. 桌面右上角默认显示下班倒计时。双击计时区域打开设置，按住左键可拖动，点「×」可隐藏。倒计时平时用紫色，下班前 30 分钟切换为青绿色，下班后切换为暖橙色。隐藏后可从设置或托盘重新显示。
5. 关闭设置窗口后，应用继续在托盘运行。单击托盘图标可以恢复窗口，右键菜单可以显示或隐藏倒计时、测试、暂停、恢复或退出。
6. 「登录后自动启动」默认关闭。开启后登录 Windows 时不打开设置页，倒计时按上次的显示状态恢复。

默认提醒时间为工作日 18:00～21:00，每 10 分钟一次。下班提醒继续按设定的星期运行，不随节假日和调休自动改变，也不支持跨午夜的提醒时间段。

主界面的「节假日倒计时」内置 2027 年元旦、春节、清明节、劳动节、端午节、中秋节和国庆节。按北京时间的日历日期倒数到放假首日；展开可看全年日期和假期天数。无需联网，暂不包含 2028 年数据。日期依据[国务院办公厅 2027 年放假安排](https://www.dxh.gov.cn/ZWGK/zc_zwgk/ZFWJ/TZGG/202611/t20261106_2875104.shtml)。

主界面和悬浮窗每秒更新，按当天已保存的下班时间计算。到点切换为「已加班」，从 00:00:00 开始计时，到次日重置。休息日显示「今天休息」。暂停、推迟或到点停止提醒都不会影响计时。

提醒卡片默认 15 秒后自动收起，也可以手动关闭。收起本次卡片不会停止后续提醒。「今天不再提醒」会保存当天暂停状态，重启应用后仍生效，次日恢复。已有卡片时不会叠加新的卡片；电脑睡眠、关机时不会提醒，唤醒后在有效时段内最多合并补一次。

## 运行和分发

目标系统：Windows x64，需要 Microsoft Edge WebView2 Runtime。

- 双击 `Offwork-<版本>-x64-portable.exe` 即可运行。发布后可从 Releases 获取；也可以按下方步骤自行构建。缺少 WebView2 时，需先安装 WebView2 Runtime。
- 测试提醒不会模拟修改系统时间，也不会自动更改用户设置。

设置保存在 Windows 当前用户的应用数据目录中，应用标识为 `com.hugo.offwork`。内容包括设置和当天提醒状态。应用不采集或上传这些数据。

## 本地开发

需要 Node.js 24、Rust stable MSVC、Microsoft C++ Build Tools（C++ 桌面开发）、Windows SDK 和 WebView2。npm 和 Cargo 锁文件固定依赖版本。

```powershell
npm.cmd ci
npm.cmd run tauri dev
```

检查、测试和打包：

```powershell
npm.cmd run test:countdown
npm.cmd run build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml --locked
npm.cmd run package
```

打包只生成免安装程序，输出到 `src-tauri/target/release/offwork.exe`。

## 结构

- `src/main.ts`：根据窗口类型加载对应界面。
- `src/components/`：设置页、提醒卡片、计时悬浮窗和节假日面板。
- `src/countdown.ts`、`src/holidays.ts`：下班计时和节假日日期计算。
- `src-tauri/src/scheduler.rs`：可控制时钟的调度状态机及边界测试。
- `src-tauri/src/storage.rs`：配置的临时文件写入与替换、读写错误处理及测试。
- `src-tauri/src/model.rs`：设置校验、每日状态序列化与恢复。
- `src-tauri/src/platform.rs`：Windows 可交互桌面检查与系统提示音。
- `src-tauri/src/lib.rs`：托盘、窗口、命令、自动启动与后台调度的集成。

参与开发见 [贡献指南](CONTRIBUTING.md)，后续方向见 [路线图](PLAN.md)，已覆盖的场景见 [验证记录](VERIFICATION.md)。

## 许可证

本项目使用 [MIT 许可证](LICENSE)。

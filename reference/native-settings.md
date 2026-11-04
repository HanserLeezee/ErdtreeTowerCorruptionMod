# 原生设置页

ERNativeUI 1.3.0（MIT）：https://github.com/Flammrock/ERNativeUI

游戏「系统 → 游戏选项」中的「黄金树爬塔」子菜单提供调试开关、界面音效音量、单人 DLC 模式，修改后选择「保存设置（重启生效）」。五个快捷键位于原生「系统 → 键盘／鼠标配置 → 按键配置」的「黄金树爬塔」分组；改键、清除绑定自动保存并即时生效。与启动器共用 DLL 目录中的 `tower-options.ini`，不使用 ERNativeUI 的独立存储。

`distribution/native_settings_build.py` 下载固定版本、验证官方 SHA256，再构建 C++17 提供程序。`native_build.py` 和 `package.py` 均携带 `native-ui`。主 DLL 在版本验证完成后的工作线程依次加载主机、提供程序并注册；不依赖 ME3／YAFSML 的额外配置，不在 DllMain 锁内注册。两个组件保持进程生命周期，不支持热卸载。

配置保存使用临时文件及 Windows 原子替换；保留未知行，仅修改本次变化的设置，合并被修改键的重复行。失败写入 `tower-preview.log`，原生提示框通知玩家。启动日志 `NATIVE_SETTINGS registration result=0` 表示注册成功，尚不等于游戏画面验证通过。1000+错误码表示连接失败，2000+错误码表示注册失败。

官方兼容表确认游戏 2.7.2.0（App 1.17.2，SHA256 F9118FEA2CFD8FA3901E8060BFADB1552CB2471771DED8A5B662C4E298FFB328）。2026-11-02 本机 YAFSML＋Seamless 2.0.2 启动验证：注册成功、1 个自定义标签、5 个快捷键滑动控件，原生界面资源补齐成功，爬塔渲染及无缝复活接口正常初始化。尚未验证实际菜单操作、保存后重启或双机联机。

ERNativeUI 1.3.0 原生 input bindings 不支持 F1–F12。按用户最新选择完全使用原生按键页，默认键为 Home、Insert、J、C、Delete；旧文件中不支持的键迁移为对应默认值。None 表示未绑定。完整映射遵循 erui.h 的 73 个键码。注册五个 keyboard-only actions，回调通过原子位图交给爬塔渲染线程，事件只保留一帧；爬塔自己接管输入时仍使用 ImGui 按键检测。原生改键和弹窗期间不派发动作。保存失败恢复界面的原绑定；其他设置保存不会覆盖快捷键。旧捕获窗口已删除。

后续实测发现顶部标签切换报错 `Graphics carrier switch escaped validated pending frame; custom transitions frozen`，启动注册成功并不代表入口可用。当前改为官方支持的 Game Options 子菜单，避免注册自定义顶部标签；子菜单显示已由玩家截图确认。

验证入口：进入系统菜单的游戏选项，选择列表中的「黄金树爬塔」，调整音量并保存；重启检查设置保持及生效。现有地图、构筑、奖励页不迁移。

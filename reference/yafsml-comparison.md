# YAFSML 与 ME3 本机对比（2026-11-02）

独立测试入口由 `distribution/loader_compare.py` 生成，没有替换正式启动器或修改游戏目录的 DLL。对比使用 `F:/爬塔启动器` 的同一份 v103 DLL 和场景资源，DLL SHA-256 为 `0ec71282c4a9b613bafa2f82dcbe383b13291bb57ca79a90e4a15e9a78e47103`。

- ME3：本机安装的 0.13.0。
- YAFSML：官方发布 0.11.0，源码提交 `35a78d3a50ab84a574ca69331c91015a2752e6cf`。
- 官方 ZIP SHA-256：`50238eda68eb6034a56da10f9b45c74aa2b0fff4b633d606fa29618ed5811aa2`。
- 来源：https://github.com/soarqin/YAFSML/releases/tag/v0.11.0
- 配置依据：https://github.com/soarqin/YAFSML/blob/v0.11.0/README.zhCN.md
- 无缝：Seamless 2.0.2，复制现有私人配置到各自测试目录，仅将存档后缀改为 `ertowerloadertest`；不输出联机密码。
- 单机测试文件：`ER_TOWER_LOADER_TEST.sl2`。YAFSML 首次为该文件复制了原版存档，原存档没有被替换。爬塔运行记录和日志位于测试目录。

## 实测

计时从测试脚本记录 UTC 启动时间到模组日志的 `First overlay frame rendered`，不是点击继续后的读条，也不是 FPS 测试。

| 入口 | 启动到首帧 | 结果 |
|---|---:|---|
| ME3 单机 | 2.350 秒 | 进入大赐福，NPC 交互正常记录 |
| YAFSML 单机首次 | 7.988 秒 | 进入大赐福，NPC、背景音乐、幻化模块初始化有日志 |
| ME3 无缝 | 4.522 秒 | ersc、爬塔、复活请求接口和叠加界面初始化成功 |
| YAFSML 无缝首次 | 8.872 秒 | ersc 先于爬塔加载，进入大赐福 |
| YAFSML 无缝缓存复测 | 5.605 秒 | BootBoost 缓存命中，进入大赐福 |

先前两次 ME3 启动没有产生爬塔会话，退出时 Steam 启动环境尚在准备；不计入成功启动耗时。每项成功计时均是单次采样，缓存条件不完全相同，不据此宣称稳定的性能差距。当前没有证据证明替换后比 ME3 更快。

YAFSML 日志确认 `Menu.IsEnableOnlineMode=false`、regulation、文本、菜单、高清公共贴图、公共特效的覆盖生效。保持其默认内存替换和 BootBoost，未开启 Arxan 中和或 CPU 亲和性调优。

## 共同问题与未验证范围

两个加载器在 Seamless 2.0.2 下都报告“Spirit usage tracing skipped：游戏物品接口版本不匹配”。召唤点刷新仍安装，但骨灰使用追踪及依赖它的双召唤保护没有完成初始化。这不是 YAFSML 独有差异，不能把本轮测试视为骨灰功能通过。

后续已针对该跳转链完成兼容修复，并在 YAFSML + Seamless 2.0.2 下验证初始化成功，见 [骨灰接口兼容记录](seamless-spirit-trace.md)。上表保留修复前的原始对比结果。

两个无缝入口都报告复活请求接口 ready；尚未做双机组队、死亡观战、战后复活、客机读条以及完整爬塔通关测试。正式启动器继续使用 ME3。

## 复用入口

```powershell
python distribution/loader_compare.py <现有整合包目录> <已解压的官方YAFSML-0.11.0目录> <游戏Game目录> <新测试目录> --mode Seamless
```

每个测试目录有 `ME3/Start-Test.cmd` 和 `YAFSML/Start-Test.cmd`。只能在保存退出当前游戏后切换。生成器要求无缝私人配置已由标准入口生成，额外 DLL 插件存在时先停止并要求明确映射，避免静默遗漏。对比目录中的资源 junction 指向现有安装，只应由标准入口更新资源；不要删除 junction 的目标内容。

本机日志及采样数据位于 `F:/Codex/2026-10-26/https-www-nexusmods-com-eldenring-mods/work/yafsml-comparison/`，包括 `comparison-results.json`。

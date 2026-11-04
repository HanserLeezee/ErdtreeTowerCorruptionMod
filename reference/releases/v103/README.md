# 1.5.0（构建 103）源码与生成资源补交

本页记录首次补交时的结果。后续功能合并与当前资源差异见 [2026-11-04 整合记录](../../integration-20261104.md)；`main-resource-differences.json` 已随当前资源更新，不再只有最初的 8 项。

1.5.0 发布时的 Git 基线是 `f99b4a8aa71cc43302892fb7d0493ec37d5d6fb5`，实际构建还包含未提交改动。以前只检出公开提交无法复现安装包的场地指纹。本次从发布时保存的二进制 Git 差异、未跟踪文件快照和安装清单还原，129 个修改文件的 SHA-256 全部匹配，没有使用当前开发目录的地图替代正式版。

- 发布页：https://github.com/tamikip/erdtree-tower/releases/tag/v103
- ZIP SHA-256：`e2f8a478a6398e8b247aa50388b5868a77d4d19778cbe30c16296bbe9fd446be`
- `source-state.json`：当时的基线、129 个文件哈希及原始差异补丁哈希。
- `scene-manifest.json`：从正式版 `tower-install.json` 提取的 1885 个场景资源；附文本换行归一化哈希。
- `main-resource-differences.json`：用户明确要求保留后续功能，因此列出当前 main 相对正式版的 8 个资源差异及各自原因。不能宣称 main 所有资源与 1.5.0 完全相同，也不要求新 DLL 与旧 DLL 二进制相同。

## 指纹与恢复内容

龙王平台必须保持：

```text
map/mapstudio/m13_00_00_00.msb.dcx|323232|acfdb3526c6f9c77
```

所有正式版地图、事件及场地清单已恢复。Boss 池包含第二幕单只碎星拉塔恩（模型 4730），联机额外槽位包含它的两行预置数据。梅琳娜的原生参数统一为 `21800000`。补交原版陨石转阶段事件生成代码，并保留后续限定于单人碎星动态实体的查找修复和调试入口。

8 个有意保留的差异：两个新版词条描述表；regulation 中后续强化、词条、誓约、附魔和原生余烬显示定义；中文物品包中 26 个余烬名称；高清／低清原生图集及布局中新增的余烬图标。regulation 使用三方逐字段合并，冲突数为 0，其余正式版参数保留。图集和文字从正式版重新添加余烬资源，没有混入当前开发版的其他地图或灵庙修改。

恢复的多人代码包含新增消息类型。整合版 Steam 协议为 62、手动 TCP 协议为 42；联机成员须使用相同新版 DLL，不能把它与原始 1.5.0 DLL 混用。

## 自检

在仓库根目录运行：

```powershell
python distribution/verify_release_sources.py --baseline reference/releases/v103/scene-manifest.json --differences reference/releases/v103/main-resource-differences.json
cargo test
cargo test --manifest-path native/Cargo.toml --lib
cargo build --manifest-path native/Cargo.toml --release --lib
python -m unittest discover -s tests -p test_release_sources.py
```

资源核对结果为 1877 个完全相同、8 个已说明的差异。脚本同时逐项检查当前 `arenas-resources.txt` 的大小和 FNV-1a，而非仅比较清单文字。

本次核心 `cargo test`：333 项通过、2 项忽略；原生：389 项通过、1 项忽略。Release DLL 编译成功，SHA-256 为 `60920bac67f74494a7ad87904ebbb850d36acb8a61d105fe079ad7280377ecdc`，并检查到 DLL 内嵌正式版龙王地图完整指纹。发布输入检查的 2 项 Python 测试及加载页测试也通过。

SceneBuilder 的依赖目录可用 `dotnet build scene-builder/SceneBuilder.csproj -c Release -p:TowerDependencies=<依赖目录>/` 指定。运行 `elite-pool-verify` 对 16 个已生成场地检查拉塔恩与联机实体／事件。Boss 和完整原生资源生成读取仓库的 `scene-assets`，或显式 `TOWER_SCENE_SOURCE`，不再从另一个本地 `outputs/erdtree-tower` 偷读 Boss 池。

`v103-service-audit` 模式另行回读地图，检查梅琳娜的参数和模型缩放、商人实体覆盖以及两条碎星联机槽位；这仍是资源检查，不是游戏对话和战斗操作测试。

自动校验不能替代第一关、营地对话、商人受击／猎人和传送门的游戏实测。本次没有把构建或场地回读称作上述实机流程通过；发布记录中的原始 1.5.0 也标记了 `gameplayVerification=false`。

## 后续正式发布

先把本次生成结果同步到仓库 `scene-assets`，确认 DLL 编译读取的目录与安装包资源源目录一致，提交源码和生成文件，再打正式包。`distribution/package.py` 现在会拒绝包含未提交源码或生成资源的发布输入；不要用旧目录替换已编译 DLL 所引用的指纹。

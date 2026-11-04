# 发行包未使用文件清理

本次只调整 `distribution/resource_filter.py` 中列出的 20 个文件的打包行为，完整包和精简包均适用。源码文件保留，编译、场景生成、旧存档兼容逻辑不变。未知文件默认保留，不以“搜索不到完整路径”自动判定未使用。

## 从下载包排除

| 文件 | 依据 |
|---|---|
| `affix-catalog.txt`、`affix-v2-catalog.txt` | `src/affixes.rs` 使用 `include_str!`，内容已编译进 DLL |
| `affix-details.tsv`、`affix-v2-details.tsv` | `native/src/reward_ui.rs` 使用 `include_str!` |
| `affix-rarities.tsv`、`affix-v2-rarities.tsv`、`reward-rarities.tsv` | `reward_ui.rs`、`src/affixes.rs`、`src/contracts.rs` 编译时嵌入 |
| `protected-ash-pools.tsv`、`random-ash-details.tsv`、`random-ash-weapon-rows.txt` | `native/src/random_ashes.rs` 编译时嵌入 |
| `talisman-definitions.tsv`、`talisman-details.tsv`、`talisman-rarities.tsv` | `src/talismans.rs`、`reward_ui.rs` 编译时嵌入 |
| `affix-effect-remap.tsv` | `AffixPoolPatch.cs`、`AffixTextFix.cs` 场景生成中间表 |
| `random-ashes.tsv` | `EquipmentStyleAssets.cs` 等构建工具的索引及校验数据 |
| `talisman-catalog.txt` | `TalismanAssets.cs`、`DlcPoolAssets.cs` 的构建数据 |
| `retired-spell-strip.json` | `RetiredSpellStrip.cs` 生成的构建报告；不删除旧法术行清单 |
| `reward-art/extra-icons.txt`、`rare-blue-generation.txt`、`rarity-generation.txt` | 绘图生成说明，运行时使用已生成的图像 |

## 明确保留

- 12 张主题图：`src/themes.rs` 动态指定图名，`native/src/scenery.rs` 读取。包括名称为 loading1/2/3 的图，仍用于主题幕。
- 职业／誓约／奖励卡、地图图像：对应界面的纹理加载器实际读取。
- 两套律法图标：线条图是主显示，插画图仍是已实现的回退路径。
- 三份字体及两份字表：`ui-sans-glyphs.txt` 通过 `with_file_name` 动态定位，用于字体缺字回退，不能删除。
- `reward-details.tsv`、`enemy-pools.txt`、`battlefields.txt`：启动脚本仍检查或读取。
- 地图、事件、原生 UI、特效、参数包：由游戏按资源约定加载，仅凭 Rust 字符串检索不足以判定闲置。

测试核对排除名单对应源码存在、编译嵌入引用未变成外部文件读取、启动脚本不依赖这些表，以及上述已用资源继续保留。未来添加新的运行时读取时，应同步移出排除名单。

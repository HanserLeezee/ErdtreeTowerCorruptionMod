# 主菜单与加载页主题

当前部署选择：标题主菜单已按用户要求恢复原版 `05_000_title.gfx` 和 `05_001_title_logo.gfx`。加载页继续使用自定义插画及公共贴图追加资源。下述主菜单主题构建方式仅为保留方案，重新生成时不要覆盖这两个原版文件。

主菜单与三张加载页使用生成的手绘插画（杀戮尖塔 2 式的粗墨线厚涂风格，画面里是法环的怪物），源图与提示词在 `art-source/theme/`：

| 文件 | 内容 |
|---|---|
| `title.jpg` | 主菜单：黄金树缠绕的高塔，树守、坩埚骑士、黄金树化身、小恶魔与翼人守在塔上，褪色者站在台阶下；左侧留暗放原版菜单 |
| `loading1.jpg` | 第一幕：废墟庭院，褪色者举盾对抗凶兆战士，葛瑞克士兵、巨蟹、小恶魔 |
| `loading2.jpg` | 第二幕：月下结晶湖，辉石龙、结晶人、辉石魔法师、湖中龙虾 |
| `loading3.jpg` | 第三幕：燃烧的王城，火焰巨人、黄金树化身、王城骑士、石像鬼 |

插画不是原作素材，是按提示词生成的原创图；没有使用《杀戮尖塔》的素材。

## 构建

```powershell
python scene-builder/title_theme.py --original <原版菜单目录> --output <新的输出目录> --ffdec <ffdec.jar> --java <java> --art art-source/theme
dotnet run --project scene-builder/SceneBuilder.csproj -- <游戏目录> <同一输出目录> <原版资源缓存目录> title-texture-patch
```

- `--art`：插画按 1920×1080 输出 BGRA8 DDS，GFx 使用原版的 DefineExternalImage2 引用。每张图仍有位图填充的全屏矩形，放在时间轴第 1 帧并命名为 `TowerThemeArt*`（主菜单放在 `Timeline_1` 里、以屏幕中心为原点；加载页放在主时间轴）。
- 主菜单的背景固定在深度 1，原有层深整体加 1，菜单与按键提示的位置直接写入时间轴；原版 ActionScript 保留。标题、标语和暗色面板绘入背景，菜单按钮仍是原生可操作控件。这样主菜单不依赖自定义 ENTER_FRAME 回调或动态字体。游戏曾只显示背景、没有文字，故不能仅凭脚本反编译成功判定显示正确。
- `title-texture-patch`：从游戏提取高、低画质两套 `01_common.tpf.dcx` 与 `01_common.sblytbnd.dcx`，分别追加四张 `SB_TowerTheme_*` 贴图及 `MENU_TowerTheme_*` 索引。写回后重读校验，原有贴图和索引内容不变。部署输出中的整个 `menu/`，不能只复制四个 GFx；`theme-textures/` 是构建中间文件，不部署。
- 旧版把 JPEG 直接嵌入 GFx，实测出现主菜单前整屏绿色。修复候选版改走原生纹理索引，仍需游戏实测。现有 JPEG 主题可用 `python scene-builder/title_textures.py --source <旧主题menu目录> --output <新的输出目录> --art art-source/theme` 转换，再运行 `title-texture-patch`；转换保留脚本、时间轴与图片编号。
- 加载页以独立原生 MovieClip 时间轴顺序轮换三幕插画，每张 240 帧（30 FPS 下约 8 秒）；不再依赖 ENTER_FRAME 的随机选择或动态文字。底部半透明提示条和每张对应的提示绘入图片。背景位于原生进度条／加载标识下面，旧背景被新背景覆盖，旧道具图鉴移到画面外；原生控件及其动画、脚本保留。主菜单左侧是半透明面板与标题文字，原版菜单按钮挪到面板里。
- `--rebase`：只适用于尚未嵌图的旧矢量主题。已带图片的主题必须从原版菜单重建，避免重复添加同名背景。不带 `--art` 时仍是首版的几何矢量插画。
- 校验：脚本导入后只有 DoABC 改变、无关脚本反编译结果不变；嵌图后只多出图片与形状定义、目标时间轴第 1 帧只多出摆放指令（`gfx_bitmap.verify`，测试见 `tests/test_gfx_bitmap.py`）。
- 修改 `scene-builder/title_theme.json` 可改标题、副标题、标语、配色和提示；配置是构建输入，修改后要重新生成。

`previews/` 里的 PNG 是设计预览，主菜单按钮文案为示意，不是游戏截图。中文字体、原生按钮位置、纹理加载、两种加载页层级和不同分辨率仍待实测。回退时恢复原来的四个 GFx 和两套 TPF／索引文件；若安装前没有覆盖文件，可移走这些新增覆盖文件，让游戏读取原版。

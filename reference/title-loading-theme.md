# 主菜单与加载页主题

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
python scene-builder/title_theme.py --original scene-assets/menu --output <新的输出目录> --ffdec <ffdec.jar> --java <java> --art art-source/theme --rebase
```

- `--art`：插画按 1920×1080 转成 JPEG，用 `scene-builder/gfx_bitmap.py` 嵌进菜单文件：每张图一个 DefineBitsJPEG2、一个位图填充的全屏矩形，放在时间轴第 1 帧并命名为 `TowerThemeArt*`（主菜单放在 `Timeline_1` 里、以屏幕中心为原点；加载页放在主时间轴）。FFDec 的脚本导入只能替换已有类，所以图片放在时间轴上，由脚本 `getChildByName` 找到后压到最底层，原版按钮与加载进度都在上面。
- 加载页每次打开随机显示三幕插画中的一张，底部是半透明提示条（标题、轮换提示，30 FPS 帧计时每 8 秒切换）；原版的背景图与道具图鉴隐藏。主菜单左侧是半透明面板与标题文字，原版菜单按钮挪到面板里。
- `--rebase`：`--original` 是已经打过这个补丁的菜单（例如仓库里的 `scene-assets/menu`），先去掉上一次的主题代码再写入。不带 `--art` 时仍是首版的几何矢量插画（需要原版菜单）。
- 校验：脚本导入后只有 DoABC 改变、无关脚本反编译结果不变；嵌图后只多出图片与形状定义、目标时间轴第 1 帧只多出摆放指令（`gfx_bitmap.verify`，测试见 `tests/test_gfx_bitmap.py`）。
- 修改 `scene-builder/title_theme.json` 可改标题、副标题、标语、配色和提示；配置是构建输入，修改后要重新生成。

`previews/` 里的 PNG 是设计预览（插画加 Pillow 画的面板与文字），主菜单按钮文案为示意，不是游戏截图。中文字体、原生按钮位置、嵌入位图在游戏里的显示、两种加载页层级和不同分辨率仍待实测。移除 `scene-assets/menu` 里这四个文件可恢复原版。

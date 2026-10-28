# 仅更新启动器

启动器版本在 `launcher/Tower.Launcher/Tower.Launcher.csproj` 中递增。Mod 显示版本保持不变，发布编号仍须大于最新发布。沿用同一更新通道，旧启动器仍能读清单并升级。

```powershell
python distribution/package_launcher.py --previous dist/release-v96/tower-release.json --build-number 97 --output dist/launcher-release-v97 --notes release-notes.md
```

`--previous` 必须是最新完整发布清单（包括此前的仅启动器更新）。命令只构建启动器，不构建 Rust Mod；从旧清单保留所有 Mod 文件及其历史数据位置，仅写入启动器变化和新的 SHA256SUMS。旧启动器文件以新构建的完整启动器文件列表为准。

生成 `tower-release.json`、`tower-pack-v97.bin` 和 `mirror/releases/`，不自动上传。将前两者作为新的 GitHub Release `v97` 附件，同步 mirror 下版本目录到 OSS，校验后再更新 OSS latest 清单并将 GitHub Release 设为最新。历史发布及数据包必须保留。新用户仍可安装上一次完整 ZIP 后更新；不要把增量数据包当作可直接解压运行的整合包。

不能只上传一个仅列启动器文件的 tower-release.json：老更新器会删除遗漏的 Mod 文件。此命令自动合并完整清单，防止这种误操作。

只改 UI 时会自动刷新页面；程序或依赖更新后提示重启。启动器设置显示当前运行版本及待重启生效的版本。更新器先记录回滚步骤再替换文件，安装记录提交后即使中断也不会回滚已完成的更新。

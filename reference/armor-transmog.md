# 防具幻化

在旅程档案的“防具幻化”页选择头、胸、手、腿，再选防具外观。支持名称搜索、分页、隐藏部位与恢复实际装备外观，默认全部跟随实际装备。记录独立保存在当前角色的 `.appearance` 文件，不修改防具物品或装备栏，原装备重量、防御和词条不变。

复用 Transmogrify Armor 2.4.14 的视觉参数别名与 Steam 外观同步。核心组件 `tower_transmog.dll` 由爬塔 DLL 自动载入，不接管赐福菜单、不占用 F8、不安装原项目的姿态／受击体积、仿身复制与留言复制钩子。同版联机玩家可以交换外观状态；未安装组件的玩家不能保证看到相同外观。

源码及修改记录：`native/vendor/tower-transmog/UPSTREAM.md`；发布包许可：`licenses/armor-transmog.txt`。

`distribution/native_build.py` 自动构建两个 DLL，`distribution/package.py` 同时打包。CMake 与 Steamworks 1.59 开发依赖缓存在工程上级 work/tower-transmog-build，可用 TOWER_CMAKE、TOWER_STEAMWORKS_SDK 指定已有路径。Steam DLL 不随包重新分发。

初始化失败时记录到 tower-preview.log / tower-transmog.log，幻化页拒绝应用，其他爬塔功能不依赖幻化初始化成功。真实换装、重载角色、隐藏部位、DLC 变身及双客户端外观仍需实机验证。

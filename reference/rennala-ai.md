# 满月女王战斗 AI

原版 `m14_00_00_00` 的二阶段实体 `c2031_9000`（14000800）使用模型 c2031、NPC 20310024、Think 20310000。该 Think 行的 logicId 和 battleGoalID 均为 203100。

原接入错误地使用了 NPC 20310124、Think 20310100；后者的 logicId=10000、battleGoalID=11000，是通用逻辑。模型一致及实体绑定成功不能证明战斗 AI 正确；单人测试也复现了不行动。

修正同时覆盖 boss-pool、两个克隆 NPC 参数（950000282/950000283）、party-extra-slots 与 7 处地图实体，并刷新竞技场资源指纹。`rennala-patch` 仅重建这一 Boss 的两个参数；随后按既有流程应用 tower-scaling-patch。参数对比确认只有这两条 NPC 行发生变化。追加构建时检查其战斗逻辑与目标 ID，避免再次接受通用 AI。

调试无敌改为本机玩家的 ChrDebugFlags.disabled_hit，保留 player_no_dead；不再启用全局 all_no_hit。与斗蛐蛐的玩家保护统一管理，关闭及角色重载时恢复原状态。实际 AI 行动和攻击命中仍需游戏验证。

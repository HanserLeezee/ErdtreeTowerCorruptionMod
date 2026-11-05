# 队伍延迟显示

路线图右上角的队伍列表显示“至房主延迟（RTT）”。房主行显示“房主”，其他行显示各自到房主的 Steam 往返延迟；客机看到的另一位队员的数值不是该队员到自己的延迟。

房主每 2 秒调用原始 SteamNetworkingMessages002 的 GetSessionConnectionInfo，读取 SteamNetConnectionRealTimeStatus_t.m_nPing。最近 4 次有效值取整数平均，不发额外探测包。状态必须为 Connected，负值和超过 60000ms 的异常值不计入平均。10 秒没有有效数据/队员汇报时显示“暂无响应”，恢复时重新计算。没有接口时显示“暂不支持”；手工 TCP 验证通道不显示这项 Steam 指标。

数据通过独立 Latency 消息传给当前队伍，客机只接受当前 session 的房主消息，校验完整名单、数值和递增序号。测量或广播失败不改变战斗、投票、断线和奖励状态。成员进入/退出时由现有队伍生命周期重建监测，离队清空显示。该状态不写存档。

延迟测试补丁协议为 63；合并 3e5cb4c 的新规则后开发版协议升为 64，联机各方都要更新。Steam API 的 ABI 依据本地 Steamworks SDK include/steam/isteamnetworkingmessages.h 与 steamnetworkingtypes.h：状态结构 120 字节、8 字节对齐，ping 偏移4、queue_time偏移48、reserved偏移56。已核对本机 steam_api64.dll 包含对应导出。

自动测试覆盖平均窗口、超时与恢复、缺值与合法0ms、名单和角色校验、协议往返、发送者/会话/序号校验、显示文案以及 ABI 布局。实际多人显示和真实网络数值仍需更新后验证，不能将 RTT 当成地图资源加载耗时。

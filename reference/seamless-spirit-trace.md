# Seamless 2.0.2 骨灰接口兼容（2026-11-02）

## 根因

ME3 和 YAFSML 均复现：无缝先于爬塔加载后，游戏 RVA `0x4B73D0`（使用状态）及 `0x4B8690`（召唤请求）的原始开头被 Seamless 跳转覆盖。原来 `items::checked_function` 的严格原版签名检查在第一个入口失败，导致追踪和双召唤保护没有初始化。

只读实机检查的 ersc.dll SHA-256：`0a093d8490967bf19c62f895fdf1fda21840282d2b2b371b9b4fc24a0cfe146e`。PE timestamp `0x6AE0AD1A`，image size `0xD65000`。

观察到的跳转链：

- 使用状态：游戏 E9 → `mov rax, thunk; jmp rax` → 携带 context 的 XMM5 thunk → ersc RVA `0x96ED0`。
- 召唤请求：游戏 E9 → `jmp [rip+0]` relay → 携带 context 的 XMM5 thunk → ersc RVA `0x96480`。
- 替换旧召唤组的调用点 `0x4B8A90` 和函数 `0x4BBA80` 未被改写，继续严格校验原版字节。

## 修复

`native/src/spirits/seamless_hook.rs` 只用于这两个入口，不放宽共享物品接口检查。允许原版签名；否则必须同时符合已验证的 2.0.2 PE 身份、原始尾部字节、有限跳转链、可读且对齐的 context、指定 ersc dispatcher 地址及其函数指纹。循环、不可读、陌生目标、修改的函数及其他版本均拒绝。

Dispatcher 指纹为 FNV-1a 64：`0x96ED0` 长 `0x63`、`0x6407C0604523C26A`；`0x96480` 长 `0x65`、`0x2A0354428EC85F58`。此值依据上述已核实二进制生成，并非通用支持其他无缝版本。

MinHook 在已有 E9 上创建 trampoline，会保留原有跳转目的地。新的调用顺序为爬塔包装函数 → trampoline → 原 Seamless 跳转链；不跳过或改写 Seamless 的 dispatcher。两个入口均预检成功后才创建追踪挂钩，启用请求挂钩失败时撤销状态挂钩，只有追踪完整安装后才启用双召唤保护。

## 验证

- 12 项骨灰模块顶层测试通过。其中一项在隔离子进程实际运行 MinHook 链：已有回调返回 `value+7`，叠加包装后返回 `value+10`，撤销后恢复 `value+7`。子进程测试也通过。
- 5 项返还相关测试通过；第一份骨灰记录不会被第二次召唤覆盖，背包恢复确认逻辑不变。
- 修复 DLL SHA-256：`7f92d0c6b116a3a40110cbe9fef4e4f45b5c1b7c95e122267270815079c28cf4`。
- 安装前备份了正式目录的旧 DLL；2026-11-02 07:20 UTC 在 YAFSML + Seamless 2.0.2 进程 20260 的日志中确认两条兼容链、使用状态检查、召唤请求追踪及双召唤保护均安装成功，没有出现旧的追踪跳过提示。
- 真正的两组骨灰共存及双机行为需实战反馈。普通胜利返还是独立的背包／召唤物快照逻辑，本次不宣称解决了所有骨灰返还问题。

后续实机日志已确认两次召唤均接受，第一组被保护，并按旧规则返还了第一份骨灰。用户随后要求两组独立判断存活并返还，另见 [分组返还规则](spirit-group-refund.md)。

实机审计及构建备份位于工作目录 `work/spirit-hook-audit.json`、`work/spirit-coop-fix/`。没有修改游戏存档、游戏文件或 Seamless DLL，也没有发布新版本。

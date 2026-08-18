# 迁移计划（分阶段）

> 状态图例: ⬜ 未开始 · 🟨 进行中 · ✅ 完成

## 阶段 0 — 基础设施 (当前)

| 任务 | 状态 | 说明 |
|---|---|---|
| 工程骨架: Rust workspace + Godot 项目 | ✅ | `server-rust/` + `client-godot/` |
| 协议 ID 枚举移植 (432 个包 ID) | ✅ | `scripts/gen_ids.sh` 从 Enums.cs 生成 |
| `binary.rs`: C# BinaryReader/Writer 兼容层 | ✅ | 7-bit 字符串、DateTime.ToBinary、ARGB |
| `frame.rs`: 帧编解码 + gzip | ✅ | 回环测试校验（含长度字段=4+载荷长的修正） |
| 核心握手/登录包移植 | ✅ | Connected/ClientVersion/Login/NewAccount… |
| Rust 服务器骨架 (tokio + 状态机) | ✅ | 登录握手垂直切片（demo_client 端到端通过） |
| Godot 客户端骨架 (编解码 + 登录 UI) | ✅ | `client-godot/` 未运行验证（本机无 Godot） |

## 阶段 1 — 协议全量移植

| 任务 | 状态 | 说明 |
|---|---|---|
| 全部 432 个数据包 → Rust (`crates/protocol`) | ✅ | 客户端 153/153、服务器 279/279（DellMember/ObjectNpc/WorldMapSetup 为命名差异，均已实现） |
| 内嵌数据类型: UserItem / ClientMapInfo / QuestInfo / UserInfo… | ✅ | ItemInfo/ClientQuestInfo/ClientMail/ClientMonsterInfo/RankCharacterInfo 等全部移植 |
| 回环序列化测试覆盖全部数据包 | ✅ | 151 项测试全通过（含 gzip、嵌套 UserItem、可空分支） |
| 协议移植到 Godot (GDScript 编解码) | 🟨 | binary/packets/client 三层已完成，包的完整编解码待与 Rust 侧对齐（主要包已覆盖） |

> 关键修复: UserInformation/UserSlotsRefresh 的槽布尔方向（C# 实际是 true=有物品）。

## 阶段 2 — 服务器核心玩法垂直切片

| 任务 | 状态 | 说明 |
|---|---|---|
| 账户/角色/存档 (SQLite) | ⬜ | 替换 C# 文件存档 |
| 进入世界: MapInformation/UserInformation/ObjectPlayer | ⬜ | |
| 移动 (Walk/Run/Turn)、广播同步 | ⬜ | |
| 聊天 (Chat/ObjectChat) | ⬜ | |
| 物品系统 (背包/装备/拾取/丢弃) | ⬜ | UserItem 内嵌序列化 |
| 战斗系统 (Attack/Magic/Struck/Damage) | ⬜ | |
| 怪物 AI + 刷新 (MonsterObject) | ⬜ | |
| NPC + 商店 + 任务 | ⬜ | |
| 公会/组队/交易/邮件/市场 | ⬜ | |
| 地图: 障碍/传送门/刷怪点 (客户端渲染需 Map txt/wil) | ⬜ | |

## 阶段 3 — Web3 钱包登录

| 任务 | 状态 | 说明 |
|---|---|---|
| EVM 钱包登录 (MetaMask/WalletConnect) | ⬜ | Godot 客户端嵌入 Web3 登录 |
| 服务器端签名验证 (secp256k1, eip-191/712) | ⬜ | Rust `k256`/`alloy` |
| 账户绑定: 钱包地址 ↔ 游戏账号 | ⬜ | SQLite 存储 + 防滥用 |
| 会话 token + 双因素(可选) | ⬜ | |

## 阶段 4 — 资产上链（后续迭代）

装备/宠物 NFT 化 (ERC-721)、游戏币 ERC-20、市场订单上链（先做链下订单簿，可选上链结算）。此阶段需重新确认链选型（默认 EVM 系: Polygon/Base）。

## 阶段 5 — 收尾

- 性能压测 (单机万人在线目标)、负载均衡(多世界分片)
- 反作弊/防外挂 (服务端校验所有移动/战斗)
- 迁移原 C# 中未覆盖的扩展系统（英雄、灵兽、跨服、攻城战）
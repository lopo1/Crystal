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
| 账户/角色/存档 (SQLite) | ✅ | characters 表存金币/经验/等级/位置/血量；断线+主动登出+周期自动存档 |
| 进入世界: MapInformation/UserInformation/ObjectPlayer | ✅ | 含固定槽位背包/装备下发 |
| 移动 (Walk/Run/Turn)、广播同步 | ✅ | 边界校验 + ObjectWalk/Run/Turn + UserLocation |
| 聊天 (Chat/ObjectChat) | ✅ | 广播给全场 |
| 物品系统 (背包/装备/拾取/丢弃) | ✅ | 拾取/购买/出售/道具使用(回血)/装备穿戴/丢弃/堆叠合并 + **背包整理(MoveItem/拆分SplitItem/合并MergeItem/EquipSlotItem)** + **仓库(Store/TakeBack)** + **金币丢弃/拾取(DropGold)** + **装备耐久/修理** + **回购(BuyItemBack: 记录最近出售→NPC 回购)** 全打通 |
| 战斗系统 (Attack/Magic/Struck/Damage) | ✅ | 近战平A + 魔法攻击(火球/雷电, 射程指向, 耗蓝/伤害/击杀) + 受击/死亡/经验/升级全打通 |
| 怪物 AI + 刷新 (MonsterObject) | ✅ | 刷新/感知索敌/追击(贪心寻路+ObjectWalk)/邻格攻击/脱战/死亡掉落全打通 |
| NPC + 商店 + 任务 | ✅ | 商人 CallNPC→NPCGoods→Buy/Sell + 任务系统(NPC对话接任务/击杀进度/完成领奖) 全打通 |
| 协作系统 (组队/公会/交易/市场/邮件) | ✅ | 组队(建队/邀请/接受/离队/解散) + 公会(建/加/离/解散) + 交易(物品+金币) + 市场(挂单/购买/撤单) + 站内邮件(带金币/物品附件) |
| 地图: 障碍/传送门/刷怪点 | ✅ | 已接入真实 .map（V100 + wemade2010 双格式）碰撞；多图注册表 + /map 传送 + 走格传送门 + **每张图独立怪物/NPC/掉落（map_index 隔离）** + 刷怪点配置数据化(spawn_config) 全打通 |
| 查看/社交/信息接口 | ✅ | Inspect/Observe 查看他人装备等级(PlayerInspect) + 好友系统(AddFriend/RemoveFriend/RefreshFriends + /friends, DB 持久化+在线状态) + 信息请求(RequestMapInfo/RequestItemInfo/RequestUserName/RequestNPCInfo) + TownRevive 回城复活 |
| 新手便利 (自动喝药/信息/导航) | ✅ | **自动喝药(SetAutoPotItem/SetAutoPotValue→血量阈值自动用消耗品)** + 信息请求(RequestMonsterInfo/RequestGuildInfo/SearchMap) + **TeleportToNPC 传送** + **BuyItemBack 回购** |

## 阶段 3 — Web3 钱包登录

| 任务 | 状态 | 说明 |
|---|---|---|
| EVM 钱包登录 | 🟨 | Rust 协议侧全通 + web3_client 端到端；Godot 内嵌可视化待做 |
| 服务器端签名验证 (secp256k1, eip-191) | ✅ | k256 恢复地址 + 挑战一次性/过期/篡改/错签名校验 + 单测覆盖 |
| 账户绑定: 钱包地址 ↔ 游戏账号 | ✅ | 地址即账号，首次登录自动注册（SQLite） |
| 会话 token（免签名重连） | ✅ | 登录后签发一次性 token，TTL 内 token 重连；双因素可选未做 |

## 阶段 4 — 资产上链（后续迭代）

装备/宠物 NFT 化 (ERC-721)、游戏币 ERC-20、市场订单上链（先做链下订单簿，可选上链结算）。此阶段需重新确认链选型（默认 EVM 系: Polygon/Base）。

## 阶段 5 — 收尾

- 性能压测 (单机万人在线目标)、负载均衡(多世界分片)
- 反作弊/防外挂 (服务端校验所有移动/战斗)
- 迁移原 C# 中未覆盖的扩展系统（英雄、灵兽、跨服、攻城战）
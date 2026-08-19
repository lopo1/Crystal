# Crystal Web3 MMORPG —— Rust 服务器 + Godot 客户端

原 Legend of Mir 2 (Crystal C#) 的 Web3 化改造。本目录为新增代码，原 C# 代码保留作协议/逻辑参考。

## 目录结构

```
server-rust/                 Rust 服务器 (workspace)
├── crates/
│   ├── protocol/            Crystal 协议层移植 (对应原 Shared/)
│   │   ├── src/binary.rs    C# BinaryReader/Writer 兼容 (7-bit 字符串/DateTime/ARGB)
│   │   ├── src/frame.rs     帧编解码 + gzip
│   │   ├── src/ids.rs       数据包 ID (由 scripts/gen_ids.sh 生成)
│   │   ├── src/types.rs     内嵌数据类型 (UserItem/SelectInfo/ClientMapInfo…)
│   │   ├── src/client/      客户端→服务器包
│   │   └── src/server/      服务器→客户端包
│   └── server/              tokio TCP 服务器 (登录/角色/世界)
│       ├── src/net.rs       帧解析 + 握手状态机
│       ├── src/account.rs   账户/角色 (内存版，后续换 SQLite)
│       ├── src/world.rs     世界/玩家/广播
│       └── examples/demo_client.rs  端到端测试客户端
client-godot/                Godot 4 客户端
├── project.godot
├── scenes/main.tscn         登录 → 角色 → 游戏界面
└── scripts/
    ├── net/crystal_binary.gd   二进制编解码 (与 Rust 一致)
    ├── net/crystal_packets.gd  包定义 + 解码
    ├── net/game_client.gd      TCP 连接/分发
    └── main.gd                 主流程
docs/                        迁移文档 (ARCHITECTURE / MIGRATION_PLAN / PROTOCOL)
scripts/gen_ids.sh            从 Shared/Enums.cs 重新生成包 ID
```

## 快速开始

```bash
# 0. 获取地图数据（真实碰撞网格，来自 Suprcode/Crystal.Database 的 Jev/Maps/*.map）
#    会自动克隆仓库并复制 0.map 到 server-rust/data/maps/；也可 MAP_SRC=本地仓库路径 复用
./scripts/get_maps.sh

# 1. 启动 Rust 服务器
cd server-rust && cargo run -p crystal-server
# 监听 127.0.0.1:7000，内置测试账号 demo（密码不校验）
# 启动日志会显示"地图 0 加载成功 ...x... 可通行 ..."，加载真实地图碰撞

# 2. 端到端自测（无需客户端）
cargo run -p crystal-server --example demo_client

# 3. Godot 客户端（需 Godot 4.3+）
# 用 Godot 打开 client-godot/ 目录，运行主场景
# 账号输入 demo → 连接并登录 → 选中角色 → 进入游戏
# 方向键移动，回车聊天
```

## 协议兼容性

所有字节格式与原始 Crystal (C#) 完全一致（见 `docs/PROTOCOL.md`）：
- 帧: `[u16 LE 总长][i16 LE 包ID][载荷]`，总长 = 4 + 载荷长
- 序列化: 小端序；字符串 7-bit 长度前缀 + UTF-8；gzip (仅 NPCGoods 包)
- 已移植: 432 个包 ID 全部生成；约 40 个核心包已完成 Rust/GDScript 双端移植

## 已跑通 (垂直切片)

连接握手 → 版本校验 → 登录 → 角色列表 → 创建/删除角色 → 进入世界
→ 地图信息 → 玩家信息 → 移动(Walk/Run/Turn) → 聊天(广播) → 登出
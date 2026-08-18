# Web3 MMORPG 目标架构

> Legend of Mir 2 (Crystal C# 源码) → Rust 服务器 + Godot 4 客户端 + Web3 钱包登录

## 总体结构

```
┌────────────────────┐    TCP    ┌─────────────────────────────┐
│  Godot 4 客户端     │◄─────────►│  Rust 服务器 (server-rust)   │
│  (client-godot)    │  Crystal  │  ├─ network   (tokio/TCP)   │
│  ├─ net/ 协议编解码 │  协议帧    │  ├─ protocol (Shared 移植)  │
│  ├─ ui/  登录/游戏UI│           │  ├─ auth     (钱包签名验证)  │
│  └─ world/ 渲染玩法 │           │  ├─ world    (地图/对象/战斗)│
└────────────────────┘           │  └─ db       (SQLite 存档)  │
                                 └─────────────────────────────┘
                                        │      │
                                        ▼      ▼
                              ┌──────────────────────┐
                              │  Web3 (后续阶段)      │
                              │  EVM 钱包登录/签名验证  │
                              │  (第一版仅钱包登录)    │
                              └──────────────────────┘
```

## 关键决策

| 决策点 | 选择 | 理由 |
|---|---|---|
| 客户端引擎 | Godot 4 | 免费开源、轻量、2D 表现力强、无授权费 |
| 服务端语言 | Rust (tokio) | 内存安全、高并发、无 GC 停顿 |
| 存档 | SQLite (第一阶段) | 原 C# 用本地文件存档，SQLite 可靠且零运维 |
| Web3 范围 | 第一版仅钱包登录 | 资产 NFT 化/游戏币 token 化留到后阶段 |
| 协议 | 完全兼容 Crystal 帧格式 | 两端可独立演进，契约不变 |

## 协议契约（不可破坏）

- 帧格式: `[u16 LE 总长][i16 LE 包ID][载荷]`，总长 = 2 + 载荷长
- 载荷: BinaryReader/BinaryWriter 风格小端序；部分包可 gzip 压缩
- 完整兼容规则见 `docs/PROTOCOL.md`

## 与旧代码的关系

- `Server/`, `Client/`, `Shared/` 等原 C# 目录**保留为协议与逻辑参考**，不做修改
- 新代码全部在 `server-rust/`(Rust) 与 `client-godot/`(Godot) 中
- 每移植一个系统，就在文档 `docs/MIGRATION_PLAN.md` 中勾选
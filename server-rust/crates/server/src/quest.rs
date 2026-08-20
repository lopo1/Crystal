//! 任务系统（阶段2）—— NPC 对话接任务 → 击杀进度 → 完成领奖。
//!
//! 任务通过《聊天命令 + 系统提示》交互（与交易/公会/市场等保持一致），
//! 由静态任务表驱动，进度持久化到 SQLite（`quest_progress` 表）。
//!
//! 交互约定：
//! - `CallNPC` 触碰任务 NPC → 系统提示任务描述/进度/是否可领奖
//! - `/quest_accept`  接受任务（按当前触碰的 NPC，取该 NPC 关联任务）
//! - `/quest_status`  查看所有进行中的任务及击杀进度
//! - `/quest_reward`  领取已完成任务的奖励
//! - `/quest_touch <NPC名>` 手动指定并“触碰”某个任务 NPC（调试/演示，无需走近）
//! - `/quest_forget <任务ID>` 放弃任务

use std::collections::HashMap;

use crate::db::Database;

/// 任务目标类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestObjective {
    /// 击杀指定怪物（image）指定数量
    Kill { monster_image: u16, count: u32 },
}

/// 一条任务定义（静态数据）
#[derive(Debug, Clone)]
pub struct QuestDef {
    pub id: u32,
    pub name: &'static str,
    /// 发布任务的 NPC 名字（可为空 = 任意任务 NPC 都可接）
    pub npc_name: &'static str,
    /// 任务描述
    pub description: &'static str,
    pub objective: QuestObjective,
    pub reward_gold: u32,
    pub reward_exp: u32,
    /// 奖励物品（模板索引，0 = 无）
    pub reward_item: i32,
    pub reward_item_count: u16,
}

/// 内置任务表
pub const QUESTS: &[QuestDef] = &[
    QuestDef {
        id: 1,
        name: "猎杀骷髅",
        npc_name: "任务管理员",
        description: "新手村附近有骷髅出没，消灭 5 只骷髅以示勇武。",
        objective: QuestObjective::Kill { monster_image: 3, count: 5 },
        reward_gold: 100,
        reward_exp: 60,
        reward_item: 3, // 金创药
        reward_item_count: 3,
    },
    QuestDef {
        id: 2,
        name: "除蜘蛛",
        npc_name: "任务管理员",
        description: "更深处盘踞着蜘蛛，剿灭 3 只蛛患。",
        objective: QuestObjective::Kill { monster_image: 4, count: 3 },
        reward_gold: 200,
        reward_exp: 150,
        reward_item: 0,
        reward_item_count: 0,
    },
];

/// 单个玩家的任务进度
#[derive(Debug, Clone, Default)]
pub struct QuestProgress {
    pub quest_id: u32,
    pub killed: u32,
    /// true = 已达目标，等待领取奖励
    pub completed: bool,
    /// true = 已领取奖励（任务结束，不再计数）
    pub finished: bool,
}

/// 任务管理器（每世界一份）
#[derive(Debug, Default)]
pub struct QuestManager {
    /// 角色名 -> 该角色当前触碰（对话）过的任务 NPC（用于 /quest_accept）
    touched_npc: HashMap<String, String>,
}

impl QuestManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// 记录某个角色当前正在对话的任务 NPC（由 CallNPC 触发）。
    pub fn touch(&mut self, player: &str, npc_name: &str) {
        self.touched_npc.insert(player.to_string(), npc_name.to_string());
    }

    /// 取某角色当前触碰的任务 NPC 名。
    pub fn touched_npc(&self, player: &str) -> Option<&str> {
        self.touched_npc.get(player).map(|s| s.as_str())
    }

    /// 根据角色触碰的 NPC，返回该 NPC 发布的“首个未完成/未领奖”任务。
    pub fn quest_for_touch(&self, player: &str, db: &Database, char_index: i32) -> Option<&'static QuestDef> {
        let npc = self.touched_npc(player)?;
        let prog = db.load_quest_progress(char_index).unwrap_or_default();
        QUESTS.iter().find(|q| {
            q.npc_name == npc && !prog.iter().any(|p| p.quest_id == q.id && p.completed)
        })
    }
}

// ---------------------------------------------------------------------------
// 辅助：击杀进度查询 / 领取校验（供 world 在 monster_died 调用）
// ---------------------------------------------------------------------------

/// 击杀一只怪物后登记进度。返回（是否对应任务、击杀后是否达成目标）。
pub fn register_kill(_player: &str, char_index: i32, monster_image: u16, db: &Database) -> (bool, u32) {
    // 查找以该怪物为目标、且玩家还未完成的进行中任务
    let Some(def) = QUESTS
        .iter()
        .find(|q| matches!(q.objective, QuestObjective::Kill { monster_image: img, .. } if img == monster_image))
    else {
        return (false, 0);
    };
    let mut progress = db.load_quest_progress(char_index).unwrap_or_default();
    let mut touched = false;
    for p in progress.iter_mut() {
        if p.quest_id == def.id {
            if !p.completed {
                p.killed = p.killed.saturating_add(1);
                touched = true;
            }
            break;
        }
    }
    if !touched {
        // 玩家还没接这个任务，不记录
        return (false, 0);
    }
    let target = match def.objective {
        QuestObjective::Kill { count, .. } => count,
    };
    // 达成目标 -> 标记可领奖（completed=true），但仍可继续击杀
    let done = progress
        .iter_mut()
        .find(|p| p.quest_id == def.id)
        .map(|p| {
            if p.killed >= target && !p.completed {
                p.completed = true;
            }
            p.completed
        })
        .unwrap_or(false);
    let _ = db.save_quest_progress(char_index, &progress);
    (true, done as u32)
}

/// 领取任务奖励。返回 Some(描述) 表示成功，Err 为失败原因。
pub fn reward(
    char_index: i32,
    quest_id: u32,
    db: &Database,
) -> Result<(QuestDef, u32, u32), String> {
    let Some(def) = QUESTS.iter().find(|q| q.id == quest_id) else {
        return Err("任务不存在".into());
    };
    let target = match def.objective {
        QuestObjective::Kill { count, .. } => count,
    };
    let mut progress = db.load_quest_progress(char_index).unwrap_or_default();
    let p = progress
        .iter_mut()
        .find(|p| p.quest_id == quest_id)
        .ok_or_else(|| "你还没有接受该任务".to_string())?;
    if p.finished {
        return Err("该任务奖励已领取".into());
    }
    if !p.completed {
        return Err(format!("任务未完成：{}/{}", p.killed, target));
    }
    // 标记为已领奖（completed 保持，finished 记录已领取）
    p.finished = true;
    let _ = db.save_quest_progress(char_index, &progress);
    db.set_quest_finished(char_index, quest_id)
        .map_err(|e| e.to_string())?;
    Ok((
        def.clone(),
        def.reward_gold,
        def.reward_exp,
    ))
}

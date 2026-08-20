//! SQLite 数据库（阶段 2）: 账户/角色/物品 持久化。
//!
//! 原 Crystal 用本地文件存档，这里用 SQLite 提供可靠且零运维的持久化。
//! 后续（阶段 3 Web3）将扩展钱包地址绑定。
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};

use crystal_protocol::types::{SelectInfo, UserItem};

/// 背包容量（与 world.rs 对齐）
pub const INVENTORY_SIZE: usize = 40;

/// 仓库容量（个人储物箱）
pub const STORAGE_SIZE: usize = 48;

/// 一封站内邮件
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mail {
    pub id: i64,
    pub from_name: String,
    pub title: String,
    pub body: String,
    pub gold: i64,
    pub item_uid: i64,
    pub is_read: bool,
    pub created_at: i64,
}

pub struct Database {
    conn: Mutex<Connection>,
    /// 物品静态数据库（由 items 表加载，供进入世界时填充背包）
    pub items: Arc<std::sync::RwLock<Vec<(i32, String)>>>,
}

impl Database {
    pub fn open(path: impl AsRef<Path>) -> anyhow::Result<Arc<Self>> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        let db = Arc::new(Database {
            conn: Mutex::new(conn),
            items: Arc::new(std::sync::RwLock::new(Vec::new())),
        });
        db.init_schema()?;
        db.seed_demo_items()?;
        Ok(db)
    }

    fn init_schema(&self) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS accounts (
                account_id TEXT PRIMARY KEY,
                pass_hash  TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS characters (
                character_index INTEGER PRIMARY KEY AUTOINCREMENT,
                account_id    TEXT NOT NULL,
                name          TEXT NOT NULL UNIQUE,
                class         INTEGER NOT NULL,
                gender        INTEGER NOT NULL,
                level         INTEGER NOT NULL DEFAULT 1,
                x             INTEGER NOT NULL DEFAULT 400,
                y             INTEGER NOT NULL DEFAULT 400,
                direction     INTEGER NOT NULL DEFAULT 0,
                hp            INTEGER NOT NULL DEFAULT 100,
                mp            INTEGER NOT NULL DEFAULT 100,
                gold          INTEGER NOT NULL DEFAULT 1000,
                experience    INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS items (
                item_index INTEGER PRIMARY KEY,
                name       TEXT NOT NULL,
                item_type  INTEGER NOT NULL DEFAULT 0,
                image      INTEGER NOT NULL DEFAULT 0,
                durability INTEGER NOT NULL DEFAULT 0,
                stack_size INTEGER NOT NULL DEFAULT 1,
                weight     INTEGER NOT NULL DEFAULT 0,
                price      INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS inventory (
                character_index INTEGER NOT NULL,
                slot            INTEGER NOT NULL,
                unique_id       INTEGER NOT NULL,
                item_index      INTEGER NOT NULL,
                count           INTEGER NOT NULL DEFAULT 1,
                current_dura    INTEGER NOT NULL DEFAULT 0,
                max_dura        INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (character_index, slot)
            );
            CREATE TABLE IF NOT EXISTS equipment (
                character_index INTEGER NOT NULL,
                slot            INTEGER NOT NULL,
                unique_id       INTEGER NOT NULL,
                item_index      INTEGER NOT NULL,
                current_dura    INTEGER NOT NULL DEFAULT 0,
                max_dura        INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (character_index, slot)
            );
            CREATE TABLE IF NOT EXISTS storage (
                character_index INTEGER NOT NULL,
                slot            INTEGER NOT NULL,
                unique_id       INTEGER NOT NULL,
                item_index      INTEGER NOT NULL,
                count           INTEGER NOT NULL DEFAULT 1,
                current_dura    INTEGER NOT NULL DEFAULT 0,
                max_dura        INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (character_index, slot)
            );

            CREATE TABLE IF NOT EXISTS mail (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                to_char     INTEGER NOT NULL,
                from_name   TEXT NOT NULL,
                title       TEXT NOT NULL,
                body        TEXT NOT NULL,
                gold        INTEGER NOT NULL DEFAULT 0,
                item_uid    INTEGER NOT NULL DEFAULT 0,
                is_read     INTEGER NOT NULL DEFAULT 0,
                created_at  INTEGER NOT NULL DEFAULT (strftime('%s','now'))
            );

            CREATE TABLE IF NOT EXISTS quest_progress (
                character_index INTEGER NOT NULL,
                quest_id        INTEGER NOT NULL,
                killed          INTEGER NOT NULL DEFAULT 0,
                completed       INTEGER NOT NULL DEFAULT 0,
                finished        INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (character_index, quest_id)
            );

            CREATE TABLE IF NOT EXISTS friends (
                character_index INTEGER NOT NULL,
                friend_index    INTEGER NOT NULL,
                memo            TEXT NOT NULL DEFAULT '',
                blocked         INTEGER NOT NULL DEFAULT 0,
                added_at        INTEGER NOT NULL DEFAULT (strftime('%s','now')),
                PRIMARY KEY (character_index, friend_index)
            );

            CREATE TABLE IF NOT EXISTS storage_pw (
                character_index INTEGER PRIMARY KEY,
                pw              TEXT NOT NULL,
                set_at          INTEGER NOT NULL DEFAULT 0
            );

            -- 师徒：玩家（mentee）至多有一个 mentor
            CREATE TABLE IF NOT EXISTS mentor_relations (
                character_index INTEGER PRIMARY KEY,
                mentor_index    INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS mentor_mentees (
                mentor_index INTEGER NOT NULL,
                mentee_index INTEGER NOT NULL,
                PRIMARY KEY (mentor_index, mentee_index)
            );

            -- 婚姻：一个角色至多一个配偶
            CREATE TABLE IF NOT EXISTS marriages (
                char_index    INTEGER PRIMARY KEY,
                spouse_index  INTEGER NOT NULL,
                date          INTEGER NOT NULL DEFAULT 0
            );
            "#,
        )?;
        // 旧库迁移：补 durability / refines / reincarnations 列（已存在则 ALTER 报错，忽略即可）
        for stmt in [
            "ALTER TABLE inventory ADD COLUMN current_dura INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE inventory ADD COLUMN max_dura INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE equipment ADD COLUMN current_dura INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE equipment ADD COLUMN max_dura INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE inventory ADD COLUMN refines INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE equipment ADD COLUMN refines INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE characters ADD COLUMN reincarnations INTEGER NOT NULL DEFAULT 0",
        ] {
            let _ = conn.execute(stmt, []);
        }
        drop(conn);
        Ok(())
    }

    /// 种子物品库（垂直切片演示用）
    fn seed_demo_items(&self) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM items", [], |r| r.get(0))?;
        if count == 0 {
            // 木剑(1) / 布衣(2) / 金创药(3) / 回城卷(4) / 铜币袋(5)
            let demo: [(i32, &str, i32, i32, i32, i32, i32, i64); 5] = [
                (1, "木剑", 1, 100, 20, 1, 3, 50),
                (2, "布衣", 1, 101, 25, 1, 4, 80),
                (3, "金创药", 2, 120, 0, 5, 1, 20),
                (4, "回城卷", 2, 121, 0, 5, 1, 60),
                (5, "铜币袋", 0, 130, 0, 20, 1, 100),
            ];
            for (idx, name, ty, img, dur, stack, w, price) in demo {
                conn.execute(
                    "INSERT INTO items (item_index,name,item_type,image,durability,stack_size,weight,price)
                     VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
                    params![idx, name, ty, img, dur, stack, w, price],
                )?;
            }
        }
        Ok(())
    }

    // ------------------------- 账户 -------------------------

    /// 返回是否注册成功（false=已存在）
    pub fn register(&self, account_id: &str) -> anyhow::Result<bool> {
        let conn = self.conn.lock().unwrap();
        let exists: i64 =
            conn.query_row("SELECT COUNT(*) FROM accounts WHERE account_id=?1", [account_id], |r| r.get(0))?;
        if exists > 0 {
            return Ok(false);
        }
        conn.execute(
            "INSERT INTO accounts (account_id, pass_hash) VALUES (?1, ?2)",
            params![account_id, hash_password(account_id)],
        )?;
        Ok(true)
    }

    /// 登录校验（阶段2 简化：账号存在即通过；阶段3 接钱包签名）
    pub fn login(&self, account_id: &str) -> anyhow::Result<bool> {
        let conn = self.conn.lock().unwrap();
        let exists: i64 =
            conn.query_row("SELECT COUNT(*) FROM accounts WHERE account_id=?1", [account_id], |r| r.get(0))?;
        Ok(exists > 0)
    }

    /// Web3 钱包登录：地址即账号。账户不存在则自动注册（首次签名即注册）。
    /// 返回角色列表（供进入选择界面）。
    pub fn web3_login(&self, wallet_address: &str) -> anyhow::Result<Vec<SelectInfo>> {
        if !self.login(wallet_address)? {
            self.register(wallet_address)?;
        }
        self.select_infos(wallet_address)
    }

    // ------------------------- 角色 -------------------------

    pub fn select_infos(&self, account_id: &str) -> anyhow::Result<Vec<SelectInfo>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT character_index, name, level, class, gender FROM characters WHERE account_id=?1",
        )?;
        let rows = stmt.query_map([account_id], |r| {
            Ok((r.get::<_, i32>(0)?, r.get::<_, String>(1)?, r.get::<_, u16>(2)?,
                r.get::<_, u8>(3)?, r.get::<_, u8>(4)?))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (index, name, level, class, gender) = row?;
            out.push(SelectInfo {
                index,
                name,
                level,
                class: num_to_class(class),
                gender: num_to_gender(gender),
                last_access: 0,
            });
        }
        Ok(out)
    }

    /// 返回 Some(info) 成功 / Err(code) 失败（2=性别错,3=职业错,4=角色满,5=重名）
    pub fn add_character(
        &self,
        account_id: &str,
        name: &str,
        class: crystal_protocol::types::MirClass,
        gender: crystal_protocol::types::MirGender,
    ) -> anyhow::Result<Result<SelectInfo, u8>> {
        let index;
        {
            let conn = self.conn.lock().unwrap();
            // 同名检查
            let dup: i64 = conn
                .query_row("SELECT COUNT(*) FROM characters WHERE name=?1", [name], |r| r.get(0))?;
            if dup > 0 {
                return Ok(Err(5));
            }
            // 角色数上限（C# 默认 4）
            let cnt: i64 = conn.query_row(
                "SELECT COUNT(*) FROM characters WHERE account_id=?1",
                [account_id],
                |r| r.get(0),
            )?;
            if cnt >= 4 {
                return Ok(Err(4));
            }
            conn.execute(
                "INSERT INTO characters (account_id,name,class,gender) VALUES (?1,?2,?3,?4)",
                params![account_id, name, class as u8, gender as u8],
            )?;
            index = conn.last_insert_rowid() as i32;
        } // 释放锁
        // 自动发初始背包（之后锁）
        self.grant_starter_items(index)?;
        Ok(Ok(SelectInfo {
            index,
            name: name.to_string(),
            level: 1,
            class,
            gender,
            last_access: 0,
        }))
    }

    pub fn delete_character(&self, account_id: &str, character_index: i32) -> anyhow::Result<bool> {
        let conn = self.conn.lock().unwrap();
        let n = conn.execute(
            "DELETE FROM characters WHERE character_index=?1 AND account_id=?2",
            params![character_index, account_id],
        )?;
        Ok(n > 0)
    }

    /// 返回 (角色行, 新账号时分配的 object_id 由上层处理)
    pub fn get_character(
        &self,
        account_id: &str,
        character_index: i32,
    ) -> anyhow::Result<Option<CharacterRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT character_index,name,class,gender,level,x,y,direction,hp,mp,gold,experience
             FROM characters WHERE character_index=?1 AND account_id=?2",
        )?;
        let mut rows = stmt.query_map(params![character_index, account_id], |r| {
            Ok(CharacterRow {
                index: r.get(0)?,
                name: r.get(1)?,
                class: r.get(2)?,
                gender: r.get(3)?,
                level: r.get(4)?,
                x: r.get(5)?,
                y: r.get(6)?,
                direction: r.get(7)?,
                hp: r.get(8)?,
                mp: r.get(9)?,
                gold: r.get(10)?,
                experience: r.get(11)?,
            })
        })?;
        Ok(rows.next().transpose()?)
    }

    /// 保存角色位置等状态
    /// 保存角色状态（位置/血量 + 金币/经验/等级），掉线与定期备份都会调用。
    pub fn save_character_state(
        &self,
        character_index: i32,
        x: i32,
        y: i32,
        direction: i32,
        hp: i32,
        mp: i32,
        gold: i64,
        experience: i64,
        level: i64,
    ) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE characters SET x=?1,y=?2,direction=?3,hp=?4,mp=?5,gold=?6,experience=?7,level=?8
             WHERE character_index=?9",
            params![x, y, direction, hp, mp, gold, experience, level, character_index],
        )?;
        Ok(())
    }

    // ------------------------- 物品 -------------------------

    /// 加载某角色的背包：[(slot, UserItem)]
    pub fn load_inventory(&self, character_index: i32) -> anyhow::Result<Vec<(i32, UserItem)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT slot,unique_id,item_index,count,current_dura,max_dura FROM inventory WHERE character_index=?1 ORDER BY slot")?;
        let rows = stmt.query_map([character_index], |r| {
            Ok((
                r.get::<_, i32>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, i32>(2)?,
                r.get::<_, u16>(3)?,
                r.get::<_, u16>(4)?,
                r.get::<_, u16>(5)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (slot, uid, item_index, count, current_dura, max_dura) = row?;
            out.push((
                slot,
                UserItem {
                    unique_id: uid as u64,
                    item_index,
                    count,
                    current_dura,
                    max_dura,
                    ..Default::default()
                },
            ));
        }
        Ok(out)
    }

    /// 新角色自动发初始背包（木剑 + 金创药x5）
    pub fn grant_starter_items(&self, character_index: i32) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        // 木剑: 满耐久；金创药: 无耐久
        let sword_dura = crate::items::find(1).map(|t| t.max_dura).unwrap_or(0);
        conn.execute(
            "INSERT OR IGNORE INTO inventory (character_index,slot,unique_id,item_index,count,current_dura,max_dura)
             VALUES (?1,0,?2,1,1,?3,?3)",
            params![character_index, make_unique(character_index, 0), sword_dura],
        )?;
        conn.execute(
            "INSERT OR IGNORE INTO inventory (character_index,slot,unique_id,item_index,count) VALUES (?1,1,?2,3,5)",
            params![character_index, make_unique(character_index, 1)],
        )?;
        Ok(())
    }

    /// 把物品加入背包首个空槽；满则返回 false。
    pub fn add_item_to_inventory(
        &self,
        character_index: i32,
        item_index: i32,
        count: u16,
    ) -> anyhow::Result<bool> {
        let conn = self.conn.lock().unwrap();
        // 若背包已有同类物品，则合并堆叠（不占新槽）
        let existing_uid: Option<i64> = conn
            .query_row(
                "SELECT unique_id FROM inventory WHERE character_index=?1 AND item_index=?2 LIMIT 1",
                params![character_index, item_index],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(uid) = existing_uid {
            conn.execute(
                "UPDATE inventory SET count=count+?1 WHERE unique_id=?2",
                params![count, uid],
            )?;
            return Ok(true);
        }
        // 找空槽（0..INVENTORY_SIZE）
        let occupied: Vec<i32> = {
            let mut stmt = conn.prepare(
                "SELECT slot FROM inventory WHERE character_index=?1 ORDER BY slot",
            )?;
            let rows = stmt.query_map([character_index], |r| r.get(0))?;
            rows.collect::<Result<Vec<i32>, _>>()?
        };
        for slot in 0..INVENTORY_SIZE as i32 {
            if !occupied.contains(&slot) {
                let max_dura = crate::items::find(item_index).map(|t| t.max_dura).unwrap_or(0);
                conn.execute(
                    "INSERT INTO inventory (character_index,slot,unique_id,item_index,count,current_dura,max_dura)
                     VALUES (?1,?2,?3,?4,?5,?6,?7)",
                    params![
                        character_index,
                        slot,
                        make_unique(character_index, slot),
                        item_index,
                        count,
                        max_dura,
                        max_dura
                    ],
                )?;
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// 交易/转移: 把某件背包物品的所有权从 from_char 转到 to_char。
    pub fn transfer_item(
        &self,
        from_char: i32,
        to_char: i32,
        unique_id: u64,
    ) -> anyhow::Result<bool> {
        let conn = self.conn.lock().unwrap();
        let exists: i64 = conn.query_row(
            "SELECT COUNT(*) FROM inventory WHERE character_index=?1 AND unique_id=?2",
            params![from_char, unique_id as i64],
            |r| r.get(0),
        )?;
        if exists == 0 {
            return Ok(false);
        }
        conn.execute(
            "UPDATE inventory SET character_index=?1 WHERE unique_id=?2",
            params![to_char, unique_id as i64],
        )?;
        Ok(true)
    }

    /// 发送一封站内邮件。返回邮件 id。
    pub fn send_mail(
        &self,
        to_char: i32,
        from_name: &str,
        title: &str,
        body: &str,
        gold: i64,
        item_uid: i64,
    ) -> anyhow::Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO mail (to_char,from_name,title,body,gold,item_uid) VALUES (?1,?2,?3,?4,?5,?6)",
            params![to_char, from_name, title, body, gold, item_uid],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// 某角色的收件箱（按时间倒序）。
    pub fn mail_inbox(&self, char_index: i32) -> anyhow::Result<Vec<Mail>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id,from_name,title,body,gold,item_uid,is_read,created_at FROM mail
             WHERE to_char=?1 ORDER BY created_at DESC, id DESC",
        )?;
        let rows = stmt.query_map([char_index], |r| {
            Ok(Mail {
                id: r.get(0)?,
                from_name: r.get(1)?,
                title: r.get(2)?,
                body: r.get(3)?,
                gold: r.get(4)?,
                item_uid: r.get(5)?,
                is_read: r.get::<_, i64>(6)? != 0,
                created_at: r.get(7)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// 标记已读
    pub fn mark_mail_read(&self, id: i64) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("UPDATE mail SET is_read=1 WHERE id=?1", params![id])?;
        Ok(())
    }

    /// 删除邮件
    pub fn delete_mail(&self, id: i64) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM mail WHERE id=?1", params![id])?;
        Ok(())
    }

    /// 取单封邮件（按 id 与收件人角色），无则 None。用于领取附件时校验归属。
    pub fn get_mail(&self, id: i64, to_char: i32) -> anyhow::Result<Option<Mail>> {
        let conn = self.conn.lock().unwrap();
        let row = conn
            .query_row(
                "SELECT id,from_name,title,body,gold,item_uid,is_read,created_at FROM mail
                 WHERE id=?1 AND to_char=?2",
                params![id, to_char],
                |r| {
                    Ok(Mail {
                        id: r.get(0)?,
                        from_name: r.get(1)?,
                        title: r.get(2)?,
                        body: r.get(3)?,
                        gold: r.get(4)?,
                        item_uid: r.get(5)?,
                        is_read: r.get::<_, i64>(6)? != 0,
                        created_at: r.get(7)?,
                    })
                },
            )
            .optional()?;
        Ok(row)
    }

    /// 领取金币附件：把 mail.gold 清零，返回领取的金币数。
    pub fn claim_mail_gold(&self, id: i64) -> anyhow::Result<i64> {
        let conn = self.conn.lock().unwrap();
        let gold: i64 =
            conn.query_row("SELECT gold FROM mail WHERE id=?1", params![id], |r| r.get(0))?;
        conn.execute(
            "UPDATE mail SET gold=0 WHERE id=?1",
            params![id],
        )?;
        Ok(gold)
    }

    /// 领取物品附件：把 mail.item_uid 清零，返回被领取物品的 unique_id（0 表示无）。
    pub fn claim_mail_item(&self, id: i64) -> anyhow::Result<i64> {
        let conn = self.conn.lock().unwrap();
        let item_uid: i64 =
            conn.query_row("SELECT item_uid FROM mail WHERE id=?1", params![id], |r| r.get(0))?;
        conn.execute(
            "UPDATE mail SET item_uid=0 WHERE id=?1",
            params![id],
        )?;
        Ok(item_uid)
    }

    /// 按角色名查 character_index（跨账号唯一）。无则 None。
    pub fn char_index_by_name(&self, name: &str) -> anyhow::Result<Option<i32>> {
        let conn = self.conn.lock().unwrap();
        let idx = conn
            .query_row(
                "SELECT character_index FROM characters WHERE name=?1",
                params![name],
                |r| r.get(0),
            )
            .optional()?;
        Ok(idx)
    }

    /// 直接给某角色加金币（离线圈地奖励/邮件投递时用），返回加金币前余额。
    pub fn add_char_gold(&self, character_index: i32, amount: i64) -> anyhow::Result<i64> {
        let conn = self.conn.lock().unwrap();
        let before: i64 = conn.query_row(
            "SELECT gold FROM characters WHERE character_index=?1",
            params![character_index],
            |r| r.get(0),
        )?;
        conn.execute(
            "UPDATE characters SET gold=gold+?1 WHERE character_index=?2",
            params![amount, character_index],
        )?;
        Ok(before)
    }

    // ------------------------- 任务 -------------------------

    /// 加载某角色全部任务进度。
    pub fn load_quest_progress(
        &self,
        character_index: i32,
    ) -> anyhow::Result<Vec<crate::quest::QuestProgress>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT quest_id,killed,completed,finished FROM quest_progress
             WHERE character_index=?1 ORDER BY quest_id",
        )?;
        let rows = stmt.query_map([character_index], |r| {
            Ok(crate::quest::QuestProgress {
                quest_id: r.get::<_, i64>(0)? as u32,
                killed: r.get::<_, i64>(1)? as u32,
                completed: r.get::<_, i64>(2)? != 0,
                finished: r.get::<_, i64>(3)? != 0,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// 保存（UPSERT）某角色任务进度。接受一个可选任务：Some 则只更新该条，None 则全量替换。
    pub fn save_quest_progress(
        &self,
        character_index: i32,
        progress: &[crate::quest::QuestProgress],
    ) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        for p in progress {
            conn.execute(
                "INSERT INTO quest_progress (character_index,quest_id,killed,completed,finished)
                 VALUES (?1,?2,?3,?4,?5)
                 ON CONFLICT(character_index,quest_id) DO UPDATE SET
                    killed=excluded.killed, completed=excluded.completed, finished=excluded.finished",
                params![
                    character_index,
                    p.quest_id as i64,
                    p.killed as i64,
                    p.completed as i64,
                    p.finished as i64
                ],
            )?;
        }
        Ok(())
    }

    /// 接受任务：创建一条进度（已存在则忽略，不清零之前的击杀）。
    pub fn accept_quest(&self, character_index: i32, quest_id: u32) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO quest_progress (character_index,quest_id,killed,completed,finished)
             VALUES (?1,?2,0,0,0)",
            params![character_index, quest_id as i64],
        )?;
        Ok(())
    }

    /// 标记任务已领奖。
    pub fn set_quest_finished(
        &self,
        character_index: i32,
        quest_id: u32,
    ) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE quest_progress SET finished=1 WHERE character_index=?1 AND quest_id=?2",
            params![character_index, quest_id as i64],
        )?;
        Ok(())
    }

    /// 放弃任务（删除进度）。
    pub fn forget_quest(&self, character_index: i32, quest_id: u32) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM quest_progress WHERE character_index=?1 AND quest_id=?2",
            params![character_index, quest_id as i64],
        )?;
        Ok(())
    }

    /// 按 unique_id 查找背包物品的模板索引；无则 None。
    pub fn inventory_item_index(
        &self,
        character_index: i32,
        unique_id: u64,
    ) -> anyhow::Result<Option<i32>> {
        let conn = self.conn.lock().unwrap();
        let item_index = conn
            .query_row(
                "SELECT item_index FROM inventory WHERE character_index=?1 AND unique_id=?2",
                params![character_index, unique_id as i64],
                |r| r.get(0),
            )
            .optional()?;
        Ok(item_index)
    }

    /// 从背包删除指定 unique_id 的物品（出售用），并返回其模板索引。
    pub fn remove_from_inventory(
        &self,
        character_index: i32,
        unique_id: u64,
    ) -> anyhow::Result<Option<i32>> {
        let conn = self.conn.lock().unwrap();
        let idx: Option<i32> = conn
            .query_row(
                "SELECT item_index FROM inventory WHERE character_index=?1 AND unique_id=?2",
                params![character_index, unique_id as i64],
                |r| r.get(0),
            )
            .optional()?;
        if idx.is_some() {
            conn.execute(
                "DELETE FROM inventory WHERE character_index=?1 AND unique_id=?2",
                params![character_index, unique_id as i64],
            )?;
        }
        Ok(idx)
    }

    /// 加载背包为固定 `INVENTORY_SIZE` 槽位向量（槽位索引即下标，空槽为 None），
    /// 与 C# `UserInformation.Inventory` / `UserSlotsRefresh` 的槽位语义一致。
    pub fn inventory_slots(&self, character_index: i32) -> anyhow::Result<Vec<Option<UserItem>>> {
        let rows = self.load_inventory(character_index)?;
        let mut slots: Vec<Option<UserItem>> =
            (0..INVENTORY_SIZE).map(|_| None).collect();
        for (slot, item) in rows {
            if let Some(slot) = usize::try_from(slot).ok() {
                if let Some(s) = slots.get_mut(slot) {
                    *s = Some(item);
                }
            }
        }
        Ok(slots)
    }

    /// 按 unique_id 查找背包物品，返回 (槽位, UserItem)。
    pub fn find_inventory_item(
        &self,
        character_index: i32,
        unique_id: u64,
    ) -> anyhow::Result<Option<(i32, UserItem)>> {
        let conn = self.conn.lock().unwrap();
        let row = conn
            .query_row(
                "SELECT slot,item_index,count,current_dura,max_dura FROM inventory
                 WHERE character_index=?1 AND unique_id=?2",
                params![character_index, unique_id as i64],
                |r| {
                    Ok((
                        r.get::<_, i32>(0)?,
                        r.get::<_, i32>(1)?,
                        r.get::<_, u16>(2)?,
                        r.get::<_, u16>(3)?,
                        r.get::<_, u16>(4)?,
                    ))
                },
            )
            .optional()?;
        Ok(row.map(|(slot, item_index, count, current_dura, max_dura)| {
            (
                slot,
                UserItem {
                    unique_id,
                    item_index,
                    count,
                    current_dura,
                    max_dura,
                    ..Default::default()
                },
            )
        }))
    }

    /// 消耗背包一个物品（数量>1 减 1，否则删除整格），返回被消耗物品的 (槽位, UserItem)。
    pub fn consume_inventory_item(
        &self,
        character_index: i32,
        unique_id: u64,
    ) -> anyhow::Result<Option<(i32, UserItem)>> {
        let Some((slot, item)) = self.find_inventory_item(character_index, unique_id)? else {
            return Ok(None);
        };
        let conn = self.conn.lock().unwrap();
        if item.count > 1 {
            conn.execute(
                "UPDATE inventory SET count=count-1
                 WHERE character_index=?1 AND unique_id=?2",
                params![character_index, unique_id as i64],
            )?;
        } else {
            conn.execute(
                "DELETE FROM inventory WHERE character_index=?1 AND unique_id=?2",
                params![character_index, unique_id as i64],
            )?;
        }
        Ok(Some((slot, item)))
    }

    /// 从背包移除指定数量 `count` 的某物品（丢弃用）。返回被丢弃物品的原始 (槽位, UserItem)。
    /// 若 `count` >= 仓库数量，则移除整格；否则数量减去 count。
    pub fn remove_item_count(
        &self,
        character_index: i32,
        unique_id: u64,
        count: u16,
    ) -> anyhow::Result<Option<(i32, UserItem)>> {
        let Some((slot, item)) = self.find_inventory_item(character_index, unique_id)? else {
            return Ok(None);
        };
        let conn = self.conn.lock().unwrap();
        let remove = count.max(1);
        if remove >= item.count {
            conn.execute(
                "DELETE FROM inventory WHERE character_index=?1 AND unique_id=?2",
                params![character_index, unique_id as i64],
            )?;
        } else {
            conn.execute(
                "UPDATE inventory SET count=count-?1
                 WHERE character_index=?2 AND unique_id=?3",
                params![remove, character_index, unique_id as i64],
            )?;
        }
        Ok(Some((slot, item)))
    }

    /// 加载装备：BTreeMap<slot, UserItem>（slot 见 `EquipmentSlot`）。
    pub fn load_equipment(
        &self,
        character_index: i32,
    ) -> anyhow::Result<std::collections::BTreeMap<i32, UserItem>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT slot,unique_id,item_index,current_dura,max_dura FROM equipment WHERE character_index=?1 ORDER BY slot",
        )?;
        let rows = stmt.query_map([character_index], |r| {
            Ok((
                r.get::<_, i32>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, i32>(2)?,
                r.get::<_, u16>(3)?,
                r.get::<_, u16>(4)?,
            ))
        })?;
        let mut out = std::collections::BTreeMap::new();
        for row in rows {
            let (slot, uid, item_index, cd, md) = row?;
            out.insert(
                slot,
                UserItem {
                    unique_id: uid as u64,
                    item_index,
                    count: 1,
                    current_dura: cd,
                    max_dura: md,
                    ..Default::default()
                },
            );
        }
        Ok(out)
    }

    /// 判断某 unique_id 是否在装备表中。
    pub fn find_equipment_by_uid(&self, character_index: i32, unique_id: u64) -> bool {
        let conn = self.conn.lock().unwrap();
        let n: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM equipment WHERE character_index=?1 AND unique_id=?2",
                params![character_index, unique_id as i64],
                |r| r.get(0),
            )
            .unwrap_or(0);
        n > 0
    }

    /// 把背包中的物品穿戴到装备槽 `equip_slot`。
    /// 若目标槽已有装备，先将其放回背包首空槽。
    pub fn equip_item(
        &self,
        character_index: i32,
        unique_id: u64,
        item_index: i32,
        equip_slot: i32,
    ) -> anyhow::Result<EquipOutcome> {
        let conn = self.conn.lock().unwrap();

        // 背包首空槽
        let occupied: Vec<i32> = {
            let mut stmt = conn.prepare(
                "SELECT slot FROM inventory WHERE character_index=?1 ORDER BY slot",
            )?;
            let rows = stmt.query_map([character_index], |r| r.get(0))?;
            rows.collect::<Result<Vec<i32>, _>>()?
        };
        let free = (0..INVENTORY_SIZE as i32).find(|s| !occupied.contains(s));

        // 若目标装备槽已占用，先把它放回背包首空槽
        let existing: Option<(i64, i32, u16, u16)> = conn
            .query_row(
                "SELECT unique_id,item_index,current_dura,max_dura FROM equipment
                 WHERE character_index=?1 AND slot=?2",
                params![character_index, equip_slot],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()?;
        if let Some((euid, eidx, ecd, emd)) = existing {
            let Some(free) = free else {
                // 背包满，无法换下旧装备
                return Ok(EquipOutcome {
                    returned_to_inventory: false,
                });
            };
            conn.execute(
                "INSERT INTO inventory (character_index,slot,unique_id,item_index,count,current_dura,max_dura)
                 VALUES (?1,?2,?3,?4,1,?5,?6)",
                params![character_index, free, euid, eidx, ecd, emd],
            )?;
            conn.execute(
                "DELETE FROM equipment WHERE character_index=?1 AND slot=?2",
                params![character_index, equip_slot],
            )?;
        }

        // 读取待穿戴物品自带耐久，随装备写入
        let (icd, imd): (u16, u16) = conn
            .query_row(
                "SELECT current_dura,max_dura FROM inventory WHERE character_index=?1 AND unique_id=?2",
                params![character_index, unique_id as i64],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap_or((0, 0));

        // 删除原背包格（正在穿戴的物品）
        conn.execute(
            "DELETE FROM inventory WHERE character_index=?1 AND unique_id=?2",
            params![character_index, unique_id as i64],
        )?;

        // 写入装备槽
        conn.execute(
            "INSERT INTO equipment (character_index,slot,unique_id,item_index,current_dura,max_dura)
             VALUES (?1,?2,?3,?4,?5,?6)",
            params![character_index, equip_slot, unique_id as i64, item_index, icd, imd],
        )?;
        Ok(EquipOutcome {
            returned_to_inventory: true,
        })
    }

    /// 卸下装备到背包首空槽；背包满则返回 false。
    pub fn unequip_item(
        &self,
        character_index: i32,
        equip_slot: i32,
    ) -> anyhow::Result<Option<(u64, i32)>> {
        let conn = self.conn.lock().unwrap();
        let existing: Option<(i64, i32, u16, u16)> = conn
            .query_row(
                "SELECT unique_id,item_index,current_dura,max_dura FROM equipment
                 WHERE character_index=?1 AND slot=?2",
                params![character_index, equip_slot],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()?;
        let Some((euid, eidx, ecd, emd)) = existing else {
            return Ok(None);
        };
        let occupied: Vec<i32> = {
            let mut stmt = conn.prepare(
                "SELECT slot FROM inventory WHERE character_index=?1 ORDER BY slot",
            )?;
            let rows = stmt.query_map([character_index], |r| r.get(0))?;
            rows.collect::<Result<Vec<i32>, _>>()?
        };
        let free = (0..INVENTORY_SIZE as i32).find(|s| !occupied.contains(s));
        let Some(free) = free else {
            return Ok(None); // 背包满
        };
        conn.execute(
            "INSERT INTO inventory (character_index,slot,unique_id,item_index,count,current_dura,max_dura)
             VALUES (?1,?2,?3,?4,1,?5,?6)",
            params![character_index, free, euid, eidx, ecd, emd],
        )?;
        conn.execute(
            "DELETE FROM equipment WHERE character_index=?1 AND slot=?2",
            params![character_index, equip_slot],
        )?;
        Ok(Some((euid as u64, eidx)))
    }

    // ------------------------- 背包整理：移动 / 拆分 -------------------------

    /// 装备槽内移动/互换（from<->to 均为装备槽）。返回是否成功。
    pub fn move_equipment_slot(
        &self,
        character_index: i32,
        from: i32,
        to: i32,
    ) -> Result<bool, anyhow::Error> {
        if from == to {
            return Ok(true);
        }
        let conn = self.conn.lock().unwrap();
        let src: Option<i64> = conn
            .query_row(
                "SELECT unique_id FROM equipment WHERE character_index=?1 AND slot=?2",
                params![character_index, from],
                |r| r.get(0),
            )
            .optional()?;
        if src.is_none() {
            return Ok(false);
        }
        let dst: Option<i64> = conn
            .query_row(
                "SELECT unique_id FROM equipment WHERE character_index=?1 AND slot=?2",
                params![character_index, to],
                |r| r.get(0),
            )
            .optional()?;
        if dst.is_none() {
            conn.execute(
                "UPDATE equipment SET slot=?1 WHERE character_index=?2 AND slot=?3",
                params![to, character_index, from],
            )?;
            return Ok(true);
        }
        conn.execute(
            "UPDATE equipment SET slot=-1 WHERE character_index=?1 AND slot=?2",
            params![character_index, from],
        )?;
        conn.execute(
            "UPDATE equipment SET slot=?1 WHERE character_index=?2 AND slot=?3",
            params![from, character_index, to],
        )?;
        conn.execute(
            "UPDATE equipment SET slot=?1 WHERE character_index=?2 AND slot=-1",
            params![to, character_index],
        )?;
        Ok(true)
    }

    /// 背包内移动/互换物品到目标槽。目标槽空则直接移动，否则与原物品互换。
    /// 返回 (是否成功, 是否发生互换)。
    pub fn move_inventory_item(
        &self,
        character_index: i32,
        from: i32,
        to: i32,
    ) -> anyhow::Result<(bool, bool)> {
        if from == to {
            return Ok((true, false));
        }
        let conn = self.conn.lock().unwrap();
        // 源槽必须有物品
        let src: Option<i64> = conn
            .query_row(
                "SELECT unique_id FROM inventory WHERE character_index=?1 AND slot=?2",
                params![character_index, from],
                |r| r.get(0),
            )
            .optional()?;
        if src.is_none() {
            return Ok((false, false));
        }
        // 目标槽是否有物品
        let dst: Option<i64> = conn
            .query_row(
                "SELECT unique_id FROM inventory WHERE character_index=?1 AND slot=?2",
                params![character_index, to],
                |r| r.get(0),
            )
            .optional()?;
        if dst.is_none() {
            // 目标空：直接把源行槽位改为 to
            conn.execute(
                "UPDATE inventory SET slot=?1 WHERE character_index=?2 AND slot=?3",
                params![to, character_index, from],
            )?;
            return Ok((true, false));
        }
        // 目标也有物品：互换两行的槽位
        conn.execute(
            "UPDATE inventory SET slot=-1 WHERE character_index=?1 AND slot=?2",
            params![character_index, from],
        )?;
        conn.execute(
            "UPDATE inventory SET slot=?1 WHERE character_index=?2 AND slot=?3",
            params![from, character_index, to],
        )?;
        conn.execute(
            "UPDATE inventory SET slot=?1 WHERE character_index=?2 AND slot=-1",
            params![to, character_index],
        )?;
        Ok((true, true))
    }

    /// 拆分堆叠：从 unique_id 上分出 `count` 个到背包首个空槽（新 unique_id）。
    /// count 必须 < 原数量。返回被分配到的空槽号, 否则 None。
    pub fn split_inventory_item(
        &self,
        character_index: i32,
        unique_id: u64,
        count: u16,
    ) -> anyhow::Result<Option<i32>> {
        let conn = self.conn.lock().unwrap();
        let Some((slot, item_index, has, cd, md)) = conn
            .query_row(
                "SELECT slot,item_index,count,current_dura,max_dura FROM inventory
                 WHERE character_index=?1 AND unique_id=?2",
                params![character_index, unique_id as i64],
                |r| Ok((r.get::<_, i32>(0)?, r.get::<_, i32>(1)?, r.get::<_, u16>(2)?, r.get::<_, u16>(3)?, r.get::<_, u16>(4)?)),
            )
            .optional()?
        else {
            return Ok(None);
        };
        if count == 0 || count >= has {
            return Ok(None); // 不能拆成 0，也不能拆分出全部
        }
        // 找空槽
        let occupied: Vec<i32> = {
            let mut stmt = conn.prepare(
                "SELECT slot FROM inventory WHERE character_index=?1 ORDER BY slot",
            )?;
            let rows = stmt.query_map([character_index], |r| r.get(0))?;
            rows.collect::<Result<Vec<i32>, _>>()?
        };
        let Some(free) = (0..INVENTORY_SIZE as i32).find(|s| !occupied.contains(s)) else {
            return Ok(None);
        };
        // 原堆减 count
        conn.execute(
            "UPDATE inventory SET count=count-?1 WHERE character_index=?2 AND unique_id=?3",
            params![count, character_index, unique_id as i64],
        )?;
        // 新行
        conn.execute(
            "INSERT INTO inventory (character_index,slot,unique_id,item_index,count,current_dura,max_dura)
             VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![
                character_index,
                free,
                make_unique(character_index, free),
                item_index,
                count,
                cd,
                md
            ],
        )?;
        let _ = slot;
        Ok(Some(free))
    }

    /// 合并堆叠：把背包 item id_from 的数量并入同种物品 id_to，删除 id_from 行。
    /// 返回 (是否成功, 合并后数量)。
    pub fn merge_inventory_items(
        &self,
        character_index: i32,
        id_from: u64,
        id_to: u64,
    ) -> anyhow::Result<(bool, u16)> {
        if id_from == id_to {
            return Ok((false, 0));
        }
        let conn = self.conn.lock().unwrap();
        let both: Option<(i32, u16, i32, i32)> = conn
            .query_row(
                "SELECT item_index,count,current_dura,max_dura FROM inventory
                 WHERE character_index=?1 AND unique_id=?2",
                params![character_index, id_from as i64],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()?;
        let (item_from, count_from, _cd, _md) = match both {
            Some(v) => v,
            None => return Ok((false, 0)),
        };
        let item_to: Option<(i32, u16)> = conn
            .query_row(
                "SELECT item_index,count FROM inventory WHERE character_index=?1 AND unique_id=?2",
                params![character_index, id_to as i64],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let Some((it, ct)) = item_to else {
            return Ok((false, 0));
        };
        if it != item_from {
            return Ok((false, 0)); // 非同种物品不可合并
        }
        conn.execute(
            "UPDATE inventory SET count=?1 WHERE character_index=?2 AND unique_id=?3",
            params![ct + count_from, character_index, id_to as i64],
        )?;
        conn.execute(
            "DELETE FROM inventory WHERE character_index=?1 AND unique_id=?2",
            params![character_index, id_from as i64],
        )?;
        Ok((true, ct + count_from))
    }

    // ------------------------- 仓库（个人储物箱） -------------------------

    /// 加载仓库全部物品：[(slot, UserItem)]
    pub fn load_storage(&self, character_index: i32) -> anyhow::Result<Vec<(i32, UserItem)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT slot,unique_id,item_index,count,current_dura,max_dura FROM storage
             WHERE character_index=?1 ORDER BY slot",
        )?;
        let rows = stmt.query_map([character_index], |r| {
            Ok((
                r.get::<_, i32>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, i32>(2)?,
                r.get::<_, u16>(3)?,
                r.get::<_, u16>(4)?,
                r.get::<_, u16>(5)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (slot, uid, idx, count, cd, md) = row?;
            out.push((
                slot,
                UserItem {
                    unique_id: uid as u64,
                    item_index: idx,
                    count,
                    current_dura: cd,
                    max_dura: md,
                    ..Default::default()
                },
            ));
        }
        Ok(out)
    }

    /// 仓库为固定槽位向量（空槽 None）。
    pub fn storage_slots(&self, character_index: i32) -> anyhow::Result<Vec<Option<UserItem>>> {
        let rows = self.load_storage(character_index)?;
        let mut slots: Vec<Option<UserItem>> = (0..STORAGE_SIZE).map(|_| None).collect();
        for (slot, item) in rows {
            if let Some(slot) = usize::try_from(slot).ok() {
                if let Some(s) = slots.get_mut(slot) {
                    *s = Some(item);
                }
            }
        }
        Ok(slots)
    }

    /// 存入：从背包槽 from 移到仓库槽 to（目标有物则互换）。成功返回 true。
    pub fn store_item(&self, character_index: i32, from: i32, to: i32) -> anyhow::Result<bool> {
        let conn = self.conn.lock().unwrap();
        let src: Option<(i64, i32, i64, u16, u16)> = conn
            .query_row(
                "SELECT unique_id,item_index,count,current_dura,max_dura FROM inventory
                 WHERE character_index=?1 AND slot=?2",
                params![character_index, from],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .optional()?;
        let Some((uid, idx, cnt, cd, md)) = src else {
            return Ok(false);
        };
        // 仓库目标槽已有 → 先移回背包源槽（互换）
        let dst: Option<i64> = conn
            .query_row(
                "SELECT unique_id FROM storage WHERE character_index=?1 AND slot=?2",
                params![character_index, to],
                |r| r.get(0),
            )
            .optional()?;
        if dst.is_some() {
            let drow: (i64, i32, i64, u16, u16) = conn
                .query_row(
                    "SELECT unique_id,item_index,count,current_dura,max_dura FROM storage
                     WHERE character_index=?1 AND slot=?2",
                    params![character_index, to],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
                )?;
            // 仓库原物 → 背包 from
            conn.execute(
                "DELETE FROM inventory WHERE character_index=?1 AND slot=?2",
                params![character_index, from],
            )?;
            conn.execute(
                "INSERT INTO inventory (character_index,slot,unique_id,item_index,count,current_dura,max_dura)
                 VALUES (?1,?2,?3,?4,?5,?6,?7)",
                params![character_index, from, drow.0, drow.1, drow.2, drow.3, drow.4],
            )?;
        } else {
            // 目标空：删除背包源行
            conn.execute(
                "DELETE FROM inventory WHERE character_index=?1 AND slot=?2",
                params![character_index, from],
            )?;
        }
        conn.execute(
            "INSERT OR REPLACE INTO storage (character_index,slot,unique_id,item_index,count,current_dura,max_dura)
             VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![character_index, to, uid, idx, cnt, cd, md],
        )?;
        Ok(true)
    }

    /// 取出：从仓库槽 from 移到背包槽 to（目标有物则互换）。成功返回 true。
    pub fn take_item(&self, character_index: i32, from: i32, to: i32) -> anyhow::Result<bool> {
        let conn = self.conn.lock().unwrap();
        let src: Option<(i64, i32, i64, u16, u16)> = conn
            .query_row(
                "SELECT unique_id,item_index,count,current_dura,max_dura FROM storage
                 WHERE character_index=?1 AND slot=?2",
                params![character_index, from],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .optional()?;
        let Some((uid, idx, cnt, cd, md)) = src else {
            return Ok(false);
        };
        // 背包目标槽已有 → 先存入仓库源槽（互换）
        let dst: Option<i64> = conn
            .query_row(
                "SELECT unique_id FROM inventory WHERE character_index=?1 AND slot=?2",
                params![character_index, to],
                |r| r.get(0),
            )
            .optional()?;
        if dst.is_some() {
            let drow: (i64, i32, i64, u16, u16) = conn
                .query_row(
                    "SELECT unique_id,item_index,count,current_dura,max_dura FROM inventory
                     WHERE character_index=?1 AND slot=?2",
                    params![character_index, to],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
                )?;
            // 背包原物 → 仓库 from
            conn.execute(
                "DELETE FROM storage WHERE character_index=?1 AND slot=?2",
                params![character_index, from],
            )?;
            conn.execute(
                "INSERT INTO storage (character_index,slot,unique_id,item_index,count,current_dura,max_dura)
                 VALUES (?1,?2,?3,?4,?5,?6,?7)",
                params![character_index, from, drow.0, drow.1, drow.2, drow.3, drow.4],
            )?;
        } else {
            conn.execute(
                "DELETE FROM storage WHERE character_index=?1 AND slot=?2",
                params![character_index, from],
            )?;
        }
        conn.execute(
            "INSERT OR REPLACE INTO inventory (character_index,slot,unique_id,item_index,count,current_dura,max_dura)
             VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![character_index, to, uid, idx, cnt, cd, md],
        )?;
        Ok(true)
    }

    // ------------------------- 装备耐久 / 修理 -------------------------

    /// 扣除某装备的当前耐久（下限 0）。返回 (当前耐久, 最大耐久, 是否跌破 0 失效)。
    pub fn damage_equipment(
        &self,
        character_index: i32,
        slot: i32,
        amount: u16,
    ) -> anyhow::Result<(u16, u16)> {
        let conn = self.conn.lock().unwrap();
        let (cd, md): (u16, u16) = conn
            .query_row(
                "SELECT current_dura,max_dura FROM equipment WHERE character_index=?1 AND slot=?2",
                params![character_index, slot],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap_or((0, 0));
        if md == 0 {
            return Ok((cd, md)); // 无耐久装备不受影响
        }
        let new_cd = cd.saturating_sub(amount);
        conn.execute(
            "UPDATE equipment SET current_dura=?1 WHERE character_index=?2 AND slot=?3",
            params![new_cd, character_index, slot],
        )?;
        Ok((new_cd, md))
    }

    /// 修理某件装备（按 unique_id，背包或装备槽皆可）。返回 (当前耐久, 最大耐久, 修理费用)。
    /// 费用 = (max - current) * 单点价格。玩家在 Inventory 中的装备也按 unique_id 定位。
    pub fn repair_item(
        &self,
        character_index: i32,
        unique_id: u64,
        price_per: u32,
    ) -> anyhow::Result<Option<(u16, u16, u32)>> {
        let conn = self.conn.lock().unwrap();
        // 先查装备表（最常见），否则查背包
        let row = conn
            .query_row(
                "SELECT current_dura,max_dura FROM equipment WHERE character_index=?1 AND unique_id=?2",
                params![character_index, unique_id as i64],
                |r| Ok((r.get::<_, u16>(0)?, r.get::<_, u16>(1)?)),
            )
            .optional()?
            .or(
                conn.query_row(
                    "SELECT current_dura,max_dura FROM inventory WHERE character_index=?1 AND unique_id=?2",
                    params![character_index, unique_id as i64],
                    |r| Ok((r.get::<_, u16>(0)?, r.get::<_, u16>(1)?)),
                )
                .optional()?,
            );
        let Some((cd, md)) = row else {
            return Ok(None); // 物品不存在
        };
        if md == 0 {
            return Ok(None); // 无耐久物不可修
        }
        let missing = md.saturating_sub(cd) as u32;
        let cost = missing.saturating_mul(price_per);
        Ok(Some((cd, md, cost)))
    }

    /// 实际维修：把某物品耐久回满（装备或背包），应在玩家付款后调用。
    pub fn apply_repair(&self, character_index: i32, unique_id: u64) -> anyhow::Result<bool> {
        let conn = self.conn.lock().unwrap();
        for table in ["equipment", "inventory"] {
            let hit = conn.execute(
                &format!(
                    "UPDATE {table} SET current_dura=max_dura WHERE character_index=?1 AND unique_id=?2"
                ),
                params![character_index, unique_id as i64],
            )?;
            if hit > 0 {
                return Ok(true);
            }
        }
        Ok(false)
    }

    // ------------------------- 仓库密码 -------------------------

    /// 返回 (密码hash, 设置时间)；未设置则 None。
    pub fn get_storage_pw(&self, c: i32) -> Option<(String, i64)> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT pw, set_at FROM storage_pw WHERE character_index=?1",
            params![c],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .unwrap_or(None)
    }

    pub fn set_storage_pw(&self, c: i32, hash: &str, set_at: i64) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO storage_pw (character_index,pw,set_at) VALUES (?1,?2,?3)
             ON CONFLICT(character_index) DO UPDATE SET pw=excluded.pw, set_at=excluded.set_at",
            params![c, hash, set_at],
        )?;
        Ok(())
    }

    pub fn clear_storage_pw(&self, c: i32) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM storage_pw WHERE character_index=?1", params![c])?;
        Ok(())
    }

    // ------------------------- 师徒 -------------------------

    /// 返回某玩家（mentee）的 mentor 角色索引。
    pub fn get_mentor(&self, c: i32) -> Option<i32> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT mentor_index FROM mentor_relations WHERE character_index=?1",
            params![c],
            |r| r.get(0),
        )
        .optional()
        .unwrap_or(None)
    }

    pub fn set_mentor(&self, c: i32, m: i32) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO mentor_relations (character_index,mentor_index) VALUES (?1,?2)
             ON CONFLICT(character_index) DO UPDATE SET mentor_index=excluded.mentor_index",
            params![c, m],
        )?;
        conn.execute(
            "INSERT OR IGNORE INTO mentor_mentees (mentor_index,mentee_index) VALUES (?1,?2)",
            params![m, c],
        )?;
        Ok(())
    }

    pub fn clear_mentor(&self, c: i32) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        let m = self.get_mentor(c);
        conn.execute(
            "DELETE FROM mentor_relations WHERE character_index=?1",
            params![c],
        )?;
        if let Some(mi) = m {
            conn.execute(
                "DELETE FROM mentor_mentees WHERE mentor_index=?1 AND mentee_index=?2",
                params![mi, c],
            )?;
        }
        Ok(())
    }

    /// 某 mentor 名下的全部 mentee 角色索引。
    pub fn mentees_of(&self, m: i32) -> Vec<i32> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT mentee_index FROM mentor_mentees WHERE mentor_index=?1")
            .unwrap();
        let rows = stmt
            .query_map([m], |r| r.get::<_, i32>(0))
            .unwrap();
        rows.filter_map(Result::ok).collect()
    }

    // ------------------------- 婚姻 -------------------------

    pub fn get_spouse(&self, c: i32) -> Option<i32> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT spouse_index FROM marriages WHERE char_index=?1",
            params![c],
            |r| r.get(0),
        )
        .optional()
        .unwrap_or(None)
    }

    pub fn set_spouse(&self, a: i32, b: i32, date: i64) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO marriages (char_index,spouse_index,date) VALUES (?1,?2,?3)
             ON CONFLICT(char_index) DO UPDATE SET spouse_index=excluded.spouse_index, date=excluded.date",
            params![a, b, date],
        )?;
        conn.execute(
            "INSERT INTO marriages (char_index,spouse_index,date) VALUES (?1,?2,?3)
             ON CONFLICT(char_index) DO UPDATE SET spouse_index=excluded.spouse_index, date=excluded.date",
            params![b, a, date],
        )?;
        Ok(())
    }

    pub fn clear_spouse(&self, c: i32) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM marriages WHERE char_index=?1", params![c])?;
        Ok(())
    }

    /// 读取某角色的婚姻日期（若无则 0）。
    pub fn marriage_date(&self, c: i32) -> i64 {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT date FROM marriages WHERE char_index=?1",
            params![c],
            |r| r.get(0),
        )
        .unwrap_or(0)
    }

    // ------------------------- 转生 -------------------------

    /// 转生次数 +1，返回新的转生次数。
    pub fn add_reincarnation(&self, c: i32) -> anyhow::Result<u32> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE characters SET reincarnations=reincarnations+1 WHERE character_index=?1",
            params![c],
        )?;
        let n: i64 = conn
            .query_row(
                "SELECT reincarnations FROM characters WHERE character_index=?1",
                params![c],
                |r| r.get(0),
            )
            .unwrap_or(0);
        Ok(n as u32)
    }

    /// 当前转生次数。
    pub fn reincarnations(&self, c: i32) -> u32 {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT reincarnations FROM characters WHERE character_index=?1",
            params![c],
            |r| r.get(0),
        )
        .unwrap_or(0)
    }

    // ------------------------- 精炼 -------------------------

    /// 读取物品精炼值（equipment 或 inventory）。
    pub fn read_refines(&self, table: &str, idx: i32, uid: u64) -> u32 {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            &format!(
                "SELECT refines FROM {table} WHERE character_index=?1 AND unique_id=?2"
            ),
            params![idx, uid as i64],
            |r| r.get(0),
        )
        .unwrap_or(0)
    }

    /// 写入物品精炼值。
    pub fn set_refines(&self, table: &str, idx: i32, uid: u64, count: u32) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            &format!(
                "UPDATE {table} SET refines=?1 WHERE character_index=?2 AND unique_id=?3"
            ),
            params![count, idx, uid as i64],
        )?;
        Ok(())
    }

    /// 计算下一次精炼的成功率与金币消耗（静态表）。
    /// 成功几率随当前精炼值递减（下限 10%），金币 = (cur+1)*50。
    pub fn next_refine(cur: u32) -> (f32, u32) {
        let chance = (0.9f32 - cur as f32 * 0.15).max(0.1);
        let cost = (cur + 1) * 50;
        (chance, cost)
    }

    // ------------------------- 好友 -------------------------

    /// 按角色索引查角色名（跨账号唯一）。
    pub fn char_name(&self, character_index: i32) -> anyhow::Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let name = conn
            .query_row(
                "SELECT name FROM characters WHERE character_index=?1",
                params![character_index],
                |r| r.get(0),
            )
            .optional()?;
        Ok(name)
    }

    /// 好友原始数据：(friend_index, name, memo, blocked)，不含在线状态。
    pub fn friend_rows(
        &self,
        character_index: i32,
    ) -> anyhow::Result<Vec<(i32, String, String, bool)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT f.friend_index, c.name, f.memo, f.blocked
             FROM friends f LEFT JOIN characters c ON c.character_index = f.friend_index
             WHERE f.character_index=?1 ORDER BY f.added_at",
        )?;
        let rows = stmt.query_map([character_index], |r| {
            Ok((
                r.get::<_, i32>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, i64>(3)? != 0,
            ))
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// 添加好友。返回 (是否成功, 好友角色索引)。目标角色不存在或已加则失败。
    pub fn add_friend(
        &self,
        character_index: i32,
        friend_name: &str,
        memo: &str,
        blocked: bool,
    ) -> anyhow::Result<(bool, Option<i32>)> {
        let conn = self.conn.lock().unwrap();
        let Some(friend_index) = conn
            .query_row(
                "SELECT character_index FROM characters WHERE name=?1",
                params![friend_name],
                |r| r.get(0),
            )
            .optional()?
        else {
            return Ok((false, None)); // 目标角色不存在
        };
        if friend_index == character_index {
            return Ok((false, None)); // 不能加自己
        }
        let exists: i64 = conn.query_row(
            "SELECT COUNT(*) FROM friends WHERE character_index=?1 AND friend_index=?2",
            params![character_index, friend_index],
            |r| r.get(0),
        )?;
        if exists > 0 {
            return Ok((false, Some(friend_index))); // 已是好友
        }
        conn.execute(
            "INSERT INTO friends (character_index,friend_index,memo,blocked) VALUES (?1,?2,?3,?4)",
            params![character_index, friend_index, memo, blocked as i64],
        )?;
        Ok((true, Some(friend_index)))
    }

    /// 移除好友。返回是否成功。
    pub fn remove_friend(&self, character_index: i32, friend_index: i32) -> anyhow::Result<bool> {
        let conn = self.conn.lock().unwrap();
        let n = conn.execute(
            "DELETE FROM friends WHERE character_index=?1 AND friend_index=?2",
            params![character_index, friend_index],
        )?;
        Ok(n > 0)
    }

    pub fn item_name(&self, item_index: i32) -> anyhow::Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        match conn.query_row("SELECT name FROM items WHERE item_index=?1", [item_index], |r| r.get(0)) {
            Ok(name) => Ok(Some(name)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}

/// 装备操作结果：`returned_to_inventory` 表示旧装备是否成功放回背包（false=背包满，操作失败）。
#[derive(Debug, Clone, Copy)]
pub struct EquipOutcome {
    pub returned_to_inventory: bool,
}

#[derive(Debug, Clone)]
pub struct CharacterRow {
    pub index: i32,
    pub name: String,
    pub class: u8,
    pub gender: u8,
    pub level: i64,
    pub x: i32,
    pub y: i32,
    pub direction: i32,
    pub hp: i32,
    pub mp: i32,
    pub gold: i64,
    pub experience: i64,
}

pub fn hash_password(s: &str) -> String {
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    format!("{:x}", h.finalize())
}

fn num_to_class(v: u8) -> crystal_protocol::types::MirClass {
    match v {
        0 => crystal_protocol::types::MirClass::Warrior,
        1 => crystal_protocol::types::MirClass::Wizard,
        2 => crystal_protocol::types::MirClass::Taoist,
        3 => crystal_protocol::types::MirClass::Assassin,
        _ => crystal_protocol::types::MirClass::Archer,
    }
}

fn num_to_gender(v: u8) -> crystal_protocol::types::MirGender {
    match v {
        0 => crystal_protocol::types::MirGender::Male,
        _ => crystal_protocol::types::MirGender::Female,
    }
}

fn make_unique(char_index: i32, slot: i32) -> i64 {
    // 简单唯一ID: 角色*100 + 槽位
    (char_index as i64) * 1000 + slot as i64
}

/// 供 main 使用（路径默认 data/crystal.db）
pub fn default_db_path() -> PathBuf {
    Path::new("data").join("crystal.db")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_CTR: AtomicU64 = AtomicU64::new(0);

    fn temp_db() -> (Arc<Database>, PathBuf) {
        let mut p = std::env::temp_dir();
        let n = TEST_CTR.fetch_add(1, Ordering::SeqCst);
        p.push(format!("crystal_db_test_{}_{}.db", std::process::id(), n));
        let _ = std::fs::remove_file(&p);
        (Database::open(&p).unwrap(), p)
    }

    #[test]
    fn starter_inventory_and_slots() {
        let (db, _p) = temp_db();
        db.register("tester").unwrap();
        let info = db
            .add_character("tester", "测试", crystal_protocol::types::MirClass::Warrior, crystal_protocol::types::MirGender::Male)
            .unwrap()
            .unwrap();
        // 初始背包：木剑(槽0) + 金创药x5(槽1)
        let slots = db.inventory_slots(info.index).unwrap();
        assert_eq!(slots.len(), INVENTORY_SIZE);
        assert_eq!(slots[0].as_ref().map(|i| i.item_index), Some(1));
        assert_eq!(slots[1].as_ref().map(|i| i.item_index), Some(3));
        assert_eq!(slots[1].as_ref().map(|i| i.count), Some(5));
        assert!(slots[2].is_none());
    }

    #[test]
    fn consume_item_decrements_and_removes() {
        let (db, _p) = temp_db();
        db.register("tester").unwrap();
        let info = db
            .add_character("tester", "测试", crystal_protocol::types::MirClass::Warrior, crystal_protocol::types::MirGender::Male)
            .unwrap()
            .unwrap();
        // 金创药 uid = char*1000+1
        let uid = (info.index as u64) * 1000 + 1;
        let (slot, item) = db.find_inventory_item(info.index, uid).unwrap().unwrap();
        assert_eq!(slot, 1);
        assert_eq!(item.count, 5);
        // 消耗一次 -> 剩 4
        let (_s, consumed) = db.consume_inventory_item(info.index, uid).unwrap().unwrap();
        assert_eq!(consumed.count, 5);
        assert_eq!(db.inventory_slots(info.index).unwrap()[1].as_ref().map(|i| i.count), Some(4));
        // 消耗其余 4 次 -> 整格被删
        for _ in 0..4 {
            db.consume_inventory_item(info.index, uid).unwrap();
        }
        assert!(db.inventory_slots(info.index).unwrap()[1].is_none());
    }

    #[test]
    fn equip_moves_item_and_returns_old() {
        let (db, _p) = temp_db();
        db.register("tester").unwrap();
        let info = db
            .add_character("tester", "测试", crystal_protocol::types::MirClass::Warrior, crystal_protocol::types::MirGender::Male)
            .unwrap()
            .unwrap();
        let weapon_uid = (info.index as u64) * 1000 + 0;
        // 穿戴木剑到武器槽(0)
        let outcome = db.equip_item(info.index, weapon_uid, 1, 0).unwrap();
        assert!(outcome.returned_to_inventory);
        // 背包不再有木剑，装备槽 0 有木剑
        assert!(db.find_inventory_item(info.index, weapon_uid).unwrap().is_none());
        let equip = db.load_equipment(info.index).unwrap();
        assert_eq!(equip.get(&0).map(|i| i.item_index), Some(1));
        // 卸下装备 -> 回背包首空槽
        let unequipped = db.unequip_item(info.index, 0).unwrap();
        assert_eq!(unequipped.map(|(_, idx)| idx), Some(1));
        assert_eq!(db.load_equipment(info.index).unwrap().get(&0).map(|i| i.item_index), None);
        let weapon_back = db
            .inventory_slots(info.index)
            .unwrap()
            .iter()
            .any(|s| s.as_ref().map(|i| i.item_index) == Some(1));
        assert!(weapon_back);
    }

    #[test]
    fn remove_item_count_partial_and_full() {
        let (db, _p) = temp_db();
        db.register("tester").unwrap();
        let info = db
            .add_character("tester", "测试", crystal_protocol::types::MirClass::Warrior, crystal_protocol::types::MirGender::Male)
            .unwrap()
            .unwrap();
        // 金创药 uid = char*1000+1，数量 5
        let uid = (info.index as u64) * 1000 + 1;
        // 丢弃 2 个 -> 剩 3
        let (_s, dropped) = db.remove_item_count(info.index, uid, 2).unwrap().unwrap();
        assert_eq!(dropped.count, 5);
        assert_eq!(db.find_inventory_item(info.index, uid).unwrap().unwrap().1.count, 3);
        // 再丢 3 个（>= 现有）-> 整格删除
        db.remove_item_count(info.index, uid, 3).unwrap();
        assert!(db.find_inventory_item(info.index, uid).unwrap().is_none());
    }

    #[test]
    fn save_character_state_persists_gold_exp_level() {
        let (db, _p) = temp_db();
        db.register("tester").unwrap();
        let info = db
            .add_character("tester", "测试", crystal_protocol::types::MirClass::Warrior, crystal_protocol::types::MirGender::Male)
            .unwrap()
            .unwrap();
        db.save_character_state(info.index, 401, 402, 0, 55, 30, 1234, 567, 3)
            .unwrap();
        let ch = db.get_character("tester", info.index).unwrap().unwrap();
        assert_eq!(ch.x, 401);
        assert_eq!(ch.y, 402);
        assert_eq!(ch.hp, 55);
        assert_eq!(ch.mp, 30);
        assert_eq!(ch.gold, 1234);
        assert_eq!(ch.experience, 567);
        assert_eq!(ch.level, 3);
    }

    #[test]
    fn add_item_merges_stacks() {
        let (db, _p) = temp_db();
        db.register("tester").unwrap();
        let info = db
            .add_character("tester", "测试", crystal_protocol::types::MirClass::Warrior, crystal_protocol::types::MirGender::Male)
            .unwrap()
            .unwrap();
        // 初始背包: 槽0=木剑x1, 槽1=金创药x5
        // 再增加 3 个金创药 -> 应合并到已有的金创药堆(8)，不占新槽
        db.add_item_to_inventory(info.index, 3, 3).unwrap();
        let slots = db.inventory_slots(info.index).unwrap();
        assert_eq!(slots[1].as_ref().map(|i| i.item_index), Some(3));
        assert_eq!(slots[1].as_ref().map(|i| i.count), Some(8));
        assert!(slots[2].is_none(), "同物品应合并堆叠，不应占新槽");
        // 木剑(已在槽0) -> 同样合并进槽0(2把)，而非新槽2
        db.add_item_to_inventory(info.index, 1, 1).unwrap();
        let slots = db.inventory_slots(info.index).unwrap();
        assert_eq!(slots[0].as_ref().map(|i| i.item_index), Some(1));
        assert_eq!(slots[0].as_ref().map(|i| i.count), Some(2));
        assert!(slots[2].is_none(), "已有木剑，不应新建槽2");
    }

    #[test]
    fn mail_roundtrip() {
        let (db, _p) = temp_db();
        db.register("tester").unwrap();
        let a = db
            .add_character("tester", "甲", crystal_protocol::types::MirClass::Warrior, crystal_protocol::types::MirGender::Male)
            .unwrap()
            .unwrap();
        let b = db
            .add_character("tester", "乙", crystal_protocol::types::MirClass::Warrior, crystal_protocol::types::MirGender::Male)
            .unwrap()
            .unwrap();
        let id = db.send_mail(b.index, "甲", "你好", "给你点金币", 100, 0).unwrap();
        let inbox = db.mail_inbox(b.index).unwrap();
        assert_eq!(inbox.len(), 1);
        assert_eq!(inbox[0].title, "你好");
        assert_eq!(inbox[0].gold, 100);
        assert!(!inbox[0].is_read);
        db.mark_mail_read(id).unwrap();
        assert!(db.mail_inbox(b.index).unwrap()[0].is_read);
        assert!(db.mail_inbox(a.index).unwrap().is_empty());
        db.delete_mail(id).unwrap();
        assert!(db.mail_inbox(b.index).unwrap().is_empty());
    }

    #[test]
    fn mail_claim_gold_zeroes_attachment() {
        let (db, _p) = temp_db();
        db.register("tester").unwrap();
        let a = db
            .add_character("tester", "甲", crystal_protocol::types::MirClass::Warrior, crystal_protocol::types::MirGender::Male)
            .unwrap()
            .unwrap();
        let b = db
            .add_character("tester", "乙", crystal_protocol::types::MirClass::Warrior, crystal_protocol::types::MirGender::Male)
            .unwrap()
            .unwrap();
        let id = db.send_mail(b.index, "甲", "金币", "", 500, 12345).unwrap();
        // 领取金币 + 物品附件
        assert_eq!(db.claim_mail_gold(id).unwrap(), 500);
        assert_eq!(db.claim_mail_item(id).unwrap(), 12345);
        let m = db.get_mail(id, b.index).unwrap().unwrap();
        assert_eq!(m.gold, 0);
        assert_eq!(m.item_uid, 0);
    }

    #[test]
    fn char_index_by_name_lookup() {
        let (db, _p) = temp_db();
        db.register("tester").unwrap();
        let info = db
            .add_character("tester", "寻名", crystal_protocol::types::MirClass::Warrior, crystal_protocol::types::MirGender::Male)
            .unwrap()
            .unwrap();
        assert_eq!(db.char_index_by_name("寻名").unwrap(), Some(info.index));
        assert_eq!(db.char_index_by_name("不存在").unwrap(), None);
        // 直接发金币
        let before = db.add_char_gold(info.index, 250).unwrap();
        assert_eq!(before, 1000); // 新角色默认 1000
    }

    #[test]
    fn quest_accept_kill_reward_full_flow() {
        let (db, _p) = temp_db();
        db.register("tester").unwrap();
        let info = db
            .add_character("tester", "任务者", crystal_protocol::types::MirClass::Warrior, crystal_protocol::types::MirGender::Male)
            .unwrap()
            .unwrap();
        // quest 1 = 猎杀骷髅（image 3，需 5 只）
        db.accept_quest(info.index, 1).unwrap();
        // 击杀 5 只骷髅：第 5 只达成
        for i in 1..=5 {
            let (matched, done) = crate::quest::register_kill("任务者", info.index, 3, &db);
            assert!(matched);
            if i < 5 {
                assert_eq!(done, 0);
            } else {
                assert_eq!(done, 1);
            }
        }
        let progress = db.load_quest_progress(info.index).unwrap();
        let q = progress.iter().find(|p| p.quest_id == 1).unwrap();
        assert_eq!(q.killed, 5);
        assert!(q.completed);
        assert!(!q.finished);
        // 领取奖励
        let (def, gold, exp) = crate::quest::reward(info.index, 1, &db).unwrap();
        assert_eq!(def.id, 1);
        assert!(gold > 0 && exp > 0);
        let progress = db.load_quest_progress(info.index).unwrap();
        assert!(progress.iter().find(|p| p.quest_id == 1).unwrap().finished);
        // 重复领取应失败
        assert!(crate::quest::reward(info.index, 1, &db).is_err());
        // 未接任务不应推进
        let (matched, _) = crate::quest::register_kill("任务者", info.index, 4, &db);
        assert!(!matched); // 未接 quest 2
    }

    #[test]
    fn move_inventory_item_to_empty_and_swap() {
        let (db, _p) = temp_db();
        db.register("tester").unwrap();
        let info = db
            .add_character("tester", "整理者", crystal_protocol::types::MirClass::Warrior, crystal_protocol::types::MirGender::Male)
            .unwrap()
            .unwrap();
        // 初始: 槽0=木剑(1), 槽1=金创药x5
        // 移到空槽 5
        let (ok, swapped) = db.move_inventory_item(info.index, 0, 5).unwrap();
        assert!(ok && !swapped);
        let slots = db.inventory_slots(info.index).unwrap();
        assert!(slots[0].is_none());
        assert_eq!(slots[5].as_ref().map(|i| i.item_index), Some(1));
        // 互换 槽5(木剑) 与 槽1(金创药)
        let (ok, swapped) = db.move_inventory_item(info.index, 5, 1).unwrap();
        assert!(ok && swapped);
        let slots = db.inventory_slots(info.index).unwrap();
        assert_eq!(slots[1].as_ref().map(|i| i.item_index), Some(1)); // 木剑到槽1
        assert_eq!(slots[5].as_ref().map(|i| i.item_index), Some(3)); // 金创药到槽5
    }

    #[test]
    fn split_inventory_item_creates_new_stack() {
        let (db, _p) = temp_db();
        db.register("tester").unwrap();
        let info = db
            .add_character("tester", "拆分者", crystal_protocol::types::MirClass::Warrior, crystal_protocol::types::MirGender::Male)
            .unwrap()
            .unwrap();
        // 金创药槽1 = x5, uid = index*1000+1
        let uid = (info.index as u64) * 1000 + 1;
        let slot = db.split_inventory_item(info.index, uid, 2).unwrap();
        assert!(slot.is_some());
        let slots = db.inventory_slots(info.index).unwrap();
        // 原堆剩 3
        assert_eq!(slots[1].as_ref().map(|i| i.count), Some(3));
        // 新堆在空槽 = 2
        assert_eq!(slots[2].as_ref().map(|i| i.count), Some(2));
        // 不能拆出全部：原堆剩 3，拆 3(>=现有) 应失败；新堆剩 2，拆 2 应失败
        assert!(db.split_inventory_item(info.index, uid, 3).unwrap().is_none());
        assert!(db.split_inventory_item(info.index, slots[2].as_ref().unwrap().unique_id, 2).unwrap().is_none());
    }

    #[test]
    fn storage_store_take_and_swap() {
        let (db, _p) = temp_db();
        db.register("tester").unwrap();
        let info = db
            .add_character("tester", "仓库者", crystal_protocol::types::MirClass::Warrior, crystal_protocol::types::MirGender::Male)
            .unwrap()
            .unwrap();
        // 存入：背包槽0(木剑) -> 仓库槽0
        assert!(db.store_item(info.index, 0, 0).unwrap());
        let storage = db.storage_slots(info.index).unwrap();
        assert_eq!(storage[0].as_ref().map(|i| i.item_index), Some(1));
        let inv = db.inventory_slots(info.index).unwrap();
        assert!(inv[0].is_none(), "存入后背包槽0应清空");
        // 再存 金创药(槽1) -> 仓库槽1
        assert!(db.store_item(info.index, 1, 1).unwrap());
        // 存款满仓库：存入 金创药(背包槽2? 现在槽1空) -> 仓库槽0 与木剑互换
        // 先往背包加入 3 个金创药（合并到槽1）
        db.add_item_to_inventory(info.index, 3, 3).unwrap();
        // 取出：仓库槽1 -> 背包空槽
        assert!(db.take_item(info.index, 1, 3).unwrap());
        let inv = db.inventory_slots(info.index).unwrap();
        assert_eq!(inv[3].as_ref().map(|i| i.item_index), Some(3));
        let storage = db.storage_slots(info.index).unwrap();
        assert!(storage[1].is_none(), "取出后仓库槽1应空");
        assert_eq!(storage[0].as_ref().map(|i| i.item_index), Some(1)); // 木剑仍在
    }

    #[test]
    fn equipment_durability_and_repair() {
        let (db, _p) = temp_db();
        db.register("tester").unwrap();
        let info = db
            .add_character("tester", "检修", crystal_protocol::types::MirClass::Warrior, crystal_protocol::types::MirGender::Male)
            .unwrap()
            .unwrap();
        // 新角色木剑在背包槽0（grant_starter_items），穿戴到武器槽(slot0)
        let inv = db.inventory_slots(info.index).unwrap();
        let sword_uid = inv[0].as_ref().unwrap().unique_id;
        db.equip_item(info.index, sword_uid, 1, 0).unwrap();
        // 装备耐久满 = 模板 max_dura(20)
        let eq = db.load_equipment(info.index).unwrap();
        assert_eq!(eq.get(&0).map(|i| i.max_dura), Some(20));
        assert_eq!(eq.get(&0).map(|i| i.current_dura), Some(20));
        // 扣 5 耐久
        let (new_cd, _) = db.damage_equipment(info.index, 0, 5).unwrap();
        assert_eq!(new_cd, 15);
        assert_eq!(db.load_equipment(info.index).unwrap().get(&0).unwrap().current_dura, 15);
        // 修理费 = 缺失5
        let (cd, md, cost) = db.repair_item(info.index, sword_uid, 1).unwrap().unwrap();
        assert_eq!((cd, md, cost), (15, 20, 5));
        // 付款后实际维修
        assert!(db.apply_repair(info.index, sword_uid).unwrap());
        assert_eq!(db.load_equipment(info.index).unwrap().get(&0).unwrap().current_dura, 20);
        // 无耐久物不可修：金创药
        let medicine = db.inventory_slots(info.index).unwrap()[1].as_ref().unwrap().unique_id;
        assert!(db.repair_item(info.index, medicine, 1).unwrap().is_none());
    }

    #[test]
    fn friend_add_list_remove() {
        let (db, _p) = temp_db();
        db.register("tester").unwrap();
        let a = db
            .add_character("tester", "甲", crystal_protocol::types::MirClass::Warrior, crystal_protocol::types::MirGender::Male)
            .unwrap()
            .unwrap();
        let b = db
            .add_character("tester", "乙", crystal_protocol::types::MirClass::Warrior, crystal_protocol::types::MirGender::Male)
            .unwrap()
            .unwrap();
        // 甲加乙
        let (ok, fi) = db.add_friend(a.index, "乙", "", false).unwrap();
        assert!(ok);
        assert_eq!(fi, Some(b.index));
        let rows = db.friend_rows(a.index).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].1, "乙");
        // 重复加应失败
        assert!(!db.add_friend(a.index, "乙", "", false).unwrap().0);
        // 加不存在的角色失败
        assert!(!db.add_friend(a.index, "不存在", "", false).unwrap().0);
        // 名字查询
        assert_eq!(db.char_name(b.index).unwrap().as_deref(), Some("乙"));
        // 移除
        assert!(db.remove_friend(a.index, b.index).unwrap());
        assert!(db.friend_rows(a.index).unwrap().is_empty());
    }

    #[test]
    fn merge_inventory_items_combines_same_stack() {
        let (db, _p) = temp_db();
        db.register("tester").unwrap();
        let info = db
            .add_character("tester", "合并者", crystal_protocol::types::MirClass::Warrior, crystal_protocol::types::MirGender::Male)
            .unwrap()
            .unwrap();
        // 初始: 槽0=木剑x1, 槽1=金创药x5；加 6 个金创药 -> 槽1 = 11
        db.add_item_to_inventory(info.index, 3, 3).unwrap();
        db.add_item_to_inventory(info.index, 3, 3).unwrap();
        let slots = db.inventory_slots(info.index).unwrap();
        assert_eq!(slots[1].as_ref().map(|i| i.count), Some(11));
        // 从槽1 拆分 4 个到新空槽：得到两堆金创药
        let med_uid = slots[1].as_ref().unwrap().unique_id;
        let split_slot = db.split_inventory_item(info.index, med_uid, 4).unwrap().unwrap();
        let slots = db.inventory_slots(info.index).unwrap();
        let s1 = slots[1].as_ref().unwrap();
        let s2 = slots[split_slot as usize].as_ref().unwrap();
        assert_eq!(s1.count, 7);
        assert_eq!(s2.count, 4);
        assert_eq!(s1.item_index, s2.item_index);
        // 合并 4 到 7 -> 槽1 = 11，id_from 行删除
        let (ok, merged) = db.merge_inventory_items(info.index, s2.unique_id, s1.unique_id).unwrap();
        assert!(ok);
        assert_eq!(merged, 11);
        let slots = db.inventory_slots(info.index).unwrap();
        assert_eq!(slots[1].as_ref().map(|i| i.count), Some(11));
        assert_eq!(slots[1].as_ref().unwrap().unique_id, s1.unique_id);
        // 非同种物品不可合并（金创药 -> 木剑）
        let wood_uid = slots[0].as_ref().unwrap().unique_id;
        let med_uid = slots[1].as_ref().unwrap().unique_id;
        assert!(!db.merge_inventory_items(info.index, med_uid, wood_uid).unwrap().0);
    }

    #[test]
    fn storage_password_set_verify_clear() {
        let (db, _p) = temp_db();
        db.register("tester").unwrap();
        let c = db
            .add_character("tester", "密码甲", crystal_protocol::types::MirClass::Warrior, crystal_protocol::types::MirGender::Male)
            .unwrap()
            .unwrap()
            .index;
        // 初始未设
        assert!(db.get_storage_pw(c).is_none());
        // 设置
        let h = hash_password("1234");
        db.set_storage_pw(c, &h, 1000).unwrap();
        let got = db.get_storage_pw(c).unwrap();
        assert_eq!(got.0, h);
        assert_eq!(got.1, 1000);
        // 校验
        assert_eq!(hash_password("1234"), got.0);
        assert_ne!(hash_password("wrong"), got.0);
        // 更新
        let h2 = hash_password("5678");
        db.set_storage_pw(c, &h2, 2000).unwrap();
        assert_eq!(db.get_storage_pw(c).unwrap().0, h2);
        // 清除
        db.clear_storage_pw(c).unwrap();
        assert!(db.get_storage_pw(c).is_none());
    }

    #[test]
    fn mentor_link_query_clear() {
        let (db, _p) = temp_db();
        db.register("tester").unwrap();
        let m = db
            .add_character("tester", "师父甲", crystal_protocol::types::MirClass::Warrior, crystal_protocol::types::MirGender::Male)
            .unwrap()
            .unwrap()
            .index;
        let t1 = db
            .add_character("tester", "徒弟乙", crystal_protocol::types::MirClass::Warrior, crystal_protocol::types::MirGender::Male)
            .unwrap()
            .unwrap()
            .index;
        let t2 = db
            .add_character("tester", "徒弟丙", crystal_protocol::types::MirClass::Warrior, crystal_protocol::types::MirGender::Male)
            .unwrap()
            .unwrap()
            .index;
        // 建立师徒
        db.set_mentor(t1, m).unwrap();
        assert_eq!(db.get_mentor(t1), Some(m));
        db.set_mentor(t2, m).unwrap();
        // mentor 名下两个 mentee
        let mut mentees = db.mentees_of(m);
        mentees.sort();
        assert_eq!(mentees, vec![t1, t2]);
        // 清除一个徒弟
        db.clear_mentor(t1).unwrap();
        assert!(db.get_mentor(t1).is_none());
        assert_eq!(db.mentees_of(m), vec![t2]);
    }

    #[test]
    fn marriage_get_set_clear() {
        let (db, _p) = temp_db();
        db.register("tester").unwrap();
        let a = db
            .add_character("tester", "新郎甲", crystal_protocol::types::MirClass::Warrior, crystal_protocol::types::MirGender::Male)
            .unwrap()
            .unwrap()
            .index;
        let b = db
            .add_character("tester", "新娘乙", crystal_protocol::types::MirClass::Warrior, crystal_protocol::types::MirGender::Female)
            .unwrap()
            .unwrap()
            .index;
        assert!(db.get_spouse(a).is_none());
        db.set_spouse(a, b, 12345).unwrap();
        assert_eq!(db.get_spouse(a), Some(b));
        assert_eq!(db.get_spouse(b), Some(a));
        assert_eq!(db.marriage_date(a), 12345);
        // 双方离婚
        db.clear_spouse(a).unwrap();
        assert!(db.get_spouse(a).is_none());
        assert_eq!(db.get_spouse(b), Some(a)); // 配偶仍指向 a（离婚由双方各自清理）
    }

    #[test]
    fn reincarnation_increments_count() {
        let (db, _p) = temp_db();
        db.register("tester").unwrap();
        let c = db
            .add_character("tester", "转生者", crystal_protocol::types::MirClass::Warrior, crystal_protocol::types::MirGender::Male)
            .unwrap()
            .unwrap()
            .index;
        assert_eq!(db.reincarnations(c), 0);
        assert_eq!(db.add_reincarnation(c).unwrap(), 1);
        assert_eq!(db.add_reincarnation(c).unwrap(), 2);
        assert_eq!(db.reincarnations(c), 2);
    }

    #[test]
    fn next_refine_monotonic() {
        let (c0, cost0) = Database::next_refine(0);
        let (c1, cost1) = Database::next_refine(1);
        let (c5, cost5) = Database::next_refine(5);
        // 成功率随精炼值递减
        assert!(c0 > c1);
        assert!(c1 >= c5);
        // 成本递增
        assert!(cost0 < cost1);
        assert_eq!(cost0, 50);
        assert_eq!(cost1, 100);
        // 成功率收敛到 10% 下限
        assert!((c5 - 0.1).abs() < 1e-6);
        // 精炼值爬取写入/读取
        let (db, _p) = temp_db();
        db.register("tester").unwrap();
        let ci = db
            .add_character("tester", "精炼员", crystal_protocol::types::MirClass::Warrior, crystal_protocol::types::MirGender::Male)
            .unwrap()
            .unwrap()
            .index;
        // 初始背包槽0 有木剑，uid = ci*1000
        let uid = (ci as u64) * 1000;
        assert_eq!(db.read_refines("inventory", ci, uid), 0);
        db.set_refines("inventory", ci, uid, 3).unwrap();
        assert_eq!(db.read_refines("inventory", ci, uid), 3);
    }
}

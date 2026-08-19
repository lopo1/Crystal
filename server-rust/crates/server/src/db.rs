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
                PRIMARY KEY (character_index, slot)
            );
            CREATE TABLE IF NOT EXISTS equipment (
                character_index INTEGER NOT NULL,
                slot            INTEGER NOT NULL,
                unique_id       INTEGER NOT NULL,
                item_index      INTEGER NOT NULL,
                PRIMARY KEY (character_index, slot)
            );
            "#,
        )?;
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
            conn.prepare("SELECT slot,unique_id,item_index,count FROM inventory WHERE character_index=?1 ORDER BY slot")?;
        let rows = stmt.query_map([character_index], |r| {
            Ok((r.get::<_, i32>(0)?, r.get::<_, i64>(1)?, r.get::<_, i32>(2)?, r.get::<_, u16>(3)?))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (slot, uid, item_index, count) = row?;
            out.push((
                slot,
                UserItem {
                    unique_id: uid as u64,
                    item_index,
                    count,
                    ..Default::default()
                },
            ));
        }
        Ok(out)
    }

    /// 新角色自动发初始背包（木剑 + 金创药x5）
    pub fn grant_starter_items(&self, character_index: i32) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO inventory (character_index,slot,unique_id,item_index,count) VALUES (?1,0,?2,1,1)",
            params![character_index, make_unique(character_index, 0)],
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
                conn.execute(
                    "INSERT INTO inventory (character_index,slot,unique_id,item_index,count) VALUES (?1,?2,?3,?4,?5)",
                    params![
                        character_index,
                        slot,
                        make_unique(character_index, slot),
                        item_index,
                        count
                    ],
                )?;
                return Ok(true);
            }
        }
        Ok(false)
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
                "SELECT slot,item_index,count FROM inventory
                 WHERE character_index=?1 AND unique_id=?2",
                params![character_index, unique_id as i64],
                |r| {
                    Ok((
                        r.get::<_, i32>(0)?,
                        r.get::<_, i32>(1)?,
                        r.get::<_, u16>(2)?,
                    ))
                },
            )
            .optional()?;
        Ok(row.map(|(slot, item_index, count)| {
            (
                slot,
                UserItem {
                    unique_id,
                    item_index,
                    count,
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
            "SELECT slot,unique_id,item_index FROM equipment WHERE character_index=?1 ORDER BY slot",
        )?;
        let rows = stmt.query_map([character_index], |r| {
            Ok((r.get::<_, i32>(0)?, r.get::<_, i64>(1)?, r.get::<_, i32>(2)?))
        })?;
        let mut out = std::collections::BTreeMap::new();
        for row in rows {
            let (slot, uid, item_index) = row?;
            out.insert(
                slot,
                UserItem {
                    unique_id: uid as u64,
                    item_index,
                    count: 1,
                    ..Default::default()
                },
            );
        }
        Ok(out)
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
        let existing: Option<(i64, i32)> = conn
            .query_row(
                "SELECT unique_id,item_index FROM equipment
                 WHERE character_index=?1 AND slot=?2",
                params![character_index, equip_slot],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        if let Some((euid, eidx)) = existing {
            let Some(free) = free else {
                // 背包满，无法换下旧装备
                return Ok(EquipOutcome {
                    returned_to_inventory: false,
                });
            };
            conn.execute(
                "INSERT INTO inventory (character_index,slot,unique_id,item_index,count)
                 VALUES (?1,?2,?3,?4,1)",
                params![character_index, free, euid, eidx],
            )?;
            conn.execute(
                "DELETE FROM equipment WHERE character_index=?1 AND slot=?2",
                params![character_index, equip_slot],
            )?;
        }

        // 删除原背包格（正在穿戴的物品）
        conn.execute(
            "DELETE FROM inventory WHERE character_index=?1 AND unique_id=?2",
            params![character_index, unique_id as i64],
        )?;

        // 写入装备槽
        conn.execute(
            "INSERT INTO equipment (character_index,slot,unique_id,item_index)
             VALUES (?1,?2,?3,?4)",
            params![character_index, equip_slot, unique_id as i64, item_index],
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
        let existing: Option<(i64, i32)> = conn
            .query_row(
                "SELECT unique_id,item_index FROM equipment
                 WHERE character_index=?1 AND slot=?2",
                params![character_index, equip_slot],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let Some((euid, eidx)) = existing else {
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
            "INSERT INTO inventory (character_index,slot,unique_id,item_index,count)
             VALUES (?1,?2,?3,?4,1)",
            params![character_index, free, euid, eidx],
        )?;
        conn.execute(
            "DELETE FROM equipment WHERE character_index=?1 AND slot=?2",
            params![character_index, equip_slot],
        )?;
        Ok(Some((euid as u64, eidx)))
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

fn hash_password(s: &str) -> String {
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
}

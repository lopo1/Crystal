//! 内存账户/角色存储（垂直切片）。
//!
//! 后续替换为 SQLite 持久化（阶段 2），并接入钱包登录（阶段 3）。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crystal_protocol::types::{MirClass, MirGender, SelectInfo};

#[derive(Debug, Clone)]
pub struct Character {
    pub index: i32,
    pub name: String,
    pub class: MirClass,
    pub gender: MirGender,
    pub level: u16,
}

#[derive(Debug, Clone, Default)]
pub struct Account {
    pub pass_hash: String,
    pub characters: Vec<Character>,
}

#[derive(Debug, Clone)]
pub struct AccountStore {
    accounts: Arc<Mutex<HashMap<String, Account>>>,
    next_char_index: Arc<Mutex<i32>>,
}

impl AccountStore {
    pub fn new() -> Self {
        AccountStore {
            accounts: Arc::new(Mutex::new(HashMap::new())),
            next_char_index: Arc::new(Mutex::new(0)),
        }
    }

    pub fn register(&self, account_id: &str) -> bool {
        let mut accounts = self.accounts.lock().unwrap();
        if accounts.contains_key(account_id) {
            return false;
        }
        accounts.insert(account_id.to_string(), Account::default());
        true
    }

    pub fn login(&self, account_id: &str) -> bool {
        self.accounts.lock().unwrap().contains_key(account_id)
    }

    pub fn select_infos(&self, account_id: &str) -> Vec<SelectInfo> {
        let accounts = self.accounts.lock().unwrap();
        let mut out = Vec::new();
        if let Some(acc) = accounts.get(account_id) {
            for c in &acc.characters {
                out.push(SelectInfo {
                    index: c.index,
                    name: c.name.clone(),
                    level: c.level,
                    class: c.class,
                    gender: c.gender,
                    last_access: 0,
                });
            }
        }
        out
    }

    pub fn add_character(
        &self,
        account_id: &str,
        name: &str,
        class: MirClass,
        gender: MirGender,
    ) -> Result<SelectInfo, u8> {
        let mut accounts = self.accounts.lock().unwrap();
        let acc = accounts.get_mut(account_id).ok_or(3u8)?; // 未登录
        if acc.characters.len() >= 4 {
            return Err(4); // 角色满
        }
        if acc.characters.iter().any(|c| c.name == name) {
            return Err(5); // 角色已存在
        }
        let mut idx = self.next_char_index.lock().unwrap();
        let index = *idx;
        *idx += 1;
        let index = index + 1; // 1 起
        acc.characters.push(Character {
            index,
            name: name.to_string(),
            class,
            gender,
            level: 1,
        });
        Ok(SelectInfo {
            index,
            name: name.to_string(),
            level: 1,
            class,
            gender,
            last_access: 0,
        })
    }

    pub fn delete_character(&self, account_id: &str, character_index: i32) -> bool {
        let mut accounts = self.accounts.lock().unwrap();
        let Some(acc) = accounts.get_mut(account_id) else {
            return false;
        };
        let before = acc.characters.len();
        acc.characters.retain(|c| c.index != character_index);
        acc.characters.len() != before
    }

    pub fn get_character(&self, account_id: &str, character_index: i32) -> Option<Character> {
        let accounts = self.accounts.lock().unwrap();
        accounts
            .get(account_id)?
            .characters
            .iter()
            .find(|c| c.index == character_index)
            .cloned()
    }
}

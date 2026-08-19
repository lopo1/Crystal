//! 自建程序化法术库（替代缺失的原版 Magic.DB / 数据库文件）。
//!
//! 垂直切片阶段定义少量基础法术，后续可扩展 per-class 法术树。
#![allow(dead_code)]

use crystal_protocol::types::ClientMagic;

/// 法术模板
#[derive(Debug, Clone, Copy)]
pub struct MagicTemplate {
    /// 法术编号（对应客户端 Attack.spell / Spell 枚举）
    pub spell: u8,
    pub name: &'static str,
    /// 基础 MP 消耗
    pub base_cost: u8,
    /// 射程（格）
    pub range: u8,
    /// 基础伤害
    pub damage: i32,
    /// 冷却（tick，约 400ms/tick）
    pub cooldown: u8,
}

/// 内置法术表（spell -> 模板）
pub fn magics() -> Vec<MagicTemplate> {
    vec![
        MagicTemplate {
            spell: 1,
            name: "火球术",
            base_cost: 5,
            range: 20,
            damage: 15,
            cooldown: 3,
        },
        MagicTemplate {
            spell: 2,
            name: "雷电术",
            base_cost: 8,
            range: 25,
            damage: 22,
            cooldown: 4,
        },
    ]
}

pub fn find(spell: u8) -> Option<MagicTemplate> {
    magics().into_iter().find(|m| m.spell == spell)
}

/// 玩家可用的法术列表（垂直切片：全员赠送火球术，法师多学雷电术）。
pub fn player_magics(class: crystal_protocol::types::MirClass) -> Vec<ClientMagic> {
    let all = magics();
    let allowed: Vec<&MagicTemplate> = all
        .iter()
        .filter(|m| m.spell == 1 || class == crystal_protocol::types::MirClass::Wizard)
        .collect();
    allowed
        .iter()
        .map(|m| ClientMagic {
            name: m.name.to_string(),
            spell: m.spell,
            base_cost: m.base_cost,
            level_cost: 0,
            icon: m.spell,
            level1: 1,
            level2: 1,
            level3: 1,
            need1: 1,
            need2: 1,
            need3: 1,
            level: 1,
            key: m.spell,
            experience: 0,
            delay: 0,
            range: m.range,
            cast_time: 0,
        })
        .collect()
}

/// 玩家是否已学会该法术（垂直切片直接按职业返回 true）
pub fn knows_magic(class: crystal_protocol::types::MirClass, spell: u8) -> bool {
    if find(spell).is_none() {
        return false;
    }
    spell == 1 || class == crystal_protocol::types::MirClass::Wizard
}

/// 法术是否有足够 MP 施放
pub fn can_cast(template: &MagicTemplate, mp: i32) -> bool {
    mp >= template.base_cost as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crystal_protocol::types::MirClass;

    #[test]
    fn fireball_exists_and_has_range() {
        let fb = find(1).unwrap();
        assert_eq!(fb.name, "火球术");
        assert!(fb.range >= 1);
        assert!(fb.damage > 0);
        assert!(fb.base_cost > 0);
        assert!(find(99).is_none());
    }

    #[test]
    fn knows_magic_by_class() {
        // 火球术所有人都会；雷电术仅法师
        assert!(knows_magic(MirClass::Warrior, 1));
        assert!(!knows_magic(MirClass::Warrior, 2));
        assert!(knows_magic(MirClass::Wizard, 2));
        assert!(!knows_magic(MirClass::Wizard, 99));
    }

    #[test]
    fn player_magics_by_class() {
        let w = player_magics(MirClass::Warrior);
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].spell, 1);
        let m = player_magics(MirClass::Wizard);
        assert_eq!(m.len(), 2);
    }

    #[test]
    fn can_cast_mp_check() {
        let fb = find(1).unwrap();
        assert!(!can_cast(&fb, 3));
        assert!(can_cast(&fb, fb.base_cost as i32));
        assert!(can_cast(&fb, 999));
    }
}

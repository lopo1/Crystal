//! 刷怪点配置（数据化）—— 阶段2 世界配置。
//!
//! 把原本内嵌在 seed_world 里的「怪物模板 + 每张地图刷怪点」抽成一份集中式结构数据，
//! 作为刷怪的唯一数据源。seed_world 据此生成实际怪物。
//!
//! 结构（与硬编码期保持等价）：
//! - 3 种怪物模板（稻草人/骷髅/蜘蛛）
//! - 地图 0 出生点附近 20 个刷怪点
//! - 地图 100 与 101 各 4 个刷怪点
//! - 每图刷怪点可随意扩充：只改这里即可，不必再动战斗/生成逻辑。

/// 怪物模板
#[derive(Debug, Clone, Copy)]
pub struct MonsterTemplate {
    pub image: u16,
    pub name: &'static str,
    pub level: u16,
    pub hp: i32,
    pub attack: i32,
    pub defence: i32,
    pub exp: u32,
    pub gold: u32,
    /// true = 远程攻击怪（射程内发射弹体，ObjectRangeAttack 表现）
    pub ranged: bool,
    /// 采集所得物品 item_index（0=不可采集；Harvest 割肉）
    pub harvest_item: i32,
}

/// 全部怪物模板
pub const MONSTER_TEMPLATES: [MonsterTemplate; 3] = [
    MonsterTemplate { image: 2, name: "稻草人", level: 1, hp: 12, attack: 1, defence: 0, exp: 5, gold: 3, ranged: false, harvest_item: 0 },
    MonsterTemplate { image: 3, name: "骷髅", level: 3, hp: 20, attack: 3, defence: 1, exp: 12, gold: 8, ranged: false, harvest_item: 6 },
    MonsterTemplate { image: 4, name: "蜘蛛", level: 4, hp: 26, attack: 5, defence: 2, exp: 20, gold: 12, ranged: true, harvest_item: 0 },
];

/// 一张地图的刷怪点列表
#[derive(Debug, Clone)]
pub struct MapSpawn {
    pub map_index: u32,
    pub points: &'static [(i32, i32)],
}

/// 全部刷怪配置：按地图归类
pub static SPAWN_CONFIG: &[MapSpawn] = &[
    // 地图 0：新手村出生点附近连续开阔地紧凑布怪（保证测试可达）
    MapSpawn {
        map_index: 0,
        points: &[
            (400, 400), (403, 402), (406, 400), (401, 405), (405, 405),
            (398, 403), (407, 403), (403, 399), (399, 407), (406, 407),
            (402, 402), (404, 398), (397, 401), (408, 405), (401, 399),
            (407, 401), (404, 408), (399, 404), (406, 405), (402, 406),
        ],
    },
    MapSpawn { map_index: 100, points: &[(8, 5), (9, 6), (11, 7), (12, 8)] },
    MapSpawn { map_index: 101, points: &[(10, 14), (12, 15), (13, 12), (11, 11)] },
];

/// 取某地图的刷怪点（无则空）。
pub fn spawn_points_for(map_index: u32) -> &'static [(i32, i32)] {
    SPAWN_CONFIG
        .iter()
        .find(|m| m.map_index == map_index)
        .map(|m| m.points)
        .unwrap_or(&[])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_has_expected_maps_and_points() {
        assert_eq!(spawn_points_for(0).len(), 20);
        assert_eq!(spawn_points_for(100).len(), 4);
        assert_eq!(spawn_points_for(101).len(), 4);
        assert!(spawn_points_for(999).is_empty());
    }

    #[test]
    fn templates_make_valid_monsters() {
        assert_eq!(MONSTER_TEMPLATES.len(), 3);
        for t in MONSTER_TEMPLATES {
            assert!(t.hp > 0 && t.attack >= 0 && t.exp > 0);
        }
    }

    #[test]
    fn each_spawn_config_has_unique_map() {
        let mut maps: Vec<u32> = SPAWN_CONFIG.iter().map(|m| m.map_index).collect();
        maps.sort();
        maps.dedup();
        assert_eq!(maps.len(), SPAWN_CONFIG.len());
    }
}

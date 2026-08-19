//! 自建程序化物品库（替代缺失的原版 Item.DB / 数据库文件）。
//!
//! 这里定义垂直切片演示用的少量物品模板。后续可用代码或 `.db` 文件扩展。
#![allow(dead_code)]

/// 物品模板
#[derive(Debug, Clone, Copy)]
pub struct ItemTemplate {
    pub index: i32,
    pub name: &'static str,
    /// 0=杂物 1=武器 2=消耗品 3=护甲
    pub item_type: u8,
    pub price: u32,
    pub image: u16,
    /// 攻击加成（武器）/ 防御加成（护甲）
    pub bonus: i32,
    /// 使用回复 HP 量（消耗品）
    pub heal: i32,
}

/// 内置物品表（index -> 模板）
pub fn items() -> Vec<ItemTemplate> {
    vec![
        ItemTemplate {
            index: 1,
            name: "木剑",
            item_type: 1,
            price: 20,
            image: 100,
            bonus: 2,
            heal: 0,
        },
        ItemTemplate {
            index: 2,
            name: "布衣",
            item_type: 3,
            price: 30,
            image: 101,
            bonus: 1,
            heal: 0,
        },
        ItemTemplate {
            index: 3,
            name: "金创药",
            item_type: 2,
            price: 10,
            image: 120,
            bonus: 0,
            heal: 30,
        },
        ItemTemplate {
            index: 4,
            name: "回城卷",
            item_type: 2,
            price: 25,
            image: 121,
            bonus: 0,
            heal: 0,
        },
        ItemTemplate {
            index: 5,
            name: "铜钱袋",
            item_type: 0,
            price: 15,
            image: 130,
            bonus: 0,
            heal: 0,
        },
    ]
}

pub fn find(index: i32) -> Option<ItemTemplate> {
    items().into_iter().find(|i| i.index == index)
}

/// 是否有该物品在库中（买卖校验用）
pub fn exists(index: i32) -> bool {
    find(index).is_some()
}

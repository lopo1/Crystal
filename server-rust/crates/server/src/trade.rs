//! 交易（Trade）管理 —— 阶段2 社交玩法。
//!
//! 以「玩家名」为参与方标识（独立于 TCP 连接），纯逻辑可单测。
//! 流程: 发起方请求 -> 对方接受 -> 双方放入金币/物品 -> 双方确认(锁定) ->
//!       全部确认后完成 -> 返回结算清单(由调用方用 DB/世界执行转移)。
//!
//! 结算返回的是「要把物品 unique_id 与金币移动的方向」，不直接触碰 DB，
//! 由 net.rs 承载执行（db 转移物品所有权 / world 增减金币）。

use std::collections::HashMap;

/// 一方放入的交易物品
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TradeSide {
    /// 放入的背包物品 unique_id 列表
    pub items: Vec<u64>,
    /// 放入的金币
    pub gold: u32,
}

/// 一场进行中的交易
#[derive(Debug, Clone)]
pub struct Trade {
    pub a: String,
    pub b: String,
    pub side_a: TradeSide,
    pub side_b: TradeSide,
    /// 各自是否已确认（锁定）
    pub lock_a: bool,
    pub lock_b: bool,
}

/// 结算清单：把 a 的物品/金币给 b，b 的物品/金币给 a
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Settle {
    pub a: String,
    pub b: String,
    pub a_items_to_b: Vec<u64>,
    pub b_items_to_a: Vec<u64>,
    pub a_gold_to_b: u32,
    pub b_gold_to_a: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TradeError {
    NoPendingInvite,
    NotInTrade,
    AlreadyLocked,
    /// 物品已在交易栏中（重复放入）
    AlreadyDeposited,
}

/// 交易管理器
#[derive(Debug, Default, Clone)]
pub struct TradeManager {
    /// 待接受邀请: 被邀者名 -> 发起者名
    pending: HashMap<String, String>,
    /// 进行中的交易（A,B 各一场）
    active: Vec<Trade>,
}

impl TradeManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// 发起交易请求（from 邀请 to）。
    /// 返回 false 表示对方已有待处理的交易邀请（同 C# TradeInvitation 占用检查）。
    pub fn request(&mut self, from: &str, to: &str) -> bool {
        if from == to || self.pending.contains_key(to) {
            return false;
        }
        self.pending.insert(to.to_string(), from.to_string());
        true
    }

    /// 拒绝邀请：移除待处理项，返回发起者名（用于通知）。
    pub fn reject(&mut self, invitee: &str) -> Option<String> {
        self.pending.remove(invitee)
    }

    /// 对方接受：从 pending 创建一场交易
    pub fn accept(&mut self, invitee: &str) -> Result<(), TradeError> {
        let inviter = self.pending.remove(invitee).ok_or(TradeError::NoPendingInvite)?;
        self.active.push(Trade {
            a: inviter,      // 发起者
            b: invitee.to_string(),
            side_a: TradeSide::default(),
            side_b: TradeSide::default(),
            lock_a: false,
            lock_b: false,
        });
        Ok(())
    }

    fn find(&mut self, who: &str) -> Option<&mut Trade> {
        self.active.iter_mut().find(|t| t.a == who || t.b == who)
    }

    /// 放入金币。任一方改动都会解锁双方（同 C# TradeUnlock 语义）。
    pub fn add_gold(&mut self, who: &str, amount: u32) -> Result<(), TradeError> {
        let t = self.find(who).ok_or(TradeError::NotInTrade)?;
        let me = if t.a == who { &mut t.side_a } else { &mut t.side_b };
        me.gold += amount;
        t.lock_a = false;
        t.lock_b = false;
        Ok(())
    }

    /// 放入一件背包物品（unique_id）。任一方改动都会解锁双方。
    pub fn add_item(&mut self, who: &str, unique_id: u64) -> Result<(), TradeError> {
        let t = self.find(who).ok_or(TradeError::NotInTrade)?;
        let me = if t.a == who { &mut t.side_a } else { &mut t.side_b };
        if me.items.contains(&unique_id) {
            return Err(TradeError::AlreadyDeposited);
        }
        me.items.push(unique_id);
        t.lock_a = false;
        t.lock_b = false;
        Ok(())
    }

    /// 取回一件已放入的物品（按放入顺序的下标）。返回被取回的 unique_id。
    pub fn remove_item_at(&mut self, who: &str, index: usize) -> Option<u64> {
        let t = self.find(who)?;
        let me = if t.a == who { &mut t.side_a } else { &mut t.side_b };
        if index >= me.items.len() {
            return None;
        }
        let uid = me.items.remove(index);
        t.lock_a = false;
        t.lock_b = false;
        Some(uid)
    }

    /// 我方已放入的金币总额
    pub fn side_gold(&self, who: &str) -> u32 {
        self.active
            .iter()
            .find(|t| t.a == who || t.b == who)
            .map(|t| if t.a == who { t.side_a.gold } else { t.side_b.gold })
            .unwrap_or(0)
    }

    /// 我方已放入的物品 unique_id 列表（按放入顺序）
    pub fn side_items(&self, who: &str) -> Vec<u64> {
        self.active
            .iter()
            .find(|t| t.a == who || t.b == who)
            .map(|t| if t.a == who { t.side_a.items.clone() } else { t.side_b.items.clone() })
            .unwrap_or_default()
    }

    /// 确认（锁定本方）
    pub fn confirm(&mut self, who: &str) -> Result<(), TradeError> {
        let t = self.find(who).ok_or(TradeError::NotInTrade)?;
        if t.a == who {
            t.lock_a = true;
        } else {
            t.lock_b = true;
        }
        Ok(())
    }

    /// 取消交易（发起方或任一方）。返回是否成功。
    pub fn cancel(&mut self, who: &str) -> bool {
        if let Some(pos) = self.active.iter().position(|t| t.a == who || t.b == who) {
            self.active.remove(pos);
            true
        } else {
            false
        }
    }

    /// 双方都确认则完成，返回结算清单；否则返回 None（未完成）。
    pub fn complete(&mut self, who: &str) -> Result<Option<Settle>, TradeError> {
        let pos = self
            .active
            .iter()
            .position(|t| t.a == who || t.b == who)
            .ok_or(TradeError::NotInTrade)?;
        let trade = &self.active[pos];
        if !trade.lock_a || !trade.lock_b {
            return Ok(None);
        }
        // 双方已确认 -> 结算
        let settle = Settle {
            a: trade.a.clone(),
            b: trade.b.clone(),
            a_items_to_b: trade.side_a.items.clone(),
            b_items_to_a: trade.side_b.items.clone(),
            a_gold_to_b: trade.side_a.gold,
            b_gold_to_a: trade.side_b.gold,
        };
        self.active.remove(pos);
        Ok(Some(settle))
    }

    /// 某玩家是否在交易中
    pub fn in_trade(&self, who: &str) -> bool {
        self.active.iter().any(|t| t.a == who || t.b == who)
    }

    /// 某玩家当前交易对象的名字（不在交易中则 None）
    pub fn active_partner_of(&self, who: &str) -> Option<String> {
        self.active
            .iter()
            .find(|t| t.a == who || t.b == who)
            .map(|t| if t.a == who { t.b.clone() } else { t.a.clone() })
    }

    /// 仅解锁自己（对应 C# TradeConfirm(false)）
    pub fn confirm_cancel(&mut self, who: &str) -> Result<(), TradeError> {
        let t = self.find(who).ok_or(TradeError::NotInTrade)?;
        if t.a == who {
            t.lock_a = false;
        } else {
            t.lock_b = false;
        }
        Ok(())
    }

    /// 双方是否都已确认锁定
    pub fn both_locked(&self, who: &str) -> bool {
        self.active
            .iter()
            .find(|t| t.a == who || t.b == who)
            .map(|t| t.lock_a && t.lock_b)
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_accept_full_flow() {
        let mut mgr = TradeManager::new();
        mgr.request("A", "B");
        assert!(mgr.accept("B").is_ok());
        assert!(mgr.in_trade("A") && mgr.in_trade("B"));
        // 放金币/物品
        mgr.add_gold("A", 100).unwrap();
        mgr.add_item("A", 11).unwrap();
        mgr.add_item("B", 22).unwrap();
        // 未确认 -> 未完成
        assert!(mgr.complete("A").unwrap().is_none());
        // A 确认，B 未确认 -> 仍未完成
        mgr.confirm("A").unwrap();
        assert!(mgr.complete("A").unwrap().is_none());
        // B 确认 -> 完成并结算
        mgr.confirm("B").unwrap();
        let settle = mgr.complete("A").unwrap().expect("完成");
        assert_eq!(settle.a_items_to_b, vec![11]);
        assert_eq!(settle.b_items_to_a, vec![22]);
        assert_eq!(settle.a_gold_to_b, 100);
        assert_eq!(settle.b_gold_to_a, 0);
        // 交易已移除
        assert!(!mgr.in_trade("A") && !mgr.in_trade("B"));
    }

    #[test]
    fn accept_without_invite_fails() {
        let mut mgr = TradeManager::new();
        assert_eq!(mgr.accept("X"), Err(TradeError::NoPendingInvite));
    }

    #[test]
    fn cancel_removes_trade() {
        let mut mgr = TradeManager::new();
        mgr.request("A", "B");
        mgr.accept("B").unwrap();
        assert!(mgr.cancel("A"));
        assert!(!mgr.in_trade("B"));
        assert!(!mgr.cancel("A")); // 已无交易
    }

    #[test]
    fn locked_side_cannot_add_in_place_but_can_confirm() {
        let mut mgr = TradeManager::new();
        mgr.request("A", "B");
        mgr.accept("B").unwrap();
        mgr.confirm("A").unwrap();
        // A 已锁定，仍可继续放（简化：不阻止；确认是单向的）
        mgr.add_item("A", 99).unwrap();
        mgr.confirm("A").unwrap();
        mgr.confirm("B").unwrap();
        let s = mgr.complete("B").unwrap().unwrap();
        assert_eq!(s.a_items_to_b, vec![99]);
    }

    #[test]
    fn request_fails_when_target_busy_or_self() {
        let mut mgr = TradeManager::new();
        assert!(mgr.request("A", "B"));
        // B 已有邀请：C 再约 B 失败
        assert!(!mgr.request("C", "B"));
        // 自己约自己失败
        assert!(!mgr.request("A", "A"));
    }

    #[test]
    fn reject_returns_inviter_and_clears_pending() {
        let mut mgr = TradeManager::new();
        mgr.request("A", "B");
        assert_eq!(mgr.reject("B"), Some("A".to_string()));
        assert_eq!(mgr.reject("B"), None);
        assert_eq!(mgr.accept("B"), Err(TradeError::NoPendingInvite));
    }

    #[test]
    fn side_views_and_remove_item_at() {
        let mut mgr = TradeManager::new();
        mgr.request("A", "B");
        mgr.accept("B").unwrap();
        mgr.add_gold("A", 50).unwrap();
        mgr.add_item("A", 11).unwrap();
        mgr.add_item("A", 12).unwrap();
        assert_eq!(mgr.side_gold("A"), 50);
        assert_eq!(mgr.side_items("A"), vec![11, 12]);
        assert_eq!(mgr.side_items("B"), Vec::<u64>::new());
        // 取回下标 0 的物品
        assert_eq!(mgr.remove_item_at("A", 0), Some(11));
        assert_eq!(mgr.side_items("A"), vec![12]);
        // 下标越界
        assert_eq!(mgr.remove_item_at("A", 5), None);
        // 重复放入被拒
        assert_eq!(mgr.add_item("A", 12), Err(TradeError::AlreadyDeposited));
    }

    #[test]
    fn change_unlocks_both_and_partner_lookup() {
        let mut mgr = TradeManager::new();
        mgr.request("A", "B");
        mgr.accept("B").unwrap();
        assert_eq!(mgr.active_partner_of("A"), Some("B".to_string()));
        assert_eq!(mgr.active_partner_of("B"), Some("A".to_string()));
        assert_eq!(mgr.active_partner_of("C"), None);
        mgr.confirm("A").unwrap();
        mgr.confirm("B").unwrap();
        assert!(mgr.both_locked("A"));
        // 任一方改动 -> 双方解锁
        mgr.add_gold("B", 10).unwrap();
        assert!(!mgr.both_locked("A"));
    }
}

//! 市场（Market）管理 —— 阶段2 社交经济。
//!
//! 纯逻辑、可单测。卖家挂单（物品 unique_id → 指定金币价），买家按低价成交，
//! 卖家可撤销。成交/撤销由调用方（net.rs）用 db/world 执行物品与金币转移。
//!
//! 每个挂单都有全局唯一 order_id（单调递增）。内存态，重启即清空（后续可持久化）。

use std::collections::HashMap;

/// 一个挂单
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarketOrder {
    pub order_id: u64,
    /// 卖家名
    pub seller: String,
    /// 出售的物品 unique_id
    pub item_uid: u64,
    /// 要价（金币）
    pub price: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketError {
    InvalidPrice,
    OrderNotFound,
}

/// 市场管理器
#[derive(Debug, Default, Clone)]
pub struct MarketManager {
    next_id: u64,
    /// order_id -> 挂单
    pub orders: HashMap<u64, MarketOrder>,
}

impl MarketManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// 卖家挂单：出售 unique_id 物品，要价 price。返回 order_id。
    pub fn list(&mut self, seller: &str, item_uid: u64, price: u32) -> Result<u64, MarketError> {
        if price == 0 {
            return Err(MarketError::InvalidPrice);
        }
        self.next_id += 1;
        let order_id = self.next_id;
        self.orders.insert(
            order_id,
            MarketOrder {
                order_id,
                seller: seller.to_string(),
                item_uid,
                price,
            },
        );
        Ok(order_id)
    }

    /// 查看市场上所有挂单。
    pub fn all_orders(&self) -> Vec<MarketOrder> {
        self.orders.values().cloned().collect()
    }

    /// 买家按价购买某挂单（从对象中移除，返回订单；由调用方执行物品+金币转移）。
    pub fn buy(&mut self, order_id: u64) -> Result<MarketOrder, MarketError> {
        self.orders.remove(&order_id).ok_or(MarketError::OrderNotFound)
    }

    /// 卖家撤销挂单（拿回物品）。
    pub fn cancel(&mut self, order_id: u64) -> Result<MarketOrder, MarketError> {
        self.orders.remove(&order_id).ok_or(MarketError::OrderNotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_buy_and_cancel() {
        let mut mgr = MarketManager::new();
        let id = mgr.list("A", 1234, 500).unwrap();
        assert_eq!(mgr.all_orders().len(), 1);
        // 买入
        let order = mgr.buy(id).unwrap();
        assert_eq!(order.seller, "A");
        assert_eq!(order.item_uid, 1234);
        assert_eq!(order.price, 500);
        // 已移除
        assert!(mgr.orders.is_empty());
        // 买不存在的
        assert_eq!(mgr.buy(id), Err(MarketError::OrderNotFound));
    }

    #[test]
    fn zero_price_rejected() {
        let mut mgr = MarketManager::new();
        assert_eq!(mgr.list("A", 1, 0), Err(MarketError::InvalidPrice));
    }

    #[test]
    fn order_ids_are_unique() {
        let mut mgr = MarketManager::new();
        let a = mgr.list("A", 1, 10).unwrap();
        let b = mgr.list("B", 2, 20).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn cancel_returns_order() {
        let mut mgr = MarketManager::new();
        let id = mgr.list("A", 99, 50).unwrap();
        let o = mgr.cancel(id).unwrap();
        assert_eq!(o.seller, "A");
        assert!(mgr.orders.is_empty());
    }
}

//! 组队（Group）管理 —— 阶段2 多人社交的核心。
//!
//! 以「玩家名」为成员标识（独立于 TCP 连接），便于单元测试与协议转发。
//! 队伍结构: `groups[gid] = [队长, ...成员]`，首元素为队长；`member_of` 反向索引；
//! `pending_invites` 记录待处理邀请。

use std::collections::HashMap;

/// 组队管理器（纯逻辑，无 IO，可单元测试）
#[derive(Debug, Default, Clone)]
pub struct GroupManager {
    /// 玩家名 -> 队伍ID
    pub member_of: HashMap<String, u32>,
    /// 待处理邀请: 被邀请玩家名 -> (发起者, 队伍ID)
    pub pending_invites: HashMap<String, (String, u32)>,
    /// 队伍ID -> 成员名列表（首元素为队长）
    pub groups: HashMap<u32, Vec<String>>,
    next_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupError {
    /// 邀请目标已在队伍中
    AlreadyInGroup,
    /// 该玩家没有待处理的邀请
    NoInvite,
    /// 不是队伍成员 / 非队长操作
    NotMember,
    GroupNotFound,
}

impl GroupManager {
    pub fn new() -> Self {
        Self::default()
    }

    fn next_id(&mut self) -> u32 {
        self.next_id += 1;
        self.next_id
    }

    /// 创建队伍（队长入队），返回队伍 ID。
    pub fn create(&mut self, leader: &str) -> u32 {
        // 若已在队伍，先离开旧队
        let _ = self.leave_group_for(leader);
        let id = self.next_id();
        self.groups.insert(id, vec![leader.to_string()]);
        self.member_of.insert(leader.to_string(), id);
        id
    }

    /// 队长把玩家从旧队伍踢出（若在队），供 add/del 使用。
    fn leave_group_for(&mut self, member: &str) -> bool {
        let Some(gid) = self.member_of.get(member).copied() else { return false };
        let is_leader = self
            .groups
            .get(&gid)
            .map(|m| m.first().map(|s| s.as_str()) == Some(member))
            .unwrap_or(false);
        let members = self.groups.get_mut(&gid).unwrap();
        members.retain(|m| m != member);
        self.member_of.remove(member);
        if is_leader || members.is_empty() {
            let rest = members.clone();
            self.groups.remove(&gid);
            for m in rest {
                self.member_of.remove(&m);
            }
        }
        true
    }

    /// 邀请玩家加入队长的队伍。仅队长可邀请，目标须不在任何队伍。
    pub fn invite(&mut self, host: &str, target: &str) -> Result<u32, GroupError> {
        if target == host {
            return Err(GroupError::NotMember);
        }
        let gid = *self.member_of.get(host).ok_or(GroupError::NotMember)?;
        let members = self.groups.get(&gid).ok_or(GroupError::GroupNotFound)?;
        if members.first().map(|s| s.as_str()) != Some(host) {
            return Err(GroupError::NotMember); // 仅队长
        }
        if self.member_of.contains_key(target) {
            return Err(GroupError::AlreadyInGroup);
        }
        self.pending_invites
            .insert(target.to_string(), (host.to_string(), gid));
        Ok(gid)
    }

    /// 被邀请者接受邀请，加入队伍，返回成员列表。
    pub fn accept(&mut self, invitee: &str) -> Result<Vec<String>, GroupError> {
        let (_, gid) = self.pending_invites.remove(invitee).ok_or(GroupError::NoInvite)?;
        // 万一受邀期间已入他队
        if self.member_of.contains_key(invitee) {
            return Err(GroupError::AlreadyInGroup);
        }
        let members = self.groups.get_mut(&gid).ok_or(GroupError::GroupNotFound)?;
        members.push(invitee.to_string());
        self.member_of.insert(invitee.to_string(), gid);
        Ok(members.clone())
    }

    /// 拒绝邀请（清除待处理邀请）。
    pub fn decline(&mut self, invitee: &str) -> bool {
        self.pending_invites.remove(invitee).is_some()
    }

    /// 离开队伍；若队长离开或队伍空，则解散。
    pub fn leave(&mut self, member: &str) -> Result<Vec<String>, GroupError> {
        let gid = *self.member_of.get(member).ok_or(GroupError::NotMember)?;
        let is_leader = self
            .groups
            .get(&gid)
            .map(|m| m.first().map(|s| s.as_str()) == Some(member))
            .unwrap_or(false);
        {
            let members = self.groups.get_mut(&gid).unwrap();
            members.retain(|m| m != member);
        }
        self.member_of.remove(member);
        if is_leader || self.groups.get(&gid).map(|m| m.is_empty()).unwrap_or(true) {
            // 解散：清除剩余成员的归属
            let rest = self.groups.remove(&gid).unwrap_or_default();
            for m in rest {
                self.member_of.remove(&m);
            }
            Ok(vec![])
        } else {
            Ok(self.groups.get(&gid).cloned().unwrap_or_default())
        }
    }

    /// 某玩家所在队伍 ID。
    pub fn group_of(&self, member: &str) -> Option<u32> {
        self.member_of.get(member).copied()
    }

    /// 队伍成员列表。
    pub fn members(&self, gid: u32) -> Vec<String> {
        self.groups.get(&gid).cloned().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_invite_accept_flow() {
        let mut mgr = GroupManager::new();
        let gid = mgr.create("A");
        assert!(mgr.group_of("A").is_some());
        // A 邀请 B
        assert!(mgr.invite("A", "B").is_ok());
        assert_eq!(mgr.group_of("B"), None, "接受前 B 尚未入队");
        // 非队长不能邀请
        let gid2 = mgr.create("C");
        let _ = gid2;
        // B 接受后入队
        let members = mgr.accept("B").unwrap();
        assert_eq!(members, vec!["A".to_string(), "B".to_string()]);
        assert_eq!(mgr.group_of("B"), Some(gid));
        // B 属于队伍，不能再被邀请
        assert_eq!(mgr.invite("A", "B"), Err(GroupError::AlreadyInGroup));
    }

    #[test]
    fn leader_leave_disbands_group() {
        let mut mgr = GroupManager::new();
        let gid = mgr.create("A");
        mgr.invite("A", "B").unwrap();
        mgr.accept("B").unwrap();
        // A 离开 -> 解散
        let members = mgr.leave("A").unwrap();
        assert!(members.is_empty());
        assert!(mgr.group_of("B").is_none(), "解散后 B 也应脱离队伍");
        assert_eq!(mgr.group_of("A"), None);
        let _ = gid;
    }

    #[test]
    fn member_leave_keeps_group() {
        let mut mgr = GroupManager::new();
        mgr.create("A");
        mgr.invite("A", "B").unwrap();
        mgr.accept("B").unwrap();
        // B 离开 -> 队伍保留 A
        let members = mgr.leave("B").unwrap();
        assert_eq!(members, vec!["A".to_string()]);
        assert!(mgr.group_of("B").is_none());
        assert!(mgr.group_of("A").is_some());
    }

    #[test]
    fn errors_for_bad_cases() {
        let mut mgr = GroupManager::new();
        mgr.create("A");
        // 无队伍者不能邀请
        assert_eq!(mgr.invite("X", "B"), Err(GroupError::NotMember));
        // 无邀请者接受报错
        assert_eq!(mgr.accept("B"), Err(GroupError::NoInvite));
        // 拒绝不存在的邀请返回 false
        assert!(!mgr.decline("nobody"));
    }
}

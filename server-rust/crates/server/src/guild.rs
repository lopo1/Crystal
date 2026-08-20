//! 公会（Guild）管理 —— 阶段2 社交玩法。
//!
//! 纯逻辑、以「公会名 + 成员名」为标识，独立于 TCP 连接，可单元测试。
//! 一个玩家只能属于一个公会；创建者即会长；解散/最后一次成员离开则删除公会。

use std::collections::HashMap;

/// 公会信息
#[derive(Debug, Clone)]
pub struct Guild {
    pub name: String,
    pub owner: String,
    /// 成员名列表（含会长）
    pub members: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuildError {
    GuildExists,
    AlreadyInGuild,
    NotInGuild,
    NotOwner,
}

/// 公会管理器
#[derive(Debug, Default, Clone)]
pub struct GuildManager {
    /// 玩家名 -> 公会名
    pub member_guild: HashMap<String, String>,
    /// 公会名 -> 公会
    pub guilds: HashMap<String, Guild>,
}

impl GuildManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// 创建公会（创建者为会长）。返回公会名。
    pub fn create(&mut self, name: &str, owner: &str) -> Result<String, GuildError> {
        let name = name.to_string();
        if self.guilds.contains_key(&name) {
            return Err(GuildError::GuildExists);
        }
        // 创建者若已在其他公会，先离开
        self.leave(owner);
        self.member_guild.insert(owner.to_string(), name.clone());
        self.guilds.insert(
            name.clone(),
            Guild {
                name: name.clone(),
                owner: owner.to_string(),
                members: vec![owner.to_string()],
            },
        );
        Ok(name)
    }

    /// 加入公会（通过公会名；简化：直接加入）。返回公会名。
    pub fn join(&mut self, guild: &str, member: &str) -> Result<String, GuildError> {
        if !self.guilds.contains_key(guild) {
            return Err(GuildError::GuildExists); // 复用为"公会不存在"
        }
        if self.member_guild.contains_key(member) {
            return Err(GuildError::AlreadyInGuild);
        }
        self.guilds.get_mut(guild).unwrap().members.push(member.to_string());
        self.member_guild.insert(member.to_string(), guild.to_string());
        Ok(guild.to_string())
    }

    /// 玩家所在公会。
    pub fn guild_of(&self, member: &str) -> Option<&Guild> {
        let g = self.member_guild.get(member)?;
        self.guilds.get(g)
    }

    /// 公会成员列表。
    pub fn members(&self, guild: &str) -> Option<Vec<String>> {
        self.guilds.get(guild).map(|g| g.members.clone())
    }

    /// 移除成员；若被移除的是会长或公会空，则解散公会。
    /// 返回 (removed_guild, remaining_members)。
    pub fn remove_member(&mut self, guild: &str, member: &str) -> Option<(String, Vec<String>)> {
        let is_owner = self
            .guilds
            .get(guild)
            .map(|g| g.owner == member)
            .unwrap_or(false);
        let g = self.guilds.get_mut(guild)?;
        g.members.retain(|m| m != member);
        self.member_guild.remove(member);
        let remaining = g.members.clone();
        if is_owner || remaining.is_empty() {
            let g = self.guilds.remove(guild)?;
            for m in &g.members {
                self.member_guild.remove(m);
            }
            Some((g.name, vec![]))
        } else {
            Some((guild.to_string(), remaining))
        }
    }

    /// 玩家离开公会（相当于移除自己）。
    pub fn leave(&mut self, member: &str) -> bool {
        if let Some(g) = self.guild_of(member).map(|g| g.name.clone()) {
            self.remove_member(&g, member);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_join_members() {
        let mut mgr = GuildManager::new();
        mgr.create("华夏", "A").unwrap();
        assert_eq!(mgr.members("华夏"), Some(vec!["A".to_string()]));
        assert!(mgr.join("华夏", "B").is_ok());
        assert_eq!(mgr.members("华夏"), Some(vec!["A".to_string(), "B".to_string()]));
        assert_eq!(mgr.guild_of("B").unwrap().owner, "A");
        // 已在公会不能重复加入
        assert_eq!(mgr.join("华夏", "B"), Err(GuildError::AlreadyInGuild));
    }

    #[test]
    fn duplicate_guild_name_fails() {
        let mut mgr = GuildManager::new();
        mgr.create("华夏", "A").unwrap();
        assert_eq!(mgr.create("华夏", "X"), Err(GuildError::GuildExists));
    }

    #[test]
    fn owner_leave_disbands() {
        let mut mgr = GuildManager::new();
        mgr.create("华夏", "A").unwrap();
        mgr.join("华夏", "B").unwrap();
        assert!(mgr.leave("A"));
        assert!(mgr.guild_of("B").is_none(), "解散后 B 脱离公会");
        assert!(mgr.guild_of("A").is_none());
        assert!(mgr.guilds.is_empty());
    }

    #[test]
    fn member_leave_keeps_guild() {
        let mut mgr = GuildManager::new();
        mgr.create("华夏", "A").unwrap();
        mgr.join("华夏", "B").unwrap();
        assert!(mgr.leave("B"));
        assert_eq!(mgr.members("华夏"), Some(vec!["A".to_string()]));
        assert!(mgr.guild_of("A").is_some());
        assert!(mgr.remove_member("华夏", "A").is_some());
        assert!(mgr.guilds.is_empty());
    }
}

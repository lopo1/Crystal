//! 地图系统：解析 Crystal `.map` 文件（C# 自定义格式，V100），提供可通行碰撞网格。
//!
//! 数据来源：`Suprcode/Crystal.Database` 的 `Jev/Maps/*.map`（复制到 `data/maps/`）。
//!
//! ## V100 文件格式（对应原 C# `Map.LoadMapCellsV100`）
//! ```text
//! bytes[0]   = 版本号（1）
//! bytes[1]   = 0
//! bytes[2..4]= 签名 'C' '#'（0x43 0x23）
//! offset=4   : Width   (i16 LE)
//! offset=6   : Height  (i16 LE)
//! offset=8   : 每个格子 26 字节（W*H 依次, x 外 y 内）:
//!   +2 跳过
//!   +2..+6  i32: 0x20000000 置位 => HighWall（阻挡通行，可远程越过）
//!   +10..+12 i16: 0x8000 置位 => LowWall（阻挡通行且不可远程越过）
//!   +12..+14 后,+2 处一字节: >0 表示门
//!   +25      后一个字节: 光照
//! 文件大小 = 8 + W*H*26
//! ```
//! 可通行 = 既非 HighWall 也非 LowWall。

use std::path::Path;

/// 一张已加载的游戏地图（含碰撞网格）
#[derive(Debug, Clone)]
pub struct GameMap {
    pub index: u32,
    pub width: u32,
    pub height: u32,
    /// 每格 1 字节：1=可通行，0=被阻挡
    walkable: Vec<u8>,
}

impl GameMap {
    fn idx(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return None;
        }
        Some((y as usize) * self.width as usize + x as usize)
    }

    /// 该坐标是否在地图内
    pub fn contains(&self, x: i32, y: i32) -> bool {
        self.idx(x, y).is_some()
    }

    /// 该坐标是否可通行（在地图内且为 walkable）
    pub fn is_walkable(&self, x: i32, y: i32) -> bool {
        match self.idx(x, y) {
            Some(i) => self.walkable[i] == 1,
            None => false,
        }
    }
}

/// 解析 V100 格式的 `.map` 文件。
pub fn load_map_file(index: u32, path: impl AsRef<Path>) -> anyhow::Result<GameMap> {
    let bytes = std::fs::read(path.as_ref())?;
    let mut g = load_map_bytes(index, &bytes)?;
    // 计算可用性统计（供日志/测试）
    let walk = g.walkable.iter().filter(|&&b| b == 1).count();
    tracing::info!(
        "地图 {index} 加载成功 {}x{}，可通行 {} / {}",
        g.width,
        g.height,
        walk,
        g.walkable.len()
    );
    let _ = &mut g;
    Ok(g)
}

/// 从字节解析地图（供单元测试直接调用）。自动识别两种格式：
/// - V100（C# 自定义，`[1,0,'C','#']`）：`8 + W*H*26`
/// - v1（wemade "Map 2010 Ver 1.0"）：`54 + W*H*15`，宽高与 xor 异或
pub fn load_map_bytes(index: u32, bytes: &[u8]) -> anyhow::Result<GameMap> {
    if bytes.len() < 8 {
        anyhow::bail!("地图数据过短");
    }
    // V100 特征：版本1 + 签名 C#
    if bytes[0] == 1 && bytes[1] == 0 && bytes[2] == 0x43 && bytes[3] == 0x23 {
        return load_v100(index, bytes);
    }
    // v1 特征：title 首字节长度 + "Map 2010 ..."（对应 C# FindType==1 的偏移检测）
    if bytes[0] == 0x10 && bytes.len() > 2 && bytes[2] == 0x61 {
        return load_v1(index, bytes);
    }
    anyhow::bail!("不支持的 .map 格式: {:02x?}", &bytes[..8])
}

/// V100 格式解析（C# LoadMapCellsV100）
fn load_v100(index: u32, bytes: &[u8]) -> anyhow::Result<GameMap> {
    let width = u16::from_le_bytes([bytes[4], bytes[5]]) as u32;
    let height = u16::from_le_bytes([bytes[6], bytes[7]]) as u32;
    let expected = 8usize + (width as usize) * (height as usize) * 26;
    if bytes.len() < expected {
        anyhow::bail!("地图数据长度不符: 期望 >= {expected}，实际 {}", bytes.len());
    }
    let mut walkable = vec![1u8; (width as usize) * (height as usize)];
    let mut offset = 8usize;
    for y in 0..height as usize {
        for x in 0..width as usize {
            offset += 2;
            let hi = (u32::from_le_bytes([bytes[offset], bytes[offset + 1], bytes[offset + 2], bytes[offset + 3]])
                & 0x2000_0000)
                != 0;
            offset += 10;
            let lo = (u16::from_le_bytes([bytes[offset], bytes[offset + 1]]) & 0x8000) != 0;
            offset += 2;
            offset += 11;
            offset += 1;
            if hi || lo {
                walkable[y * width as usize + x] = 0;
            }
        }
    }
    Ok(GameMap { index, width, height, walkable })
}

/// v1 wemade 2010 格式解析（C# LoadMapCellsv1）
fn load_v1(index: u32, bytes: &[u8]) -> anyhow::Result<GameMap> {
    let w = i16::from_le_bytes([bytes[21], bytes[22]]);
    let xor = i16::from_le_bytes([bytes[23], bytes[24]]);
    let h = i16::from_le_bytes([bytes[25], bytes[26]]);
    let width = (w ^ xor) as u32;
    let height = (h ^ xor) as u32;
    if width == 0 || height == 0 || width > 10000 || height > 10000 {
        anyhow::bail!("[v1] 非法尺寸 {width}x{height}");
    }
    let expected = 54usize + (width as usize) * (height as usize) * 15;
    if bytes.len() < expected {
        anyhow::bail!("[v1] 地图数据长度不符: 期望 >= {expected}，实际 {}", bytes.len());
    }
    let xormask = (xor as u32) & 0xFFFF;
    let mut walkable = vec![1u8; (width as usize) * (height as usize)];
    let mut offset = 54usize;
    for y in 0..height as usize {
        for x in 0..width as usize {
            let cell = u32::from_le_bytes([bytes[offset], bytes[offset + 1], bytes[offset + 2], bytes[offset + 3]]);
            let hi = (cell ^ 0xAA38_AA38) & 0x2000_0000 != 0;
            offset += 6;
            let lw = (u16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as u32 ^ xormask) & 0x8000 != 0;
            offset += 2;
            offset += 5; // 门字节
            offset += 1; // 光照
            offset += 1;
            if hi || lw {
                walkable[y * width as usize + x] = 0;
            }
        }
    }
    Ok(GameMap { index, width, height, walkable })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造一张 4x4 全通的测试地图字节
    fn fake_map_bytes(w: u16, h: u16) -> Vec<u8> {
        let mut b = vec![0u8; 8 + (w as usize) * (h as usize) * 26];
        b[0] = 1;
        b[2] = 0x43;
        b[3] = 0x23;
        b[4..6].copy_from_slice(&w.to_le_bytes());
        b[6..8].copy_from_slice(&h.to_le_bytes());
        b
    }

    #[test]
    fn parses_header_and_all_walkable() {
        let m = load_map_bytes(0, &fake_map_bytes(700, 700)).unwrap();
        assert_eq!(m.width, 700);
        assert_eq!(m.height, 700);
        assert!(m.is_walkable(0, 0));
        assert!(m.is_walkable(699, 699));
        assert!(!m.contains(700, 0));
    }

    #[test]
    fn rejects_bad_signature() {
        let mut b = fake_map_bytes(10, 10);
        b[2] = 0x00;
        assert!(load_map_bytes(0, &b).is_err());
    }

    #[test]
    fn marks_highwall_as_blocked() {
        let mut b = fake_map_bytes(4, 4);
        // 第 (0,0) 格：offset 8；highwall i32 在 offset+2
        let cell = 8usize;
        let hi_i32 = cell + 2;
        // 置位 0x20000000
        let orig = u32::from_le_bytes([b[hi_i32], b[hi_i32 + 1], b[hi_i32 + 2], b[hi_i32 + 3]]);
        let v = orig | 0x2000_0000;
        b[hi_i32..hi_i32 + 4].copy_from_slice(&v.to_le_bytes());
        let m = load_map_bytes(0, &b).unwrap();
        assert!(!m.is_walkable(0, 0));
        assert!(m.is_walkable(1, 0));
    }

    #[test]
    fn rejects_truncated_file() {
        let b = vec![1, 0, 0x43, 0x23, 0xff, 0xff, 0xff, 0xff, 0]; // 0000s but short
        assert!(load_map_bytes(0, &b).is_err());
    }

    /// 真实地图数据集成测试：当 data/maps/0.map 存在时，验证可正确解析且碰撞生效。
    #[test]
    fn loads_real_map_data() {
        let path = "data/maps/0.map";
        let Ok(bytes) = std::fs::read(path) else {
            eprintln!("跳过：缺少 {path}");
            return;
        };
        let m = load_map_bytes(0, &bytes).unwrap();
        assert_eq!(m.width, 700);
        assert_eq!(m.height, 700);
        // 可通行格子约占三分之二（与上游数据一致），且既非全通也非全堵
        let walk = m.walkable.iter().filter(|&&b| b == 1).count();
        let total = m.walkable.len();
        assert!(walk > total / 2 && walk < total);
        // 出生点(400,400)可行走
        assert!(m.is_walkable(400, 400));
        // 存在不可行走格（地图有墙）
        let blocked = m.walkable.iter().filter(|&&b| b == 0).count();
        assert!(blocked > 0);
    }

    /// 构造一张 v1 全通地图字节：所有格 hi = 0xAA38AA38、lo = xor（XOR 后均 0 => 可通行）。
    fn fake_v1(w: u16, h: u16, xor: u16) -> Vec<u8> {
        let mut b = vec![0u8; (54 + w as usize * h as usize * 15) as usize];
        b[0] = 0x10;
        b[1] = 0x4d;
        b[2] = 0x61;
        b[21..23].copy_from_slice(&((w as u16) ^ xor).to_le_bytes());
        b[23..25].copy_from_slice(&xor.to_le_bytes());
        b[25..27].copy_from_slice(&((h as u16) ^ xor).to_le_bytes());
        let mut off = 54usize;
        for _ in 0..(w as usize * h as usize) {
            b[off..off + 4].copy_from_slice(&0xAA38_AA38u32.to_le_bytes());
            b[off + 6..off + 8].copy_from_slice(&xor.to_le_bytes());
            off += 15;
        }
        b
    }

    /// v1（wemade 2010）格式解析测试：构造 4x3 全通 v1 地图。
    #[test]
    fn parses_v1_wemade_map() {
        let b = fake_v1(4, 3, 0x1234);
        let m = load_map_bytes(9, &b).unwrap();
        assert_eq!((m.width, m.height), (4, 3));
        assert!(m.is_walkable(0, 0));
        assert!(m.is_walkable(3, 2));
    }

    /// v1 格式墙阻挡校验。
    #[test]
    fn v1_marks_wall_blocked() {
        let mut b = fake_v1(3, 3, 0x100);
        // 中心格 (1,1) index=4, cell offset=54+4*15；置其 hi 位为 0x20000000（XOR 用）
        let off = 54 + 4 * 15;
        // 要让 (cell ^ 0xAA38AA38) & 0x20000000 != 0：cell = 0xAA38AA38 ^ 0x20000000
        let cell = 0xAA38_AA38u32 ^ 0x2000_0000;
        b[off..off + 4].copy_from_slice(&cell.to_le_bytes());
        let m = load_map_bytes(9, &b).unwrap();
        assert!(!m.is_walkable(1, 1));
        assert!(m.is_walkable(0, 0));
    }

    /// 真实 v1 地图集成测试：data/maps/0100.map 若存在则解析。
    #[test]
    fn loads_real_v1_map_data() {
        let paths = ["data/maps/0100.map", "data/maps/0101.map"];
        for path in paths {
            let Ok(bytes) = std::fs::read(path) else {
                eprintln!("跳过：缺少 {path}");
                continue;
            };
            let m = load_map_bytes(100, &bytes).unwrap();
            assert!(m.width >= 1 && m.height >= 1);
            // 必存在墙，也有可行走格
            let walk = m.walkable.iter().filter(|&&b| b == 1).count();
            let blocked = m.walkable.iter().filter(|&&b| b == 0).count();
            assert!(walk > 0 && blocked > 0);
        }
    }
}

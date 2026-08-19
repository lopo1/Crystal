#!/usr/bin/env bash
# 获取 Crystal 地图数据（供 Rust 服务器使用）。
#
# 数据来源: https://github.com/Suprcode/Crystal.Database 的 Jev/Maps/*.map
# 这些是 C# 自定义格式(.map, V100)的二进制碰撞数据，本仓库不直接纳入版本控制，
# 由该脚本下载/复制到 server-rust/data/maps/（已在 .gitignore 中排除）。
#
# 用法:
#   ./scripts/get_maps.sh                     # 自动 git clone（浅）并复制 0.map
#   MAP_SRC=/path/to/Crystal.Database ./scripts/get_maps.sh   # 复用本地已克隆仓库
#
# 缺省只复制 0.map（新手村）；设置 COPIES=<逗号分隔> 或 ALL_MAPS=1 可复制更多。

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${ROOT}/server-rust/data/maps"
mkdir -p "$DEST"

# 需要的地图编号（默认 0 = 新手村）
COPIES="${COPIES:-0}"
ALL_MAPS="${ALL_MAPS:-0}"

WORK=""
cleanup() { [ -n "$WORK" ] && rm -rf "$WORK"; }
trap cleanup EXIT

if [ -n "${MAP_SRC:-}" ]; then
  echo "使用本地地图仓库: $MAP_SRC"
  # 兼容两种入参：仓库根目录（Jev/Maps）或直接是地图目录
  if [ -d "$MAP_SRC/Jev/Maps" ]; then
    SRC="$MAP_SRC/Jev/Maps"
  else
    SRC="$MAP_SRC"
  fi
else
  WORK="$(mktemp -d)"
  echo "浅克隆 Crystal.Database ..."
  git clone --depth 1 --filter=blob:none --sparse \
    https://github.com/Suprcode/Crystal.Database.git "$WORK/db" \
    || git clone --depth 1 https://github.com/Suprcode/Crystal.Database.git "$WORK/db"
  cd "$WORK/db"
  git sparse-checkout set Jev/Maps 2>/dev/null || true
  SRC="$WORK/db/Jev/Maps"
fi

if [ ! -d "$SRC" ]; then
  echo "错误: 找不到地图目录 $SRC" >&2
  exit 1
fi

if [ "$ALL_MAPS" = "1" ]; then
  echo "复制全部地图 ..."
  cp "$SRC"/*.map "$DEST/"
else
  for n in $(echo "$COPIES" | tr ',' ' '); do
    f="$SRC/${n}.map"
    if [ -f "$f" ]; then
      cp "$f" "$DEST/"
      echo "已复制 $n.map $(du -h "$f" | cut -f1)"
    else
      echo "警告: 找不到 $f" >&2
    fi
  done
fi

echo "地图已就绪: $DEST"
ls -1 "$DEST" | head

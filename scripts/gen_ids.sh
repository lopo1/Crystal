#!/usr/bin/env bash
# 从原 Crystal Shared/Enums.cs 重新生成 Rust 数据包 ID 枚举 (ids.rs)
# 用法: scripts/gen_ids.sh
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ENUMS="$ROOT/Shared/Enums.cs"

if [ ! -f "$ENUMS" ]; then
  echo "错误: 未找到 ${ENUMS}（原 Crystal Shared 层）" >&2
  exit 1
fi

python3 - "$ENUMS" "$ROOT/server-rust/crates/protocol/src/ids.rs" <<'PY'
import sys
import re

def parse(path, enum_name):
    items = []
    with open(path, encoding='utf-8-sig') as f:
        lines = f.readlines()
    start = next(i for i, l in enumerate(lines) if f'public enum {enum_name}' in l)
    i = start
    while '{' not in lines[i]:
        i += 1
    i += 1
    n = 0
    while i < len(lines):
        line = lines[i].strip().rstrip(',').strip()
        if line == '}':
            break
        if line and not line.startswith('//'):
            items.append((line, n))
            n += 1
        i += 1
    return items

def emit(rust_name, items):
    out = [f'pub enum {rust_name} {{']
    for name, i in items:
        out.append(f'    {name} = {i},')
    out.append('}')
    out.append(f'impl {rust_name} {{')
    out.append(f'    pub fn from_i16(v: i16) -> Option<Self> {{')
    out.append(f'        use {rust_name}::*;')
    out.append('        Some(match v {')
    for name, i in items:
        out.append(f'            {i} => {name},')
    out.append('            _ => return None,')
    out.append('        })')
    out.append('    }')
    out.append('}')
    return '\n'.join(out)

enums_path, out_path = sys.argv[1], sys.argv[2]
client = parse(enums_path, 'ClientPacketIds')
server = parse(enums_path, 'ServerPacketIds')

with open(out_path, 'w') as f:
    f.write('//! 数据包 ID 枚举 —— 由原 Crystal `Shared/Enums.cs` 的枚举顺序生成。\n')
    f.write('//! 与 C# 侧一一对应，禁止手改序号；重新生成: `scripts/gen_ids.sh`。\n\n')
    f.write('#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]\n#[repr(i16)]\n')
    f.write(emit('ClientPacketId', client))
    f.write('\n\n#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]\n#[repr(i16)]\n')
    f.write(emit('ServerPacketId', server))
print(f'client: {len(client)}, server: {len(server)} -> {out_path}')
PY
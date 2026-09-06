#!/usr/bin/env bash
set -euo pipefail

MAX_LOC=200
FAILED=0

ALLOWLIST=(
    "src/shared/wayland/adapters/wayland.rs"
    "src/shared/scripting/adapters/lua.rs"
    "src/shared/scripting/adapters/rhai.rs"
    "src/shared/config/domain.rs"
    "src/shared/rendering/adapters/tiny_skia.rs"
    "src/shared/config/adapters/dto.rs"
    "src/shared/primitives/mod.rs"
    "src/shared/events/signals.rs"
    "src/shared/dbus/adapters/connection.rs"
    "src/shared/wayland/adapters/shm.rs"
    "src/shared/primitives/color.rs"
    "src/features/mpris/adapters/zbus.rs"
    "src/shared/dbus/domain.rs"
)

is_allowlisted() {
    local target="$1"
    for allowed in "${ALLOWLIST[@]}"; do
        if [[ "$target" == "$allowed" ]]; then
            return 0
        fi
    done
    return 1
}

count_prod_loc() {
    local file="$1"
    python3 -c "
import sys
path = sys.argv[1]
with open(path, 'r', encoding='utf-8', errors='ignore') as f:
    lines = f.readlines()

in_test = False
brace_depth = 0
prod_lines = 0

for line in lines:
    stripped = line.strip()
    if stripped.startswith('#[cfg(test)]'):
        in_test = True
        brace_depth = line.count('{') - line.count('}')
        continue
    if in_test:
        brace_depth += line.count('{') - line.count('}')
        if brace_depth <= 0 and ('}' in line or ';' in line):
            in_test = False
        continue
    prod_lines += 1

print(prod_lines)
" "$file"
}

echo "=== Checking Production File LOC Limits (Max ${MAX_LOC} LOC) ==="

while IFS= read -r file; do
    prod_loc=$(count_prod_loc "$file")
    if (( prod_loc > MAX_LOC )); then
        if is_allowlisted "$file"; then
            echo "  [ALLOWLISTED] $file ($prod_loc LOC > $MAX_LOC)"
        else
            echo "  [FAILED] $file ($prod_loc LOC > $MAX_LOC)"
            FAILED=1
        fi
    fi
done < <(find src/features src/shared -type f -name "*.rs" | sort)

if (( FAILED != 0 )); then
    echo ""
    echo "ERROR: One or more files exceed the ${MAX_LOC} production LOC limit!"
    exit 1
else
    echo "All non-allowlisted files satisfy the ${MAX_LOC} production LOC limit."
fi

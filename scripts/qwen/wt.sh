#!/bin/sh
# wt.sh <testfilter> [clippy]: checks in a copy (does not collide with running Qwen runs)
cd "$(dirname "$0")/../.." || exit 1
python3 scripts/qwen/workcopy.py
cd /tmp/gen/work || exit 1
cargo fmt -p snapious-core 2>/dev/null
cargo test -p snapious-core "$1" 2>&1 | grep -E "^error|test result: .* [0-9]+ passed|panicked|assertion" -A7 | head -40
if [ "$2" = "clippy" ]; then cargo clippy -p snapious-core --all-targets 2>&1 | grep -B3 -A10 "$3" | grep -E "^warning|^error|-->" | head; fi

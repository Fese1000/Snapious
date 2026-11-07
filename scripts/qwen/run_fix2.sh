#!/bin/bash
cd "$(dirname "$0")"
while pgrep -f run_fix_phase1.sh >/dev/null; do sleep 10; done
python3 run_card.py view/map_layout.rs view::map_layout --note "REVISION, second attempt. File is implemented, 3 tests red: (1) position_at_point(0.0,0.0) must return 0 (atan2(0,-0.0)=π): first, if x == 0.0 && y == 0.0 { return 0 }. (2)+(3) tick_step: for exp in 0..: for mult in [1,2,5]: step = mult * 10usize.saturating_pow(exp); if length <= step.saturating_mul(max(max_ticks,1)) return step; on overflow (step == usize::MAX) also return it. At least 1. Clippy: this file contains a negated comparison on f64 several times (e.g. '!(width > 0.0)' in LinearMapLayout::new and another place further up); write it positively instead, e.g. 'width <= 0.0 || width.is_nan()'. Do not change the tests."
echo FIX2_FERTIG

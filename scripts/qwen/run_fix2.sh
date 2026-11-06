#!/bin/bash
cd "$(dirname "$0")"
while pgrep -f run_fix_phase1.sh >/dev/null; do sleep 10; done
python3 run_card.py view/map_layout.rs view::map_layout --note "ÜBERARBEITUNG, zweiter Anlauf. Datei ist implementiert, 3 Tests rot: (1) position_at_point(0.0,0.0) muss 0 liefern (atan2(0,-0.0)=π): vorab if x == 0.0 && y == 0.0 { return 0 }. (2)+(3) tick_step: für exp in 0..: für mult in [1,2,5]: step = mult * 10usize.saturating_pow(exp); wenn length <= step.saturating_mul(max(max_ticks,1)) gib step zurück; bei Überlauf (step == usize::MAX) ebenfalls zurückgeben. Mindestens 1. Clippy: in dieser Datei steht mehrfach ein negierter Vergleich auf f64 (z. B. '!(width > 0.0)' in LinearMapLayout::new und eine weitere Stelle weiter oben); schreibe stattdessen positiv, z. B. 'width <= 0.0 || width.is_nan()'. Tests nicht ändern."
echo FIX2_FERTIG

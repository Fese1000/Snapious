#!/bin/bash
cd "$(dirname "$0")"
P=run_card.py
python3 $P view/map_layout.rs view::map_layout --note "REVISION. The file is already implemented, three tests fail: (1) kreis_rueckwaerts: position_at_point(0.0, 0.0) must return 0, but atan2(0, -0.0) gives π; handle x==0 and y==0 first. (2) schrittweite and (3) skalenstriche: tick_step is wrong (e.g. tick_step(100,10) returns 100 instead of 10). Just rewrite it: for exp in 0.. and mult in [1,2,5]: step = mult*10^exp; return the first step for which length/step <= max_ticks (integer: length <= step*max_ticks; max_ticks 0 counts as 1; overflow with saturating_mul). Also Clippy: line 87 uses a negated comparison on f64, write it positively or with partial_cmp. Do not change the tests."
python3 $P view/enzyme_map.rs view::enzyme_map cloning/enzymes.rs --note "REVISION: only fix Clippy (line 83: variable does not need to be mut). Change nothing else."
python3 $P view/feature_layout.rs view::feature_layout model/feature.rs --note "REVISION: only fix Clippy (line 117: sort_by_key instead of sort_by). Change nothing else."
python3 $P view/selection.rs view::selection --note "REVISION: only fix Clippy (line 93: simplify match to unwrap_or_default). Change nothing else."
python3 $P view/translation.rs view::translation model/feature.rs model/sequence.rs cloning/enzymes.rs --note "REVISION: only fix Clippy: line 69 doc list item without indentation (the doc comment may be reformatted for this), line 121 unnecessary cast usize->usize. Change nothing else."
echo ALLE_FERTIG

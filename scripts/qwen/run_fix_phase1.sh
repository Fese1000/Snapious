#!/bin/bash
cd "$(dirname "$0")"
P=run_card.py
python3 $P view/map_layout.rs view::map_layout --note "ÜBERARBEITUNG. Die Datei ist schon implementiert, drei Tests schlagen fehl: (1) kreis_rueckwaerts: position_at_point(0.0, 0.0) muss 0 liefern, atan2(0, -0.0) ergibt aber π; behandle x==0 und y==0 vorab. (2) schrittweite und (3) skalenstriche: tick_step ist falsch (z. B. tick_step(100,10) liefert 100 statt 10). Schreibe es einfach neu: für exp in 0.. und mult in [1,2,5]: step = mult*10^exp; gib das erste step zurück, für das length/step <= max_ticks (ganzzahlig: length <= step*max_ticks; max_ticks 0 zählt als 1; Überlauf mit saturating_mul). Außerdem Clippy: Zeile 87 nutzt negierten Vergleich auf f64, schreibe ihn positiv oder mit partial_cmp. Die Tests nicht ändern."
python3 $P view/enzyme_map.rs view::enzyme_map cloning/enzymes.rs --note "ÜBERARBEITUNG: nur Clippy beheben (Zeile 83: Variable muss nicht mut sein). Sonst nichts ändern."
python3 $P view/feature_layout.rs view::feature_layout model/feature.rs --note "ÜBERARBEITUNG: nur Clippy beheben (Zeile 117: sort_by_key statt sort_by). Sonst nichts ändern."
python3 $P view/selection.rs view::selection --note "ÜBERARBEITUNG: nur Clippy beheben (Zeile 93: match durch unwrap_or_default vereinfachen). Sonst nichts ändern."
python3 $P view/translation.rs view::translation model/feature.rs model/sequence.rs cloning/enzymes.rs --note "ÜBERARBEITUNG: nur Clippy beheben: Zeile 69 Doc-Listenpunkt ohne Einrückung (Doc-Kommentar darf dafür umformatiert werden), Zeile 121 unnötiger Cast usize->usize. Sonst nichts ändern."
echo ALLE_FERTIG

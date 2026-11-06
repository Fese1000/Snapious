#!/bin/bash
cd "$(dirname "$0")"
P=run_card.py
python3 $P view/selection.rs view::selection
python3 $P view/feature_layout.rs view::feature_layout model/feature.rs
python3 $P view/translation.rs view::translation model/feature.rs model/sequence.rs cloning/enzymes.rs
python3 $P view/selection_stats.rs view::selection_stats primer/tm.rs cloning/enzymes.rs
python3 $P view/map_layout.rs view::map_layout
python3 $P view/enzyme_map.rs view::enzyme_map cloning/enzymes.rs model/document.rs model/sequence.rs
python3 $P view/edit_session.rs view::edit_session model/document.rs model/feature.rs
echo ALLE_FERTIG

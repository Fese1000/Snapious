#!/bin/bash
# Arbeitet queue.txt Zeile für Zeile ab. Zeilenformat: LOGDATEI|run_card-Argumente (Hinweis mit --note "..." am Ende).
# Erledigte Zeilen wandern nach queue_done.txt. Das Skript läuft, bis STOP existiert.
cd "$(dirname "$0")"
touch queue.txt queue_done.txt
while [ ! -f STOP ]; do
  if pgrep -f "run_card.py" >/dev/null; then sleep 15; continue; fi
  LINE=$(head -n 1 queue.txt)
  if [ -z "$LINE" ]; then sleep 20; continue; fi
  sed -i '' '1d' queue.txt
  LOG=${LINE%%|*}; ARGS=${LINE#*|}
  eval "python3 run_card.py $ARGS" > "$LOG" 2>&1
  echo "$(date +%H:%M) $LOG: $(grep ERGEBNIS "$LOG" | tail -1)" >> queue_done.txt
done

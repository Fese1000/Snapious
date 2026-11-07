#!/bin/bash
# Works through queue.txt line by line. Line format: LOGFILE|run_card arguments (hint with --note "..." at the end).
# Finished lines move to queue_done.txt. The script runs until STOP exists.
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

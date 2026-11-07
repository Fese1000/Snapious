#!/usr/bin/env python3
"""workcopy.py: copies the core sources to /tmp/gen/work (separate target directory) so checks do not collide with running Qwen runs.
The file Qwen is currently working on is replaced there by its skeleton."""
import os, re, subprocess, shutil
ROOT=os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
W="/tmp/gen/work"
os.makedirs(W,exist_ok=True)
subprocess.run(["rsync","-a","--delete","--exclude","target","--exclude","app","--exclude","build","--exclude","src/main","--exclude","src/test","--exclude","scripts","--exclude","docs","--exclude",".git",f"{ROOT}/Cargo.toml",f"{ROOT}/Cargo.lock",f"{ROOT}/crates",W+"/"],check=True)
ps=subprocess.run(["pgrep","-fa","run_card.py"],capture_output=True,text=True).stdout
for line in ps.splitlines():
    m=re.search(r"run_card\.py (\S+\.rs)",line)
    if m:
        rel=m.group(1); sk=f"{ROOT}/scripts/qwen/logs/{os.path.basename(rel)}.skeleton"
        if os.path.exists(sk):
            shutil.copy(sk,f"{W}/crates/snapious-core/src/{rel}")
            print("Skeleton inserted for",rel)

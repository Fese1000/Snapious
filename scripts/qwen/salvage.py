#!/usr/bin/env python3
"""salvage.py <ziel.rs relativ zu crates/snapious-core/src> <testfilter>: probiert die gespeicherten Qwen-Antworten durch
(neueste zuerst; abgeschnittene Antworten zählen bis zum Abbruch), setzt die erste mit grünen Tests ein, sonst die mit den
wenigsten Fehlern. Das Gerüst (Tests) stammt aus logs/<datei>.skeleton."""
import os, re, subprocess, sys
ROOT=os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
rel,flt=sys.argv[1],sys.argv[2]
name=os.path.basename(rel); LOG=f"{ROOT}/scripts/qwen/logs"
W="/tmp/gen/work"
work=f"{W}/crates/snapious-core/src/{rel}"
target=f"{ROOT}/crates/snapious-core/src/{rel}"
sk=open(f"{LOG}/{name}.skeleton").read()
tests=sk[sk.find("#[cfg(test)]"):]
subprocess.run([sys.executable,f"{ROOT}/scripts/qwen/workcopy.py"],check=True)
best=None
for n in (3,2,1):
    f=f"{LOG}/{name}.reply{n}"
    if not os.path.exists(f): continue
    t=re.sub(r"<think>.*?</think>","",open(f).read(),flags=re.S)
    if "```rust\n" not in t: continue
    impl=t.split("```rust\n",1)[1].split("```")[0].split("#[cfg(test)]")[0]
    open(work,"w").write(impl.rstrip()+"\n\n"+tests)
    p=subprocess.run(f"cargo test -p snapious-core {flt} 2>&1",shell=True,cwd=W,capture_output=True,text=True).stdout
    m=re.search(r"test result: (ok|FAILED)\. (\d+) passed; (\d+) failed",p)
    if not m: score=(2,999)
    else: score=(0 if m.group(1)=="ok" and int(m.group(2))>0 else 1,int(m.group(3)))
    print(f"reply{n}: {'kompiliert nicht' if not m else m.group(0)}")
    if best is None or score<best[0]: best=(score,impl)
    if score[0]==0: break
if best:
    open(target,"w").write(best[1].rstrip()+"\n\n"+tests)
    print("eingesetzt: Wertung",best[0])

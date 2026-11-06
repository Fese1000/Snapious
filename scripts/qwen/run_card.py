#!/usr/bin/env python3
"""Allgemeiner Qwen-Runner: run_card.py <ziel.rs relativ zu crates/snapious-core/src> <testfilter> [kontextdatei ...] [--note "Hinweis"]
Das Ziel enthält ein Gerüst mit todo!() und fertigen Tests; Qwen liefert die ganze Datei, der Testblock muss unverändert bleiben."""
import json, re, subprocess, sys, time, urllib.request, os
ROOT=os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
SRC=f"{ROOT}/crates/snapious-core/src"
LOG=f"{ROOT}/scripts/qwen/logs"; os.makedirs(LOG,exist_ok=True)
args=sys.argv[1:]; note=""
if "--note" in args:
    i=args.index("--note"); note=args[i+1]; del args[i:i+2]
rel,flt,ctx=args[0],args[1],args[2:]
TARGET=f"{SRC}/{rel}"; name=os.path.basename(rel)
skeleton=open(TARGET).read()
open(f"{LOG}/{name}.skeleton","w").write(skeleton)
system=open(f"{ROOT}/docs/plan/00-ARBEITSANWEISUNG.md").read().split("## Fortschritt")[0]
def tests_of(c):
    i=c.find("#[cfg(test)]"); return c[i:] if i>=0 else ""
def sigs(path):
    t=open(path).read()
    if len(t)<9000: return t
    return "\n".join(l for l in t.splitlines() if re.match(r"\s*(pub |impl |///|#\[derive)",l))
ctxtxt=""
for c in ["error.rs"]+ctx:
    ctxtxt+=f"\n### {c} (nur lesen)\n```rust\n{sigs(f'{SRC}/{c}')}\n```\n"
user=f"""/no_think
Du hast KEINEN Dateizugriff. Gib die VOLLSTÄNDIGE neue Datei `crates/snapious-core/src/{rel}` in genau EINEM ```rust-Codeblock zurück.
Gib NUR den Teil der Datei VOR `#[cfg(test)]` zurück (Testblock weglassen, ich hänge ihn selbst an). Alle `///`-Beschreibungen der öffentlichen Signaturen bleiben BUCHSTÄBLICH unverändert; ersetze nur die `todo!()`-Rümpfe und entferne ungenutzte Imports. Die `///`-Kommentare sind die Aufgabenbeschreibung. Kein Text außer dem Codeblock. {note}
{ctxtxt}
### Aktuelle Datei (Gerüst)
```rust
{skeleton}```
"""
msgs=[{"role":"system","content":system},{"role":"user","content":user}]
def ask(m):
    """Standard: Roh-Endpunkt mit leerem Denkblock (kein endloses Nachdenken); QWEN_THINK=1 nutzt wieder die Chat-Schnittstelle."""
    t=time.time()
    if os.environ.get("QWEN_THINK")=="1":
        body=json.dumps({"model":"qwen3.6-35b-a3b","messages":m,"temperature":0.2,"max_tokens":32000}).encode()
        r=urllib.request.Request("http://localhost:1234/v1/chat/completions",data=body,headers={"Content-Type":"application/json"})
        with urllib.request.urlopen(r,timeout=3600) as f: d=json.load(f)
        print(f"[{time.time()-t:.0f}s {d.get('usage')}]",flush=True)
        return d["choices"][0]["message"]["content"]
    prompt="".join(f"<|im_start|>{x['role']}\n{x['content']}<|im_end|>\n" for x in m)+"<|im_start|>assistant\n<think>\n\n</think>\n\n"
    body=json.dumps({"model":"qwen3.6-35b-a3b","prompt":prompt,"temperature":0.2,"max_tokens":12000,"stop":["<|im_end|>"]}).encode()
    r=urllib.request.Request("http://localhost:1234/v1/completions",data=body,headers={"Content-Type":"application/json"})
    with urllib.request.urlopen(r,timeout=1800) as f: d=json.load(f)
    print(f"[{time.time()-t:.0f}s {d.get('usage')}]",flush=True)
    return d["choices"][0]["text"]
def extract(t):
    t=re.sub(r"<think>.*?</think>","",t,flags=re.S)
    m=re.findall(r"```rust\n(.*?)```",t,flags=re.S)
    if m: return max(m,key=len)
    if "```rust\n" in t: return t.split("```rust\n",1)[1]  # abgeschnittene Antwort: bis dahin verwenden
    return None
def run(c):
    p=subprocess.run(c,shell=True,cwd=ROOT,capture_output=True,text=True); return p.stdout+p.stderr
def lint(code):
    body=code.split("#[cfg(test)]")[0]; bad=[]
    for pat in ["unwrap()","expect(","panic!","unsafe","todo!","unimplemented!"]:
        if pat in body: bad.append(pat)
    if re.search(r"[\u3000-\u9fff\uac00-\ud7af]",code): bad.append("nicht-deutsche Schriftzeichen (alles auf Deutsch oder Englisch)")
    return bad
for a in range(1,4):
    print(f"=== {name} Versuch {a}",flush=True)
    rep=ask(msgs); open(f"{LOG}/{name}.reply{a}","w").write(rep)
    code=extract(rep)
    if not code:
        msgs+=[{"role":"assistant","content":rep},{"role":"user","content":"Gib die vollständige Datei in genau einem ```rust-Codeblock zurück."}]; continue
    code=code.split("#[cfg(test)]")[0].rstrip()+"\n\n"+tests_of(skeleton)
    bad=lint(code)
    if bad:
        msgs+=[{"role":"assistant","content":rep},{"role":"user","content":f"Verbotene Konstrukte im Bibliothekscode: {bad}. Entferne sie (Fehler per Result/Option behandeln) und gib die vollständige Datei erneut aus."}]; continue
    open(TARGET,"w").write(code)
    out=run(f"cargo test -p snapious-core {flt} 2>&1 | tail -60")
    ok=re.search(r"test result: ok\. (\d+) passed; 0 failed",out)
    if not ok:
        print(out[-1500:],flush=True)
        msgs+=[{"role":"assistant","content":rep},{"role":"user","content":"cargo test meldet:\n"+out[-2500:]+"\nBehebe das (Tests nicht ändern) und gib die vollständige Datei erneut aus."}]; continue
    run("cargo fmt")
    cl=run("cargo clippy -p snapious-core --all-targets 2>&1")
    mine=[l for l in cl.splitlines() if rel in l]
    if mine:
        ctxl="\n".join(cl.splitlines()[:0])+"\n".join(l for l in cl.splitlines() if rel in l or l.startswith(("warning","error")))[:2500]
        print("clippy:",ctxl,flush=True)
        msgs+=[{"role":"assistant","content":rep},{"role":"user","content":"Tests grün, aber clippy meldet in dieser Datei:\n"+ctxl+"\nBehebe das und gib die vollständige Datei erneut aus."}]; continue
    print(f"ERGEBNIS {name}: GRÜN ({ok.group(1)} Tests) nach {a} Versuch(en)",flush=True); sys.exit(0)
open(TARGET,"w").write(skeleton); print(f"ERGEBNIS {name}: ROT nach 3 Versuchen, Gerüst zurückgesetzt",flush=True); sys.exit(1)

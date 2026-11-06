import json, re, subprocess, sys, time, urllib.request
ROOT="/Users/felixcuntz/Documents/Snapious/GeneiousRebuild"
SCR="/private/tmp/claude-501/-Users-felixcuntz-Documents-Snapious/6e54639d-23df-43ed-8da8-413b11bf9a90/scratchpad/qwen"
TARGET=f"{ROOT}/crates/snapious-core/src/view/seq_layout.rs"
skeleton=open(f"{SCR}/seq_layout.skeleton.rs").read()
system=open(f"{ROOT}/docs/plan/00-ARBEITSANWEISUNG.md").read().split("## Fortschritt")[0]
card=open(f"{ROOT}/docs/plan/karten/1.1-zeilenlayout.md").read()
err=open(f"{ROOT}/crates/snapious-core/src/error.rs").read()
def tests_of(code):
    i=code.find("#[cfg(test)]"); return code[i:] if i>=0 else ""
user=f"""Arbeitsweise für diese Sitzung: Du hast KEINEN Dateizugriff und kein Terminal. Gib stattdessen die VOLLSTÄNDIGE neue Datei `crates/snapious-core/src/view/seq_layout.rs` in genau EINEM ```rust-Codeblock zurück.
Der Block `#[cfg(test)] mod tests {{ ... }}` muss BUCHSTÄBLICH unverändert übernommen werden. Ersetze alle `todo!()` durch echten Code. Kein Text außer dem Codeblock und höchstens drei Sätzen davor.

{card}

### Datei crates/snapious-core/src/error.rs (nur lesen)
```rust
{err}```

### Datei crates/snapious-core/src/view/seq_layout.rs (aktueller Stand mit todo!())
```rust
{skeleton}```
"""
messages=[{"role":"system","content":system},{"role":"user","content":user}]
def ask(msgs):
    body=json.dumps({"model":"qwen3.6-35b-a3b","messages":msgs,"temperature":0.2,"max_tokens":8000}).encode()
    req=urllib.request.Request("http://localhost:1234/v1/chat/completions",data=body,headers={"Content-Type":"application/json"})
    t=time.time()
    with urllib.request.urlopen(req,timeout=900) as r: d=json.load(r)
    print(f"[Antwort nach {time.time()-t:.0f}s, tokens={d.get('usage')}]")
    return d["choices"][0]["message"]["content"]
def extract(txt):
    txt=re.sub(r"<think>.*?</think>","",txt,flags=re.S)
    m=re.findall(r"```rust\n(.*?)```",txt,flags=re.S)
    return max(m,key=len) if m else None
def run(cmd):
    p=subprocess.run(cmd,shell=True,cwd=ROOT,capture_output=True,text=True)
    return p.returncode,(p.stdout+p.stderr)
for attempt in range(1,4):
    print(f"=== Versuch {attempt}")
    reply=ask(messages)
    open(f"{SCR}/reply{attempt}.txt","w").write(reply)
    code=extract(reply)
    if not code:
        print("Kein Codeblock gefunden"); messages+= [{"role":"assistant","content":reply},{"role":"user","content":"Gib die vollständige Datei in genau einem ```rust-Codeblock zurück."}]; continue
    if tests_of(code).strip()!=tests_of(skeleton).strip():
        print("Testblock wurde verändert -> zurückgewiesen")
        messages+=[{"role":"assistant","content":reply},{"role":"user","content":"Der Testblock `#[cfg(test)] mod tests` wurde verändert. Übernimm ihn buchstäblich unverändert. Gib die vollständige Datei erneut aus."}]
        continue
    open(TARGET,"w").write(code)
    rc,out=run("cargo test -p snapious-core seq_layout 2>&1 | tail -40")
    print(out[-1500:])
    if "test result: ok. 6 passed" in out:
        rc2,out2=run("cargo fmt && cargo clippy -p snapious-core -- -D warnings 2>&1 | tail -30")
        print("clippy/fmt:",out2[-1200:])
        if "error" not in out2.lower() and "warning" not in out2.lower():
            print("ERGEBNIS: GRÜN nach",attempt,"Versuch(en)"); sys.exit(0)
        messages+=[{"role":"assistant","content":reply},{"role":"user","content":"Tests grün, aber clippy/fmt meldet:\n"+out2[-1500:]+"\nBehebe das und gib die vollständige Datei erneut aus."}]
        continue
    messages+=[{"role":"assistant","content":reply},{"role":"user","content":"cargo test meldet:\n"+out[-1800:]+"\nBehebe das (Tests nicht ändern) und gib die vollständige Datei erneut aus."}]
print("ERGEBNIS: nicht grün nach 3 Versuchen"); open(TARGET,"w").write(skeleton); sys.exit(1)

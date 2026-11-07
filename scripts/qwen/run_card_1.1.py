import json, re, subprocess, sys, time, urllib.request
ROOT="/Users/felixcuntz/Documents/Snapious/GeneiousRebuild"
SCR="/private/tmp/claude-501/-Users-felixcuntz-Documents-Snapious/6e54639d-23df-43ed-8da8-413b11bf9a90/scratchpad/qwen"
TARGET=f"{ROOT}/crates/snapious-core/src/view/seq_layout.rs"
skeleton=open(f"{SCR}/seq_layout.skeleton.rs").read()
system=open(f"{ROOT}/docs/plan/00-WORKING-INSTRUCTIONS.md").read().split("## Fortschritt")[0]
card=open(f"{ROOT}/docs/plan/cards/1.1-row-layout.md").read()
err=open(f"{ROOT}/crates/snapious-core/src/error.rs").read()
def tests_of(code):
    i=code.find("#[cfg(test)]"); return code[i:] if i>=0 else ""
user=f"""Working mode for this session: you have NO file access and no terminal. Instead, return the COMPLETE new file `crates/snapious-core/src/view/seq_layout.rs` in exactly ONE ```rust code block.
The block `#[cfg(test)] mod tests {{ ... }}` must be copied LITERALLY unchanged. Replace all `todo!()` with real code. No text besides the code block and at most three sentences before it.

{card}

### File crates/snapious-core/src/error.rs (read only)
```rust
{err}```

### File crates/snapious-core/src/view/seq_layout.rs (current state with todo!())
```rust
{skeleton}```
"""
messages=[{"role":"system","content":system},{"role":"user","content":user}]
def ask(msgs):
    body=json.dumps({"model":"qwen3.6-35b-a3b","messages":msgs,"temperature":0.2,"max_tokens":8000}).encode()
    req=urllib.request.Request("http://localhost:1234/v1/chat/completions",data=body,headers={"Content-Type":"application/json"})
    t=time.time()
    with urllib.request.urlopen(req,timeout=900) as r: d=json.load(r)
    print(f"[reply after {time.time()-t:.0f}s, tokens={d.get('usage')}]")
    return d["choices"][0]["message"]["content"]
def extract(txt):
    txt=re.sub(r"<think>.*?</think>","",txt,flags=re.S)
    m=re.findall(r"```rust\n(.*?)```",txt,flags=re.S)
    return max(m,key=len) if m else None
def run(cmd):
    p=subprocess.run(cmd,shell=True,cwd=ROOT,capture_output=True,text=True)
    return p.returncode,(p.stdout+p.stderr)
for attempt in range(1,4):
    print(f"=== Attempt {attempt}")
    reply=ask(messages)
    open(f"{SCR}/reply{attempt}.txt","w").write(reply)
    code=extract(reply)
    if not code:
        print("No code block found"); messages+= [{"role":"assistant","content":reply},{"role":"user","content":"Return the complete file in exactly one ```rust code block."}]; continue
    if tests_of(code).strip()!=tests_of(skeleton).strip():
        print("Test block was modified -> rejected")
        messages+=[{"role":"assistant","content":reply},{"role":"user","content":"The test block `#[cfg(test)] mod tests` was modified. Copy it literally unchanged. Output the complete file again."}]
        continue
    open(TARGET,"w").write(code)
    rc,out=run("cargo test -p snapious-core seq_layout 2>&1 | tail -40")
    print(out[-1500:])
    if "test result: ok. 6 passed" in out:
        rc2,out2=run("cargo fmt && cargo clippy -p snapious-core -- -D warnings 2>&1 | tail -30")
        print("clippy/fmt:",out2[-1200:])
        if "error" not in out2.lower() and "warning" not in out2.lower():
            print("ERGEBNIS: GRÜN after",attempt,"attempt(s)"); sys.exit(0)
        messages+=[{"role":"assistant","content":reply},{"role":"user","content":"Tests green, but clippy/fmt reports:\n"+out2[-1500:]+"\nFix this and output the complete file again."}]
        continue
    messages+=[{"role":"assistant","content":reply},{"role":"user","content":"cargo test reports:\n"+out[-1800:]+"\nFix this (do not change the tests) and output the complete file again."}]
print("ERGEBNIS: not green after 3 attempts"); open(TARGET,"w").write(skeleton); sys.exit(1)

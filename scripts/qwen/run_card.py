#!/usr/bin/env python3
"""General Qwen runner: run_card.py <target.rs relative to crates/snapious-core/src> <test filter> [context file ...] [--note "hint"]
The target contains a skeleton with todo!() and finished tests; Qwen returns the whole file, the test block must stay unchanged."""
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
system=open(f"{ROOT}/docs/plan/00-WORKING-INSTRUCTIONS.md").read().split("## Fortschritt")[0]
def tests_of(c):
    i=c.find("#[cfg(test)]"); return c[i:] if i>=0 else ""
def sigs(path):
    t=open(path).read()
    if len(t)<9000: return t
    return "\n".join(l for l in t.splitlines() if re.match(r"\s*(pub |impl |///|#\[derive)",l))
ctxtxt=""
for c in ["error.rs"]+ctx:
    ctxtxt+=f"\n### {c} (read only)\n```rust\n{sigs(f'{SRC}/{c}')}\n```\n"
user=f"""/no_think
You have NO file access. Return the COMPLETE new file `crates/snapious-core/src/{rel}` in exactly ONE ```rust code block.
Return ONLY the part of the file BEFORE `#[cfg(test)]` (omit the test block, I will append it myself). All `///` descriptions of the public signatures stay LITERALLY unchanged; only replace the `todo!()` bodies and remove unused imports. The `///` comments are the task description. No text besides the code block. {note}
{ctxtxt}
### Current file (skeleton)
```rust
{skeleton}```
"""
msgs=[{"role":"system","content":system},{"role":"user","content":user}]
def ask(m):
    """Default: raw endpoint with an empty think block (no endless thinking); QWEN_THINK=1 uses the chat interface again."""
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
    if "```rust\n" in t: return t.split("```rust\n",1)[1]  # truncated reply: use what is there
    return None
def run(c):
    p=subprocess.run(c,shell=True,cwd=ROOT,capture_output=True,text=True); return p.stdout+p.stderr
def lint(code):
    body=code.split("#[cfg(test)]")[0]; bad=[]
    for pat in ["unwrap()","expect(","panic!","unsafe","todo!","unimplemented!"]:
        if pat in body: bad.append(pat)
    if re.search(r"[\u3000-\u9fff\uac00-\ud7af]",code): bad.append("non-English characters (everything in English)")
    return bad
for a in range(1,4):
    print(f"=== {name} attempt {a}",flush=True)
    rep=ask(msgs); open(f"{LOG}/{name}.reply{a}","w").write(rep)
    code=extract(rep)
    if not code:
        msgs+=[{"role":"assistant","content":rep},{"role":"user","content":"Return the complete file in exactly one ```rust code block."}]; continue
    code=code.split("#[cfg(test)]")[0].rstrip()+"\n\n"+tests_of(skeleton)
    bad=lint(code)
    if bad:
        msgs+=[{"role":"assistant","content":rep},{"role":"user","content":f"Forbidden constructs in the library code: {bad}. Remove them (handle errors via Result/Option) and output the complete file again."}]; continue
    open(TARGET,"w").write(code)
    out=run(f"cargo test -p snapious-core {flt} 2>&1 | tail -60")
    ok=re.search(r"test result: ok\. (\d+) passed; 0 failed",out)
    if not ok:
        print(out[-1500:],flush=True)
        msgs+=[{"role":"assistant","content":rep},{"role":"user","content":"cargo test reports:\n"+out[-2500:]+"\nFix this (do not change the tests) and output the complete file again."}]; continue
    run("cargo fmt")
    cl=run("cargo clippy -p snapious-core --all-targets 2>&1")
    mine=[l for l in cl.splitlines() if rel in l]
    if mine:
        ctxl="\n".join(cl.splitlines()[:0])+"\n".join(l for l in cl.splitlines() if rel in l or l.startswith(("warning","error")))[:2500]
        print("clippy:",ctxl,flush=True)
        msgs+=[{"role":"assistant","content":rep},{"role":"user","content":"Tests green, but clippy reports in this file:\n"+ctxl+"\nFix this and output the complete file again."}]; continue
    print(f"ERGEBNIS {name}: GRÜN ({ok.group(1)} tests) after {a} attempt(s)",flush=True); sys.exit(0)
open(TARGET,"w").write(skeleton); print(f"ERGEBNIS {name}: ROT after 3 attempts, skeleton restored",flush=True); sys.exit(1)

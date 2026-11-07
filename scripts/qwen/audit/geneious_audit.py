#!/usr/bin/env python3
"""Compare Geneious plugins one by one with Qwen: python3 geneious_audit.py [plugin name part ...]
Per plugin: class and label list from the jar (as REA shows it as an inventory) + function overview of Snapious -> Qwen (no thinking phase,
via /v1/completions) -> out/<plugin>.md. Qwen only ever gets one plugin."""
import json, os, re, sys, urllib.request, zipfile, glob
HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "../../.."))
PLUG = "/Applications/Geneious Prime.app/Contents/Resources/app/bundledPlugins"
OUT = os.path.join(HERE, "out"); os.makedirs(OUT, exist_ok=True)

def snapious_features():
    rows = []
    base = os.path.join(ROOT, "crates/snapious-core/src")
    for p in sorted(glob.glob(base + "/**/*.rs", recursive=True)):
        rel = os.path.relpath(p, base)
        if rel.endswith("mod.rs") or rel == "lib.rs" or rel == "error.rs":
            continue
        with open(p, encoding="utf-8") as f:
            for line in f:
                if line.startswith("//!"):
                    rows.append(f"{rel}: {line[3:].strip()[:95]}")
                    break
    return "\n".join(rows)

def ffi_functions():
    with open(os.path.join(ROOT, "crates/snapious-ffi/src/lib.rs"), encoding="utf-8") as f:
        names = re.findall(r"^\s*pub fn ([a-z0-9_]+)", f.read(), re.M)
    return ", ".join(sorted(set(names)))

def digest(plugdir):
    names, labels = [], set()
    for jar in glob.glob(plugdir + "/**/*.jar", recursive=True):
        try:
            z = zipfile.ZipFile(jar)
        except Exception:
            continue
        cls = [n for n in z.namelist() if n.endswith(".class") and ("com/biomatters/plugins/" in n or "com/michaelrthon" in n)]
        if not cls:
            continue
        for n in cls:
            base = n.rsplit("/", 1)[-1][:-6]
            if "$" in base or len(base) <= 2:
                continue
            names.append(n[len("com/biomatters/plugins/"):-6] if n.startswith("com/biomatters/plugins/") else n[:-6])
            data = z.read(n)
            for m in re.finditer(rb"[ -~]{12,}", data):
                s = m.group().decode("ascii")
                if s.count(" ") >= 1 and re.search(r"[A-Za-z]{4}", s) and not s.startswith(("L", "(", "com/", "java", "[")) and "/" not in s[:20]:
                    labels.add(s)
    return sorted(set(names))[:140], sorted(labels)[:140]

def ask(prompt):
    body = {"model": "qwen3.6-35b-a3b", "temperature": 0.2, "max_tokens": 1100,
            "prompt": "<|im_start|>user\n" + prompt + "<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n\n"}
    req = urllib.request.Request("http://localhost:1234/v1/completions", json.dumps(body).encode(), {"Content-Type": "application/json"})
    return json.load(urllib.request.urlopen(req, timeout=900))["choices"][0]["text"].strip()

SKIP = ("cloud", "serverDatabase", "shareButton", "teaching", "stars", "star.", "notes", "favorites", "documentHistory",
        "operationsTable", "treeMapViewer", "stylizedDnaHelix", "luma", "endnote", "PDFdocument", "parentdescendant", "textviewer")

def main():
    feats = snapious_features()
    ffi = ffi_functions()
    want = sys.argv[1:]
    for d in sorted(os.listdir(PLUG)):
        short = d.replace("com.biomatters.plugins.", "")
        if want and not any(w.lower() in short.lower() for w in want):
            continue
        if not want and any(k.lower() in short.lower() for k in SKIP):
            continue
        out = os.path.join(OUT, short + ".md")
        if os.path.exists(out):
            continue
        names, labels = digest(os.path.join(PLUG, d))
        if not names:
            continue
        # Step A: describe only the plugin, without mentioning Snapious
        a = ask(f"""Geneious Prime (bioinformatics software), plugin: {short}
Classes:
{chr(10).join(names)}
Visible texts:
{chr(10).join(labels)}

Which biological functions does this plugin offer the user (operations, analyses, file formats, views with biological content)?
Answer only with a list, one short function per line (at most 10 words each), at most 12 lines, in English, starting with "- ".
Only functions that can be recognized from the class names or texts above; do not invent anything. If there is no biological function: only NONE.""")
        if a.strip().upper().startswith(("NONE", "KEINE")) or not a.strip():
            open(out, "w").write("NONE\n"); continue
        # Step B: check each function against Snapious
        b = ask(f"""Snapious is a rebuild of Geneious. Function overview of Snapious (file: purpose):
{feats}

Functions reachable in the Snapious app (interface): {ffi}

Geneious functions from the plugin {short}:
{a}

Check EACH of these Geneious functions individually against Snapious. One entry per line in the format:
- <function> | Core: yes/partial/no | App: yes/no | <matching Snapious file or what is missing>
Name only files and function names that actually appear above. Do not add extra functions. Be strict: when in doubt, "no".""")
        open(out, "w").write("GENEIOUS FUNCTIONS:\n" + a + "\n\nCOMPARISON:\n" + b + "\n")
        print(short, flush=True)

main()

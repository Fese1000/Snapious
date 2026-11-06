#!/usr/bin/env python3
"""Geneious-Plugin für Plugin mit Qwen vergleichen: python3 geneious_audit.py [Plugin-Teilname ...]
Je Plugin: Klassen- und Beschriftungsliste aus dem Jar (wie REA sie als Inventar zeigt) + Funktionsübersicht von Snapious -> Qwen (ohne Denkphase,
über /v1/completions) -> out/<plugin>.md. Qwen bekommt immer nur ein Plugin."""
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
        # Schritt A: nur das Plugin beschreiben, ohne Snapious zu erwähnen
        a = ask(f"""Geneious Prime (Bioinformatik-Software), Plugin: {short}
Klassen:
{chr(10).join(names)}
Sichtbare Texte:
{chr(10).join(labels)}

Welche biologischen Funktionen bietet dieses Plugin dem Nutzer (Operationen, Analysen, Dateiformate, Ansichten mit biologischem Inhalt)?
Antworte nur mit einer Liste, eine kurze Funktion pro Zeile (je höchstens 10 Wörter), höchstens 12 Zeilen, Deutsch, mit "- " beginnend.
Nur Funktionen, die aus den Klassennamen oder Texten oben erkennbar sind; nichts erfinden. Ohne biologische Funktion: nur KEINE.""")
        if a.strip().upper().startswith("KEINE") or not a.strip():
            open(out, "w").write("KEINE\n"); continue
        # Schritt B: jede Funktion gegen Snapious prüfen
        b = ask(f"""Snapious ist ein Nachbau von Geneious. Funktionsübersicht von Snapious (Datei: Zweck):
{feats}

In der Snapious-App erreichbare Funktionen (Schnittstelle): {ffi}

Geneious-Funktionen aus dem Plugin {short}:
{a}

Prüfe JEDE dieser Geneious-Funktionen einzeln gegen Snapious. Ein Eintrag pro Zeile im Format:
- <Funktion> | Kern: ja/teilweise/nein | App: ja/nein | <passende Snapious-Datei oder was fehlt>
Nenne nur Dateien und Funktionsnamen, die oben wirklich vorkommen. Keine zusätzlichen Funktionen hinzufügen. Sei streng: im Zweifel "nein".""")
        open(out, "w").write("GENEIOUS-FUNKTIONEN:\n" + a + "\n\nABGLEICH:\n" + b + "\n")
        print(short, flush=True)

main()

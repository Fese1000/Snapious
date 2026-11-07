# Working with Qwen3.6-35B-A3B (local)

Purpose of this document: cut tasks so that a model with **35B parameters in total but only about 3B active per token** implements them reliably.
*Assessment (without own measurements on this model):* Such mixture-of-experts models are broadly knowledgeable and fast, but are more prone than dense models of the same size to failing at long multi-step derivations,
fine details in long texts, and strict type logic (Rust borrow checker). Therefore:
**little context, small steps, ready-made tests, mechanical verification.** If it turns out in practice that the model can handle more, the steps can get bigger.

## Basic Principles

1. **Tests first, written by Claude.** For each task, Claude (or you) sets up the skeleton: a file with types, function signatures with `todo!()` in the body, and **finished tests**. Qwen only fills in the function bodies
   until `cargo test` is green. This is the most effective help for small models because "green/red" is an unambiguous goal.
2. **One prompt = one task card** (at most about 200 lines of new code, one file, one topic). Larger tasks from `01`–`07` are broken into cards beforehand (rules below).
3. **Small context.** The prompt contains only: system text (`00-WORKING-INSTRUCTIONS.md`), the task card, the affected file, at most two helper files (e.g. `error.rs`, a type the card uses).
   Not the whole phase file, not the Java project (unless the card names exactly one Java method as a reference and inlines it).
4. **Mechanical verification instead of judgment.** Qwen runs `cargo check`/`test`/`clippy`/`fmt` itself and reads the error messages. No "looks good".
5. **Three attempts, then stop.** If errors persist, do not keep guessing; record them in `OPEN-QUESTIONS.md` (see system text). Claude then takes a look.
6. **Review by Claude** after each task (checklist below) before the next card is handed out.
7. Recommended settings (adjust if the model reacts differently): low temperature (about 0.2–0.3), do not truncate responses (sufficient `max_tokens`),
   thinking mode – if available – on for decomposition and debugging, off for plain body filling.

## Task Card (Template)

```
# Card <No>: <Title>
File(s): crates/snapious-core/src/<path>.rs      (skeleton and tests already present)
Prerequisite: <which cards must be done>
Goal: <one sentence>
Functions: <list with signature and one sentence of behavior each>
Rules/formulas: <exactly the necessary ones, with a numeric example>
Edge cases: <list>
Reference: <optional: exactly one Java method, inlined>
Done when: `cargo test -p snapious-core <module>` green, fmt/clippy clean.
Do not: <what typically goes wrong: change other files, change tests, new crates>
```

### Example: Card 1.1 "Line layout" (verified: skeleton compiles, with the reference solution all 6 tests are green)

```
# Card 1.1: Line layout of the sequence view
File: crates/snapious-core/src/view/seq_layout.rs (skeleton and tests present)
Prerequisite: none (only `error.rs`)
Goal: A sequence is split into lines of fixed length; positions and visible lines can be computed.
Functions:
  SequenceLayout::new(length, bases_per_line) -> Result<Self>   // bases_per_line == 0 → Err(InvalidInput)
  line_count() -> usize            // ceil(length / bases_per_line), for length 0: 0
  line_of(position) -> usize       // position / bases_per_line
  column_of(position) -> usize     // position % bases_per_line
  line_start(line) -> usize        // line * bases_per_line
  line_end(line) -> usize          // min(line_start + bases_per_line, length)
  first_visible_line(scroll_y, line_height) -> usize      // floor(scroll_y / line_height), for scroll_y <= 0 or line_height <= 0: 0
  last_visible_line(&self, scroll_y, view_height, line_height) -> usize
                                   // floor((scroll_y + view_height) / line_height), capped at line_count() - 1 (for 0 lines: 0)
  bases_per_line_for(view_width, char_width, margin_width, group_size) -> usize
                                   // group width = (group_size + 1) * char_width; available = view_width - margin_width (at least 0)
                                   // groups = floor(available / group width), at least 1; result = groups * group_size
Edge cases: length 0, position exactly at the start of a line, last line shorter, group_size 0 → treated like 1.
Done when: `cargo test -p snapious-core seq_layout` green.
Do not: change tests; change other files; `unwrap()`.
```

**Skeleton that Claude sets up beforehand** (bodies empty, tests finished):

```rust
//! Line layout of the sequence view (pure computation logic, no UI).
use crate::error::{Error, Result};

/// Splitting of a sequence into lines of fixed length.
#[derive(Clone, Debug, PartialEq)]
pub struct SequenceLayout { length: usize, bases_per_line: usize }

impl SequenceLayout {
    pub fn new(length: usize, bases_per_line: usize) -> Result<Self> { todo!() }
    pub fn line_count(&self) -> usize { todo!() }
    pub fn line_of(&self, position: usize) -> usize { todo!() }
    pub fn column_of(&self, position: usize) -> usize { todo!() }
    pub fn line_start(&self, line: usize) -> usize { todo!() }
    pub fn line_end(&self, line: usize) -> usize { todo!() }
    pub fn first_visible_line(scroll_y: f64, line_height: f64) -> usize { todo!() }
    pub fn last_visible_line(&self, scroll_y: f64, view_height: f64, line_height: f64) -> usize { todo!() }
}

pub fn bases_per_line_for(view_width: f64, char_width: f64, margin_width: f64, group_size: usize) -> usize { todo!() }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zeilen_bei_laenge_125_und_60_pro_zeile() {
        let l = SequenceLayout::new(125, 60).unwrap();
        assert_eq!(l.line_count(), 3);
        assert_eq!(l.line_of(59), 0);
        assert_eq!(l.line_of(60), 1);
        assert_eq!(l.column_of(61), 1);
        assert_eq!(l.line_start(2), 120);
        assert_eq!(l.line_end(2), 125);
    }

    #[test]
    fn leere_sequenz_hat_keine_zeilen() {
        assert_eq!(SequenceLayout::new(0, 60).unwrap().line_count(), 0);
    }

    #[test]
    fn null_basen_pro_zeile_ist_fehler() {
        assert!(SequenceLayout::new(10, 0).is_err());
    }

    #[test]
    fn sichtbare_zeilen_bei_scrollposition_250() {
        let l = SequenceLayout::new(6000, 60).unwrap(); // exactly 100 lines
        assert_eq!(SequenceLayout::first_visible_line(250.0, 20.0), 12);
        assert_eq!(l.last_visible_line(250.0, 100.0, 20.0), 17);
    }

    #[test]
    fn letzte_zeile_wird_auf_vorhandene_zeilen_begrenzt() {
        let l = SequenceLayout::new(125, 60).unwrap(); // 3 lines
        assert_eq!(l.last_visible_line(0.0, 10_000.0, 20.0), 2);
    }

    #[test]
    fn basen_pro_zeile_aus_fensterbreite() {
        assert_eq!(bases_per_line_for(800.0, 10.0, 80.0, 10), 60);
        assert_eq!(bases_per_line_for(50.0, 10.0, 0.0, 10), 10);
    }
}
```

*(`error.rs` with `Error::InvalidInput(String)` and `type Result<T>` is created in Phase 0.)*

## How Claude Breaks the Tasks from `01`–`07` into Cards

| Rule | Example |
|---|---|
| One card per **file** and **topic**, at most about 200 lines of code | 2.1 Virtual digest → 2.1a `End`/`Fragment` types, 2.1b `cut_positions` (only hits and cuts), 2.1c `digest_linear`, 2.1d `digest_circular`, 2.1e features in fragments |
| Types and constructors first, algorithms afterwards | 2.3 → 2.3a data types, 2.3b `find_overlap`, 2.3c `assemble_linear`, 2.3d circular, 2.3e warnings, 2.3f primer design |
| Pure functions before state | `EditSession` only after `SequenceDocument::insert/delete` |
| Every card has tests with **concrete numbers** (from the phase files or Java tests) | see example above |
| Complex formulas are **written out** on the card | Nearest-neighbor Tm: table and formula on the card, not "see paper" |
| Floating-point numbers in tests always with a tolerance | `assert!((a - b).abs() < 1e-4)` |
| Tasks touching several modules are separated: core API first, then consumers | 2.2 Restriction cloning after 2.1 |

**The order of the cards** follows `08-RUST-CORE-SWIFT-APP.md`, section "Order". Claude sets up skeleton + tests **one card in advance** (so the next card is always ready).

## Cheat Sheet: Typical Rust Errors and What to Do

| Error | Cause | Solution |
|---|---|---|
| `E0382 borrow of moved value` | Value was moved | `.clone()` at the first use, or the function takes `&T` instead of `T` |
| `E0502/E0499 cannot borrow as mutable` | simultaneous borrows | Copy the value into a local variable beforehand (`let n = v.len();`), then mutate |
| `E0308 mismatched types` | `usize`/`i32`/`f64` mixed | convert explicitly: `x as usize`, `x as f64`; for `usize` subtraction: `checked_sub`/`saturating_sub` |
| `E0599 no method named …` | trait not imported or wrong type | add `use`; get to the right type with `.iter()` or `.as_bytes()` |
| `E0106 missing lifetime specifier` | reference in a struct/return | do **not** introduce `'a`: struct fields own `String`/`Vec<u8>`; return an owned value |
| `E0277 the trait bound … is not satisfied` | missing `derive`s | add `#[derive(Clone, Debug, PartialEq)]` (for `f64`: no `Eq`/`Hash`) |
| `unused …` warnings with `-D warnings` | clippy | remove or use `_name` |
| `attempt to subtract with overflow` in the test | `usize` underflow | `saturating_sub` or compare beforehand |
| `index out of bounds` | unchecked index | `get(i)` + `ok_or(...)?`, or beforehand `if i >= len { return Err(...) }` |
| Test hangs (infinite loop) | loop without progress (e.g. cut at position 0) | the loop variable must grow in **every** iteration; handle the case `pos == last` explicitly |
| `clippy::needless_range_loop` | loop over an index | `for (i, x) in v.iter().enumerate()` |
| Sequence indexed as a `String` | UTF-8 | treat sequences as `&[u8]` (`bases[i]`) |

## Typical Pitfalls with Small Models (and Countermeasures)

| Pitfall | Countermeasure |
|---|---|
| changes the given tests until they are green | Card: "Do not: change tests"; Claude checks via `git diff`/file comparison that the test code is unchanged (file hash of `mod tests`) |
| invents functions/crates that do not exist | only the standard library + allowed crates (`08`, table); `cargo check` exposes it immediately |
| solves a simpler problem than asked | Card contains numeric examples; tests check exact values |
| aborts on long answers or repeats itself | small cards; have it fill in body by body; if needed, one function per prompt |
| changes formatting/renames | `cargo fmt` at the end; the card names the allowed files |
| gets stuck in error loops | three-attempts rule (system text no. 12) |
| forgets edge cases | card lists edge cases; tests cover them |
| mixes Java syntax into Rust | cards contain **only Rust signatures**; Java only as a reference snippet, explicitly marked as such |

## Checklist for Claude After Each Qwen Card

1. Were only the allowed files changed? (compare file list)
2. Is the test code unchanged? (compare `mod tests` with the template)
3. `cargo test -p snapious-core`, `cargo clippy -p snapious-core -- -D warnings`, `cargo fmt --check` without errors?
4. Spot check: one additional own test with a different numeric example (edge case). If red → card goes back.
5. No `unwrap()`/`expect()`/`panic!`/`unsafe` in library code? (`grep`)
6. No new dependencies in `Cargo.toml`? No macOS-specific calls?
7. Documentation comments in German, source named for algorithms?
8. Update progress in `00-WORKING-INSTRUCTIONS.md` or the card list.

## Proposal for Collaboration

| Who | Task |
|---|---|
| Claude | Phase 0 and 0.9, FFI (F.0), all Swift tasks (S.x), skeletons + tests of the Qwen cards, review |
| Qwen | Function bodies of the Rust cards from Phase 1 onward (layout, formats, algorithms, cloning, primers, trees, network, …) |
| Felix | Install Xcode/programs, start Qwen and pass on outputs, visual feedback on the app |

If the model proves more reliable than assumed: merge cards (e.g. a whole task `2.x` per card). If it often fails: shrink cards further (one function per prompt).

## Pilot Run of Card 1.1 (25.10.2026) – Result

- **Access:** LM Studio runs on `http://localhost:1234/v1` (OpenAI-compatible), model ID `qwen3.6-35b-a3b`. Claude calls it directly via `chat/completions` (script `scripts/qwen/run_card_1.1.py`).
- **Working mode during the run:** Qwen has no file access; it receives system text + card + files and returns the **complete file in a code block**. The script checks that the test block is **literally unchanged**,
  writes the file, runs `cargo test`, and passes error messages back (up to 3 attempts).
- **Result:** All 6 tests green after **2 attempts**. Attempt 1: tests green, but `clippy` complained about a hand-written call sum (`manual_div_ceil`); attempt 2 fixed that after feedback.
  The test block remained unchanged. My additional edge-case tests (negative scroll values, character width 0, `group_size` 0, window smaller than margin, boundary cases in the group count) all passed.
- **Observations:** Answer 1 took 148 s and used about 5,100 thinking tokens (thinking mode on) – a lot for such a simple card; answer 2 took 34 s. The card could have been somewhat larger (e.g. two files).
  Qwen omitted the `///` comments for the functions (rule 9). The checklist will check this in future with `grep -L "///"`.
- **Conclusion:** Increase card size by about 1.5 to 2 times; turn thinking mode off for plain filling in where possible; explicitly repeat the rule "documentation comments for all `pub` items" in the card.

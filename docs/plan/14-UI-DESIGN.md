# Phase 7 – User Interface: Own Look, Easier Operation

Starting point: The app uses standard SwiftUI (NavigationSplitView, Form, TabView, toolbar with ~9 equal-weight buttons,
system colors, 116 scattered `Color`/`.font` usages, no shared design). Goal: a recognizable style of its own and
fewer clicks to a result. The Rust core stays untouched; everything happens in `app/Sources/SnapiousApp`.

## Guidelines
1. **One design system instead of scattered colors**: all colors, spacing, fonts, radii come from one file (`Theme.swift`).
2. **Task before function**: The user chooses what they want to do ("Clone", "Primers", "Check sequencing"), not menus/tabs.
3. **Data is the main character**: Sequence, map, gel, and tree get the most space; controls recede.
4. **Calm, dense, dark-mode-capable**: Light and dark mode on equal footing, science look (like figures in papers), no Apple standard gray surfaces.
5. **Always a way back**: Undo, history, no modal dialog without Cancel.

## Step 1 – Design System (`Theme.swift`, `Components.swift`)
- Palette: base ink dark blue/paper white, one strong accent (petrol/turquoise), secondary accent amber; fixed
  DNA base colors (A green, C blue, G amber, T red) and feature colors for genes/promoters/markers, contrast-checked.
- Font: monospace sequence font (SF Mono → optionally bundled JetBrains Mono), headings with `.rounded`/serif accent.
- Building blocks: `Card`, `SectionHeader`, `PillButton`, `IconButton`, `Chip`/`Tag`, `StatTile`, `EmptyState`, `Toast`, `Badge`.
- Convert all views to these building blocks, no more direct `Color.accentColor`/`.font(.caption)` (check via script).

## Step 2 – Framework and Navigation
- Own sidebar instead of the system list: project header, search field on top, folders/favorites/recently used, type icons with color,
  preview mini-map per sequence.
- Tidy up the toolbar: only *Import*, *Search (⌘K)*, *Undo*; everything else moves into the **command palette**.
- **Command palette ⌘K**: find and run all 29 operations, views, documents, settings by typing.
- Replace tabs with a **task bar**: groups *View* (sequence, map), *Build* (digest, clone, primers, guides),
  *Check* (reads, analysis, protein, RNA), *Compare* (alignment, tree), *Sequence data* (NGS), *Tools*.
- Right **inspector sidebar** (can be shown/hidden) for selection, features, notes, versions instead of separate dialogs.

## Step 3 – Sequence and Map View (Centerpiece)
- Sequence: base colors, codon grid, translation rows on/off, ruler, gentle hover/selection highlighting, minimap overview.
- Feature tracks as rounded arrows with labels, overlaps distributed across tracks, click = inspector.
- Map: plasmid ring with gradient colors, enzyme labels without overlap (collision avoidance), zoom via pinch,
  animations when switching ring↔linear.
- Gel, tree, heatmap, logo, coverage in the same style; uniform **export as PNG/SVG** directly on the chart.

## Step 4 – Guided Workflows
- **Start page** instead of an empty window: large cards "Open project", "Fetch from NCBI", "Drag in file", sample project, recently used.
- **Wizards** for standard tasks (step bar on top, live preview): cloning (vector→insert→enzymes→result),
  primer design, check sequencing, variant search from reads.
- Empty states with a hint and button ("No primers yet – design primers"), understandable error messages with a suggested solution.
- First-run tour (3–4 speech bubbles) and tooltips with keyboard shortcuts.

## Step 5 – Feedback and Finishing Touches
- Progress/cancel for long tasks (BLAST, mapping, alignment) as a toast/bar at the bottom, not as a blocking dialog.
- Undo/redo for all changes (based on the versions).
- Keyboard shortcut overview (⌘/), visible drag-and-drop targets, multi-selection action bar.
- Short animations (150–250 ms), respect `Reduce Motion`; VoiceOver labels, minimum contrast 4.5:1, dynamic type size.
- Window sizes: layout tested from 900 px to full screen (snapshot tests per width).

## Order and Effort
| # | Package | Visible result | Effort |
|---|---------|----------------|--------|
| 1 | Theme + building blocks, ContentView/sidebar converted | App looks different | medium |
| 2 | Command palette + tidied toolbar + task bar | fewer clicks | medium |
| 3 | New sequence/map view | biggest quality leap | large |
| 4 | Start page + wizards + empty states | easy entry | medium |
| 5 | Inspector, toasts, undo, accessibility | finishing touches | medium |

## Verification
- Extend snapshot mode (`Snapious --snapshot`) to cover all new views in light/dark and 3 widths, and look at each result.
- SelfTest stays green; new pure logic (command search, label collision avoidance, color assignment) with Swift tests.
- Live check via computer use after each package; Rust tests stay green and unchanged.

## Division of Labor
Claude does layout and design work itself (visual judgment needed). Qwen only gets pure logic as a card:
command search (fuzzy match), label collision avoidance, track assignment of overlapping features.

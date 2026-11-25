# iStCAD → IST_CADCraft mapping

IST_CADCraft keeps the CADCraft core (engine, formats, renderer, dialogs) and lays the iStCAD
window over it. The shell lives in `crates/ui-egui/src/istcad.rs` (icons in `istcad_icons.rs`); `CadApp::istcad.enabled`
switches between it and CADCraft's own chrome (the desktop app turns it on in
`apps/cadcraft/src/main.rs`; `ui.istcad.shell {"on": false}` switches back at run time).

## Layout (iStCAD `MainWindow.xaml` rows)

iStCAD is the reference: sizes, colours and order below are iStCAD's.

| iStCAD | IST_CADCraft (`crates/ui-egui/src/istcad.rs`) |
|---|---|
| Title bar 40, `#292F38`: logo 30, Undo/Redo 26, rule, New/Open/Save/Save As 29, rule, File/Edit/View 60×28; title 14 pt + version 9 pt `#A7AEB8`; search host 218×34 `#3B414A`; Minimize/Maximize/Close 42 wide | `title_bar`: the same, with iStCAD's own line icons (`istcad_icons.rs`, drawn from the XAML path data). Menus list the registry's commands one level under File/Edit/View. Search lists matching commands. On Windows/Linux the window has no OS frame: the bar drags, double-click maximizes, the bottom/side edges resize. |
| Main toolbar 50, `#30363F`, `#555B65` border, radius 5: "Text Style" / "Dim Style" / "Layer" labels `#C6CAD2` with `ComboLike` boxes (112/96/104, `#343A43`); rule; tool buttons 38×36 (Select, Line, Circle, Arc, Polyline, Rectangle, Move, Trim, Dimension); Settings, Preferences | `main_toolbar`: the same. The boxes list the drawing's tables and make a pick current (`style.current`, `dimstyle.current`, `layer.current`); with no drawing they show Standard / ISO-25 / ByLayer. The running command's tool is highlighted `#4A515B` (Select when idle). |
| Drawing tabs 28: list button 48, tabs (radius 4,4,0,0, active `#4E555F`/`#707781`, inactive `#313740`/`#4D545E`, dirty dot `#E7B259` / saved `#BEC4CD`, name 11 pt ≤ 150, × 21), `+` 30 | `drawing_tabs`: the same. The list button also reaches the Start page. |
| Drawing area `#192429`, `#626973` frame; grid `#212C34` / every fifth `#34404F`; auto-hide handle 12×58 `#4A515B` "›", strip 60 wide with 34×32 buttons for the 20 catalog commands ranked by use | Theme tokens `canvas`, `grid_minor`, `grid_major`, `axis_*`; `canvas_frame`; `quick_bar` with iStCAD's `FrequentCommandCatalog` order (LINE, CIRCLE, RECTANGLE, ARC, MOVE, COPY, ERASE, SELECT, ZE, ZP, PAN, UNDO, REDO, NEW, OPEN, SAVE, SAVEAS, PLOT, LAYOUT, REGEN). Ctrl+1 toggles Properties. |
| Command dock 86, two rows, columns 4.35 : 1.05. Left: command bar 40 (`#3B414A`, "›", "Command:", input, status 10 pt ≤ 260, ⌃ history), then Model/layout tabs (selected `#565D67`, others `#414750`), `+` 27, PDF 36×24. Right: OSNAP ORTHO POLAR OTRACK DYN (18 high, on `#AEB4BE` with `#292E36` text) + drafting settings; then ▧ ▣ ⚙, zoom %, −, +, Ext. 45, slider 68, GRID 38×22. AutoComplete 430 wide above the input (name column 130, description `#9FADB2`, selected `#5A6069`); history 196 high above the dock (`#182226` at 95 %). | `command_dock`, `command_bar`, `autocomplete`, `history_window`, `layout_tabs`, `drafting_toggles`, `zoom_controls`: the same. The command line shows the running command's prompt with clickable options; typing, Enter/Space, Tab, arrows and F2 keep CADCraft's command-line handling. The zoom readout's 100 % is how the drawing first showed. |

## Colours

`theme::Tokens::DARK` uses iStCAD's palette: chrome `#292F38` / `#30363F`, panels `#363C45`,
controls `#414750` / hover `#4A515B`, borders `#555B65`, text `#E8ECEE` / `#B7BCC5`, accent teal
`#55C7C0`, picked-point orange `#F2A56F`, viewport `#192429` with grid `#212C34` / `#34404F` and
axes `#3E4E5E`, object snap `#55E6A5`, selection windows `#CACFD8` / `#9DAAC0`. iStCAD colours
with no token of their own are in `istcad::ink`. The Light theme is CADCraft's.

## Commands and aliases

iStCAD's aliases resolve to CADCraft commands (`crates/engine/src/cmd/*`):

| iStCAD | Command id | Change |
|---|---|---|
| `L`, `PL`, `C`, `REC`, `A`, `E`, `M`, `CO`, `U` | `line`, `pline`, `circle`, `rectang`, `arc`, `erase`, `move`, `copy`, `u` | already in CADCraft |
| `S` / `SELECT` | `select` | `S` moved from STRETCH to SELECT; STRETCH is now `STR` |
| `ZE`, `ZP` | `zoom.extents`, `zoom.previous` | new aliases (`ZOOM E` also works) |
| `Y` / `REDO` | `redo` | new alias |
| `PDF` | `exportpdf` | new alias; `PLOT` opens the Plot dialog |
| `PERF` | UI command `perf` | new: echoes entity count, display primitives, frame/build/draw times and the renderer |
| `LAYOUT`, `REGEN`, `UNDO` | same | already in CADCraft |

Space on an empty command line repeats the last command; Tab accepts an AutoComplete suggestion.

## Not carried over from iStCAD

- iStCAD's autosave folder `%LocalAppData%\iStCAD_1\Recovery`: CADCraft's own autosave and
  recovery are used instead.
- iStCAD's ACadSharp DWG/DXF layer: CADCraft reads and writes DWG/DXF natively in Rust.

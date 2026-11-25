# iStCAD → IST_CADCraft mapping

IST_CADCraft keeps the CADCraft core (engine, formats, renderer, dialogs) and lays the iStCAD
window over it. The shell lives in `crates/ui-egui/src/istcad.rs`; `CadApp::istcad.enabled`
switches between it and CADCraft's own chrome (the desktop app turns it on in
`apps/cadcraft/src/main.rs`; `ui.istcad.shell {"on": false}` switches back at run time).

## Layout (iStCAD `MainWindow.xaml` rows)

| iStCAD | IST_CADCraft |
|---|---|
| Row 0 — title bar 40: logo, Undo/Redo, New/Open/Save/SaveAs, File/Edit/View, centred title + version, Search, Min/Max/Close | `istcad::title_bar`: same order. Menus list the registry's commands placed directly under File/Edit/View. Search lists matching commands by name, label or alias; Enter or a click runs one. On Windows/Linux the window has no OS frame (`with_decorations(false)`): the bar drags the window, double-click maximizes, the edges resize. macOS keeps its traffic lights. |
| Row 1 — main toolbar 50: Text Style, Dim Style, Layer; Select, Line, Circle, Arc, Polyline, Rectangle, Move, Trim, Dimension; Settings, Preferences | `istcad::main_toolbar`: the pickers list the drawing's tables and run `style.current`, `dimstyle.current`, `layer.current`. Tools start `select`, `line`, `circle`, `arc`, `pline`, `rectang`, `move`, `trim`, `dimlinear`. Settings opens Drafting Settings, Preferences opens Options. |
| Row 2 — drawing tabs + `+` | CADCraft's file tabs (`chrome::file_tabs`). |
| Row 3 — drawing area, auto-hide left strip (20 most-used commands), Properties palette (Ctrl+1) | CADCraft's docking area and canvas; `istcad::quick_bar` adds the strip, ranked by the commands started this session (`Shell::ranked`). Ctrl+1 toggles Properties. |
| Row 4 — command dock: command line, history, Model/Layout tabs, `+`, PDF; OSNAP ORTHO POLAR OTRACK DYN, drafting settings; zoom %, −, +, Ext., slider, GRID | `istcad::command_dock`: layout tabs, New Layout, PDF (`exportpdf`); the drafting toggles (right-click OSNAP for snap modes); zoom readout (100 % = the drawing's first view), zoom buttons, logarithmic zoom slider, GRID. The command line, AutoComplete and history (F2) are CADCraft's, above the dock. |

## Colours

`theme::Tokens::DARK` uses the iStCAD palette: chrome `#292F38` / `#30363F`, panels `#363C45`,
controls `#414750` / hover `#4A515B`, borders `#555B65`, text `#E8ECEE` / `#B7BCC5`, accent teal
`#55C7C0`, picked-point orange `#F2A56F`, model space `#121E38` with grid `#1B2742` / `#283754`.
The Light theme is CADCraft's.

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

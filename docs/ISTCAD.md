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
| Drawing area `#192429`, `#626973` frame; grid `#212C34` / every fifth `#34404F`; auto-hide handle 12×58 `#4A515B` "›", strip 60 wide with 34×32 buttons for the 20 catalog commands ranked by use | Theme tokens `canvas`, `grid_minor`, `grid_major`, `axis_*`; `canvas_frame`; `quick_bar` with iStCAD's `FrequentCommandCatalog` order (LINE, CIRCLE, RECTANGLE, ARC, MOVE, COPY, ERASE, SELECT, ZE, ZP, PAN, UNDO, REDO, NEW, OPEN, SAVE, SAVEAS, PLOT, LAYOUT, REGEN). |
| Properties palette (Ctrl+1), 1.0.233: column 296 on the right, hidden at start, `#313741`, padding 6, resizable; "PROPERTIES"; selection box ("No selection" / type / "N objects selected"); groups General, Geometry, Text, View (closed), Misc under 24-point `#262B33` header bars with ◀ / ▼; rows 30 high, label `#BEC4CD` 1 part, value `#E1E3E8` right-aligned 1.15 parts. Toggling it is noted in the command history. | `istcad_props.rs`: the same, toggled by Ctrl+1, PROPERTIES, PR and PROPS (`ui.istcad.properties {"on"}` for automation). Editing follows AutoCAD's palette: click a value to type (Enter or leaving the field applies, Esc cancels), pick from a list (Layer, Color, Linetype, Lineweight, styles, justification, hatch pattern…), or flip a Yes/No. Every selected object of the described type changes; values they don't share read "Multiple". The groups are the engine's (`properties.describe`): Geometry for lines (Start/End X Y Z, Delta, Length, Angle), circles (Center, Radius, Diameter, Circumference, Area), arcs (Center, Radius, Start/End/Total angle, Arc length), polylines (Current vertex with Vertex X Y, Global width, Elevation, Area, Length; Misc Closed, Linetype generation), points, ellipses (Center, Major/Minor radius, Radius ratio), block references (Position, Scale X Y Z; Misc Name, a list that switches the block, and Rotation); leaders (Lines & Arrows: Arrow, Path type; Geometry: Current vertex, Vertex X Y; Misc: Dim style); multileaders (Leaders: Leader type, None included (Straight or Spline gives one back a leader line), Arrowhead size, Horizontal landing, Landing distance; Text: Contents, Text style, Justification, Text height, Text rotation, Line space factor, Text width; Block: Source block, Block scale, Block rotation; Geometry: Landing X Y, moving the text with it; Misc: Multileader style, Annotative); View (Center X Y, Height: typing zooms there); Pattern (Type, Pattern name, Angle, Scale, Origin, Background color, Associative, Island detection style), Gradient and Geometry (Elevation, Area) for hatches; an Attributes group with each tag of the selected block references (ATTEDIT); splines (Degree, Closed, counts, Length, Area, fit points or control vertices); rays and xlines (Basepoint, Second point, Direction vector); tables (Table style, Rows and Columns (added or taken away at the end), Table width and height, Position); viewports (Center, Width, Height, On, Display locked, Standard and Custom scale, Twist angle); images (Position, Rotation, Width, Height, Brightness, Contrast, Fade, Show image, Show clipped); wipeouts, solids, traces, 3D faces and 3D polylines (their vertices); regions (Area, Perimeter); multilines (read only; MLEDIT changes them); tolerances (Dim style, Text, Position); helixes (Position, radii, Height, Turns, Twist); attribute definitions (Tag, Prompt, Value, the text's rows and modes); Text and Geometry for text and mtext; Misc and Lines & Arrows … Tolerances for dimensions. Each X row with its Y row has a pick button (shown when pointed at): the command line asks for a point and both rows take it. Text contents open in a box of several lines under the row (Enter: new line, Ctrl+Enter or clicking elsewhere applies). Beside the selection box: PICKADD (+ / 1), Select objects and Quick Select. With nothing selected, General sets what new objects get: layer, color, linetype, lineweight, linetype scale, thickness. Empty groups are left out. |
| Command dock 86, two rows, columns 4.35 : 1.05. Left: command bar 40 (`#3B414A`, "›", "Command:", input, status 10 pt ≤ 260, ⌃ history), then Model/layout tabs (selected `#565D67`, others `#414750`), `+` 27, PDF 36×24. Right: OSNAP ORTHO POLAR OTRACK DYN (18 high, on `#AEB4BE` with `#292E36` text) + drafting settings; then ▧ ▣ ⚙, zoom %, −, +, Ext. 45, slider 68, GRID 38×22. AutoComplete 430 wide above the input (name column 130, description `#9FADB2`, selected `#5A6069`); history 196 high above the dock (`#182226` at 95 %). | `command_dock`, `command_bar`, `autocomplete`, `history_window`, `layout_tabs`, `drafting_toggles`, `zoom_controls`: as 1.0.233 (AutoComplete `#343A43` ≤ 260 × 190, names 10 pt; history 137 high, Consolas 12, selectable), the row above reads OSNAP ORTHO POLAR OTRACK, and DYN and AutoCAD's GRID, SNAP, LWT and QP toggles take the place of the three view glyphs left of the zoom readout (GRID from the right end, DYN from the row above). The command line shows the running command's prompt with clickable options; typing, Enter/Space, Tab, arrows and F2 keep CADCraft's command-line handling. The zoom readout's 100 % is how the drawing first showed. |

## Drawing area behaviour

- **Dynamic input (DYN)**: iStCAD's panel, 242 wide, 18 points below-right of the pointer and
  kept 8 points inside the drawing area (`#363C45` at 95 %, `#AEB4BE` border): the prompt (10 pt,
  with options and default) over "↳" and the value field (`#4A515B`). It holds what is typed, or the
  live value under the pointer, dimmed; "Command:" while a command name is typed with nothing
  running. (`canvas/dyn_boxes.rs`, `istcad_dynamic_input`.)
- **Object snap marker**: iStCAD's shapes in `#55E6A5` with the short mode name (END, MID, CEN,
  QUAD, INT, INS…) on a dark tag 8 points right and 5 below. (`canvas/cursor.rs`.)
- **Selection, as in AutoCAD**: a click on empty space starts the rectangular window (second
  click closes it; left to right = window, right to left = crossing). Pressing and dragging from
  empty space draws a **lasso** of any shape: dragged to the right it is a window lasso (blue,
  solid: objects entirely inside), to the left a crossing lasso (green, dashed: inside or
  crossed); **Space** while dragging cycles Window → Crossing → Fence (objects the path crosses).
  Shift removes from the selection. It works with no command running and at "Select objects".
  Typed WP / CP / F at "Select objects" still give polygon windows and fences.
  (`canvas/lasso.rs`; engine `select::LassoKind`, `Session::lasso_select`.)

## Start-up and saved state

With the shell on, the app opens as iStCAD does: no tool sets, no docked palettes, no view cube
(CADCraft's panels stay in the Window menu). The auto-hide strip's usage counts are saved with
the preferences (`istcadUsage`), as iStCAD keeps its command usage between sessions.

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

## Performance work (DWG open, save and edit)

Measured on three customer DWGs (3–11 MB, 3k–27k objects; files not in the repository) with
`cadcraft-cli perf FILE` and `convert`, on a two-core Linux runner:

| | Before | After |
|---|---|---|
| Display list after an edit (KC, 26.7k objects) | 62–66 ms, whole list | 8–10 ms, one 1024-object chunk (`ChunkCache`), same list |
| GPU upload after an edit | whole mesh | only the changed chunk's buffers |
| Save as DWG 2007 (KET_CAU_42B) | ~2.2 s | ~1.2 s |
| Save as DWG 2007 (Q8, 10 MB) | ~1.8 s | ~1.35 s |

Save output is byte-identical before and after (checked with `cadcraft-cli roundtrip`).
What made saving slow (callgrind): a quadratic ENDBLK lookup per block, the drawing written
three times for a 2007 file with legacy visual styles, and hex text of binary chunks normalised
one character at a time. What remains is mostly acadrust's DXF reader and its 2007 page
compressor in the DWG worker.

## Plot (iStCAD 1.0.231–1.0.233 `ShowPlotDialog`)

The iStCAD window opens iStCAD's Plot (PDF) dialog for PLOT, PRINT and Ctrl+P (`istcad_plot.rs`): Paper (ISO A0–A5, ANSI A–D, Custom; margin; Auto / Portrait / Landscape), Plot layout (Model: Extents, Display, Window picked in the drawing; a layout: its sheet; Fit to paper or 1:1 … 5:1 or Custom mm = units; Center the plot or an X/Y offset; the fitted scale), Plot style (Color, Grayscale, Black; object lineweights; open the PDF after plotting), a live preview a quarter second after each change, and All layouts → PDF, Apply to layout, Plot PDF, Close. Not there yet: iStCAD's custom pen table (shown disabled), scaled and default lineweights, and the layout viewport card.

# Changelog

Bingrove is a Meshgrove plugin: a release is one `bingrove.wasm`, installed under *Settings →
Plugins*. It needs a Meshgrove with the plugin system (0.8.0 or newer).

## 0.5.0 – 2026-11-09

Walls, reworked. **Wall plates and wall parts from 0.4 do not fit those from 0.5**: the hook,
the lift and the magnets moved. Print plate and parts with the same version.

- Two wall hooks per cell, one low and one high, so a hanging part tips out neither at the top
  nor at the bottom (with one low hook, the top of a part rocked away from the wall). Each holds
  more and prints cleanly: wider and taller, its finger joined to the stem by a 45° gusset, so
  the plate prints it with only a short free overhang. The keyholes in the feet are now the
  hooks' real way, with a lead-in on the tooth and a millimetre to spare past the stop. Wall
  parts are 4 mm short at the top and go in pushed up to their stop; anything from about 2 mm of
  lift works.
- A latch: every cell of a hook plate has a spring blade beside the hooks. It stands from the
  bed in a slot through the floor, joined to the plate only at its root, so it really bends
  (printed upright, along its layers). The lowest two outer feet of a wall part have a rib that
  clicks over its nose on the way down and holds against lifting, with hardly any play once
  the part has settled. Lift firmly to take a part off. *Output* takes the latch strength and the hook play on walls; 0 leaves the latch out.
- Calibration: the wall cell gets the blade, and three blocks with a softer or firmer latch
  (0.3, 0.5, 0.7) join the three hook blocks.
- The 3D preview shows the space you plan for, in blue round the assembly: the inside of the
  drawer, the surface with the room above it, or the wall with the room in front of it. Sides
  the plate or the bins stick out of turn red. Needs Meshgrove 0.9.0; older versions show the
  preview as before.
- Surfaces and walls take their real size in mm next to the cells. A new size takes as many
  cells as it holds; cells set by hand stay, and if they are more than the size holds the plan
  warns (`grid_larger_than_space`) and the preview shows where the plate sticks out. Before,
  the size silently followed the cells, so a grid planned too large for its shelf only showed
  after printing.
- Wall **hoppers** (`hopper`): a wall bin with a low front and sides that slope up to the wall.
- Wall **hooks and brackets** (`wall_hooks`): arms out of the part's bottom with their tips up,
  one, two, a fork for a hammer's handle, or one wide bracket for cables and hoses.
- Wall **quivers** (`wall_quivers`): thin tubes for rulers, long bits, brushes, pens and spray
  cans, open at the top and optionally below.
- New sets: `wall_hook_single`, `wall_hook_double`, `wall_hammer`, `wall_cable_saddle`,
  `wall_quiver_pens`, `wall_quiver_brushes`, `wall_quiver_long_bits`, `wall_spray_cans`.
- The panel speaks English too: it follows Meshgrove's language, including the names of holders
  and sets. File, folder and print list names stay German.
- Panel: after generating it says the files were handed to Meshgrove, and which setting to
  check if nothing arrives, instead of a success message that also showed when Meshgrove had
  refused them. From Meshgrove 0.8.2 on, the real result replaces that line: in the library,
  waiting for review, refused (with the setting to change) or failed.
- Plate connectors no longer collide where four pieces meet. They sat next to the start of
  each cut, so at a crossing the tabs from below and from the left ended up in the same corner
  of the fourth piece, and a pocket cut into the countersink of the corner screw. Now they sit
  in the middle of every other cell along a cut, half a cell from any crossing and screw; on a
  skeleton floor the pocket keeps floor around it. Plates printed with earlier versions do not
  fit plates printed with this one.

## 0.4.0 – 2026-11-03

Walls.

- Kind **wall**: a plate on the wall with a hook on every cell's floor, magnets where the wall
  parts have theirs, screw holes in the middle of the corner cells. Wall parts are 3 mm shorter
  at the top, go in lifted and slide down onto the hooks, which then keep them from being pulled
  straight off; without hooks they hold by magnets and any bin fits.
- Wall parts, made lying on their bottom face: **wall bins** (open boxes, compartments, open
  front, text), **standing holes** (leaning, or open below for screwdrivers hanging through),
  **stepped sockets** for extension bars leaning against the wall, **V-slots** for pliers
  standing jaws down. Sets: extensions 1/4", 3/8", 1/2" and all, hex keys, screwdrivers, pens,
  pliers, scissors, wrenches 6–24.
- **IKEA Skadis**: pockets in the plate's top edge and printed hooks that slide in from behind.
- The assembly stands up in the preview and in the 3MF; calibration adds a wall plate cell and
  blocks with 0.2/0.3/0.4 mm around the hook (profile `hook_gap`).
- Panel: *Art → Wand* with cells across and up, the reach out of the wall, hooks, screws,
  connectors and Skadis; wall bins and holders get their own fields. Tools: `kind: wall`,
  `hooks`, `mount.skadis`, holders `wall_holes`, `wall_cradles`, `wall_vslots`.
- Parts table: the picked part's whole name and exact size underneath.
- Fixed: V-slot holders came out as a nearly solid block (the slots were cut in front of
  them); the dovetail connectors between plate pieces could not be put together (tabs now stand
  on the build plate and pockets open below); the tools' `vslots` was not accepted (`v_slots`
  still is).

## 0.3.0 – 2026-11-03

More holders, more bin features, lids, plate options.

- Holders: sockets on square **pegs** by drive (1/4", 3/8", 1/2") as an option of every row of
  holes, with a calibration bar of nine pegs; **cradles** with sloped, stepped beds for extension
  bars, ratchets and files; **V-slots** for pliers, scissors and wrenches, on a ramp when asked;
  rows of holes **leaning back**; **rings** for bottles, spray cans and spools. Sets for all of
  them (sockets on pegs, extensions, ratchets, pliers, scissors, wrenches 6–24, hex keys and pens
  tilted, cans, bottles, spools).
- Bins: scoop and label shelf on any wall, open front, finger notches in the divider walls, a
  slanted window in the front wall, and a push-on **lid** with the stacking lip on top (bins
  stack on it) and a plug with click bumps below, one lid part per footprint.
- Plate: floor as skeleton or full, magnet pockets underneath for steel surfaces.
- Tools: `holder` kinds `cradles`, `vslots`, `rings`, `row` with `pegs` and `tilt`; placement
  fields `label_side`, `scoop_sides`, `open_front`, `finger_cutouts`, `window`, `lid`; plate
  `style` and `magnets_below`.

## 0.2.0 – 2026-11-02

Workbench surfaces, holders with their own sizes, engraving, the panel in tabs.

- Surfaces: a plate that is the grid itself, cells left free where nothing stands; solid floor,
  countersunk M4 screw holes, dovetail connectors between the pieces (with a calibration pair).
- Holders: hole grids with diameter, depth, pitch, count and shape (round, hex, square); rows of
  holes with their own sizes and the size engraved in front of each, with sets (sockets on 1/4",
  3/8" and 1/2" drives, metric and inch, hex keys, drills, standing screwdrivers) and stepped
  rows; slots for cards, swatches, coin cells and blades.
- Engraving with the Hershey Simplex stroke font: label shelf or front wall of a bin, titles on
  holders, the calibration ladder in plain numbers.
- Panel: tabs Projekt | Belegung | Ausgabe; a bin's settings under the grid on a click;
  compartment count with suggested mixes of bin sizes; the 3D preview as a job without a time
  limit.
- Edge bins can be switched off (the margin stays plate); plan tool takes `bins`, `kind`,
  `cells`, `cover`, `floor`, `mount`.

## 0.1.0 – 2026-11-01

First plugin: drawers planned from their inner size, edge bins that use wide margins, bins with
dividers, label shelf, scoop, magnets and cutouts (box, cylinder, polygon, silhouette of a
library model), holders for tape rolls, batteries, bits, pens and screwdrivers, calibration
pieces, STL per part and 3MF per print bed, print lists in Meshgrove, tools for the AI
(`bingrove_options`, `bingrove_plan`, `bingrove_generate`), a CLI.

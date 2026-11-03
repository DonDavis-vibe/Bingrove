# Changelog

Bingrove is a Meshgrove plugin: a release is one `bingrove.wasm`, installed under *Settings →
Plugins*. It needs a Meshgrove with the plugin system (0.8.0 or newer).

## Unreleased

- Panel: after generating it says the files were handed to Meshgrove, and which setting to
  check if nothing arrives, instead of a success message that also showed when Meshgrove had
  refused them. A Meshgrove that reports the outcome replaces that line with the real result:
  in the library, waiting for review, refused (with the setting to change) or failed.

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

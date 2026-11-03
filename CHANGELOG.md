# Changelog

Bingrove is a Meshgrove plugin: a release is one `bingrove.wasm`, installed under *Settings →
Plugins*. It needs a Meshgrove with the plugin system (0.8.0 or newer).

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

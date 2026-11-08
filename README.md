<p align="center"><img src="assets/banner.png" alt="Bingrove" width="100%"></p>

# Bingrove

**Gridfinity for drawers, workbenches and walls.** Give Bingrove a size and what should go in;
it plans the baseplate and the bins and holders, makes the files (an STL per part, a 3MF per
print bed) and puts them on print lists. It runs as a plugin in
[Meshgrove](https://github.com/DonDavis-vibe/MeshGrove), from the command line, or through an AI
such as Claude.

*Deutsche Fassung: [README.de.md](README.de.md)*

## Three kinds of project

### Drawer: use the whole drawer

A 42 mm grid rarely fits a drawer. A 250 × 305 mm drawer takes a 5 × 7 grid and leaves
**19.5 mm on the left and right** that hold nothing. Bingrove keeps the grid and lets the bins
next to a wide margin reach into it (*edge bins*, `randbin_…`). The inner bins stay plain
Gridfinity, so any bin from anywhere still fits. In that drawer the usable area goes from 83 % to
97 %.

```
back
 #################        ~  edge zone the bins reach into
 ~ A  A  B  B  C ~        #  solid plate margin (too narrow to use)
 ~ A  A  B  B  C ~
 ~ D  E  E  F  F ~        A: randbin_2x2_12u_L19.25.stl   D: randbin_1x5_6u_L19.25.stl
 ~ D  E  E  F  F ~        B: bin_2x2_12u.stl              E: bin_2x3_6u.stl
 ~ D  E  E  G  G ~        C: randbin_1x2_12u_L19.25.stl   …
 ~ D  I  K  M  O ~
 ~ D  H  J  L  N ~
 #################
front
```

The overhang either goes **down to the drawer floor** on a skirt (default, prints without
supports, 4.75 mm more height), **rests** on the plate's margin (needs supports), or there are
**no edge bins** and the margin stays plate. Margins under 10 mm (adjustable) always stay plate,
so the insert does not slide.

### Surface: a workbench or desk

The plate is the grid itself, cells × 42 mm, with no margins. Cells stay free wherever nothing
is put. The plate gets a solid floor (skeleton or full), countersunk M4 screw holes, dovetail
connectors between its pieces and, for steel tops, magnet pockets underneath.

### Wall: parts that hang

A plate on the wall with a hook and a spring blade in every cell. Wall parts are 4 mm shorter
at the top: you **push them up to their stop, put them in straight and let them slide down**.
The hooks then keep them from being pulled straight off, and a **latch clicks in**, so a part
does not come along when you pull a tool out upwards. Lift a part firmly to take it out.
Magnets help. Without hooks the parts hold by magnets alone, and then any bin fits.

How firmly the latch holds and how much play the hook has are set under *Output* (*Latch*,
*Hook play*); the calibration pieces show what suits your printer.

The plate is screwed on (holes in the middle of the corner cells) or hung on an **IKEA Skadis**
board with printed hooks that slide into its top edge. Wall parts print lying on their bottom
face, so their pockets open upwards while printing.

## Install

Bingrove needs Meshgrove **0.8.0 or newer**. Use 0.8.2 or newer: number fields there take a value
only after Enter, table columns can be widened, and the panel shows whether your files really
reached the library.

1. Download `bingrove.wasm` and `bingrove.wasm.sha256` from the
   [releases](https://github.com/DonDavis-vibe/Bingrove/releases) and check the download:
   ```bash
   sha256sum -c bingrove.wasm.sha256
   ```
2. Copy `bingrove.wasm` to `~/.local/share/meshgrove/plugins/`.
3. In Meshgrove open *Settings → Plugins*, install Bingrove and grant "AI tools" and "files into
   the library".
4. Open it with *Plugins ▾ → Bingrove*.

To build it yourself, see [Development](#development).

## Using the panel

The panel has three parts:

- **Left**, three tabs. *Project*: kind (drawer, surface, wall), name, sizes, margins and the
  plate. *Layout*: how many compartments you want; Bingrove offers the evenest mixes of bin
  sizes that tile the grid ("10 × 3×1", "5 × 2×2 + 5 × 2×1", …), and one click lays them out.
  *Output*: printer, print lists, magnet sizes. Below the tabs: the grid's summary, warnings,
  and the buttons *Lay out automatically*, *Generate + print list* and *Calibration pieces*.
- **Middle**, the plan as a grid editor. Drag to move, pull an edge to resize, draw new bins on
  free cells, right-click for a menu. A click on a bin opens its settings below the grid: height,
  compartments, label shelf, scoop, engraving, holder kind and the holder's own sizes.
- **Right**, the 3D preview and the parts list. A click on a part shows it on the print bed, with
  its full name and exact size underneath.

**Generate + print list** writes everything to `Gridfinity/<name>` in the library and makes two
print lists: `<name> – Teile` (each STL with its quantity) and `<name> – Druckbetten` (a 3MF per
bed, parts arranged). One click in Meshgrove's history undoes it. The project file
`*.gridfinity.json` reopens with "Mit Bingrove öffnen" from the gallery.

## What it makes

**Plates** are cut along grid lines into pieces that fit your printer: an open frame, or with a
floor and magnet holes. Dovetail connectors join the pieces: lay one piece onto the other's tab.

**Bins** have a stacking lip, compartments, a label shelf and a scoop on any wall, magnet holes,
an open front, finger notches in the dividers and a slanted window. A push-on **lid** carries the
stacking lip on top, so another bin stands on it. Pockets can be cut for particular things: a
box, a cylinder, a polygon, or the outline of a model from your library. Text is engraved in a
stroke font (Hershey Simplex) on the label shelf or the front.

**Holders** replace a bin on its cells. Every one has sets to load and its sizes stay editable:

| Holder | For | You set |
|---|---|---|
| Hole grid | batteries, pens, router shanks, drills | diameter, depth, pitch, count, shape (round, hex, square) |
| Row of holes | sockets, screwdrivers, drills, hex keys | a size per hole (engraved), rows, steps, tilt, or square **pegs** by drive |
| Cradles | extension bars, ratchets, files lying | width × length per place, slope, steps |
| V-slots | pliers, scissors, wrenches standing | thickness × depth per slot, ramp, closed back |
| Slots | cards, SD cards, swatches, coin cells, blades | width, depth, pitch |
| Rings | bottles, spray cans, spools | diameter, height |
| Troughs | pens and screwdrivers lying | diameter, length, handle |
| Bits, tape | 1/4" bits, tape rolls on posts | count, roll size |

Besides open **trays**, a wall takes **hoppers**: low at the front, the sides slanting up to the
wall, like workshop storage bins.

**Wall holders** hang on a wall plate:

| Holder | For | You set |
|---|---|---|
| Standing holes | screwdrivers (hanging through), pens, files | diameter, depth, count, lean away from the wall |
| Leaning sockets | extension bars, hex keys | a size per place (engraved), lean towards the wall, steps |
| V-slots | pliers and scissors standing jaws down | thickness × depth per slot, length, closed front |
| Hooks and brackets | a hammer (fork), cables and hoses (bracket), anything that hangs | arms, gap, reach, tip, arm size |
| Quivers | rulers, long bits, brushes, pens, spray cans | Ø and text per tube, depth, wall, open below |

Sets include sockets on 1/4", 3/8" and 1/2" (metric and inch, in holes or on pegs), hex keys,
drills, screwdrivers, extension bars, ratchets, pliers, scissors, wrenches 6–24, cans, bottles
and spools, and the same for walls, plus single and double hooks, a hammer fork, a cable bracket
and quivers for pens, brushes, long bits and spray cans.

**Calibration pieces** dial in a printer before the big print: a magnet-hole ladder, a fit test
with three bins of different clearance, a connector pair, a bar of socket pegs, and a wall cell
with six blocks: three with different clearance around the hook, three with a firmer or
softer latch.

**Output**: an STL per distinct part with its quantity, a 3MF per print bed, the assembly as 3MF
to look at (a wall stands up), and the project file.

## With an AI

Meshgrove's chat and its MCP server (`meshgrove-mcp`, for Claude Code, Claude Desktop, LM Studio
and others) see three tools:

| Tool | What it does |
|---|---|
| `bingrove_options` | presets, your printers, holders and sets, defaults, limits |
| `bingrove_plan` | plans without making files: grid, parts, a text plan, warnings; fast enough to iterate |
| `bingrove_generate` | makes the files and hands them to Meshgrove as one suggestion, with the print lists |

Some requests and what they become:

- *"250 × 305 × 100 mm drawer: pens, two cable bins at the back, a 60 × 35 × 25 mm bit set, six
  18650 cells"*: a drawer with those items.
- *"A 600 × 300 mm bench plate with a 1/4" socket set and a row of 8 mm router bits"*:
  `kind: surface` with the set `sockets_1_4_mm` and a hole grid.
- *"A wall of 8 × 6 cells for my extension bars, pliers and two trays"*: `kind: wall` with the
  sets `wall_extensions` and `wall_pliers` and two bins.

Photos work through the AI client: it reads the sizes off the picture and passes numbers.

## Command line

The `bingrove` command plans drawers and surfaces; walls are made in the panel or through the
tools.

```bash
cargo build --release
target/release/bingrove presets
target/release/bingrove drawer 250x305x100 --name Kleinkram --inhalt "Stifte:20x180x20" --inhalt "2*Kabel:bin 2x2x12" --zusammenbau
target/release/bingrove drawer 600x300x100 --name Bank --flaeche --plan
target/release/bingrove bin 3 1 -d 3 1 --magnete ecken
target/release/bingrove tape 103 -n 2 1 -s 3
```

Files go to `out/<name>/`; `--plan` only prints the plan.

## Development

```
crates/bingrove-core     planning (grid, margins, split, solver, layout repair, parts list) and,
                         with the feature "geometry", the solids and STL/3MF export
crates/bingrove-cli      the bingrove command
crates/bingrove-plugin   the Meshgrove plugin (cdylib → bingrove.wasm, its own workspace)
assets/                  icon and banner
scripts/                 build.sh, install-into-meshgrove.sh, check.sh, release-notes.sh
```

```bash
rustup target add wasm32-wasip2
scripts/check.sh
scripts/build.sh
scripts/install-into-meshgrove.sh
```

`check.sh` runs fmt, clippy (native and wasm), the tests and the plugin build; `build.sh` writes
`target/plugin/bingrove.wasm`, and `install-into-meshgrove.sh` copies it into Meshgrove's plugin
folder.

The plugin takes `meshgrove-plugin-api` from the Meshgrove repository, pinned to a release tag.
To work on both at once, the comment in `crates/bingrove-plugin/Cargo.toml` shows how to point it
at a local checkout. CI runs `check.sh` on every push; a tag `v*` builds the plugin and publishes
it with its checksum and the changelog entry as a GitHub release. Changes per version:
[CHANGELOG.md](CHANGELOG.md).

Geometry runs on [manifold-rust](https://crates.io/crates/manifold-rust), a pure-Rust port of
Manifold, so there is no C++ toolchain and the same code builds for the plugin sandbox. The
Gridfinity profiles follow
[gridfinity-rebuilt-openscad](https://github.com/kennetek/gridfinity-rebuilt-openscad). The
drawer logic grew out of a Python script for two drawers, whose 25 parts the tests reproduce to
±0.01 mm.

## Known limits

- Presets describe drawers only; a workbench starts from *Kind → Surface*, a wall from *Kind → Wall*.
- Bins are rectangles. L-shaped bins, wall patterns, lite bins, sliding lids, half-grid and a
  case with a lid are not there yet.
- Long holders (a whole socket set) are not split for the print bed; the plan warns.
- The outline of a library model is its convex hull, so hollows in it get filled.
- Wall parts print on their bottom face, so round holes come out slightly oval. With hooks on
  the plate only wall parts fit.
- The latch's spring blades are made for PLA; PETG creeps less if parts stay on for years.
- Skadis hooks hold the plate by its top edge only (the board's slots are 40 mm apart, the grid
  42 mm).
- The panel follows Meshgrove's language, German or English. File, folder and print list names
  stay German, so existing lists keep working, and so does the command line.

## License

MIT or Apache-2.0, at your option (`LICENSE-MIT`, `LICENSE-APACHE`). The Gridfinity profiles
follow the published spec; Gridfinity is by Zack Freedman, and Bingrove is not affiliated with
him or with gridfinity.xyz. The engraving uses the Hershey Simplex font: the Hershey Fonts were
originally created by Dr. A. V. Hershey while working at the U. S. National Bureau of Standards;
the format of the font data in this distribution was originally created by James Hurt,
Cognition, Inc.

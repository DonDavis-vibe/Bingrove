<p align="center"><img src="assets/banner.png" alt="Bingrove" width="100%"></p>

# Bingrove

**Gridfinity drawer inserts that use the whole drawer.** Bingrove plans a baseplate and bins
from a drawer's inner size, makes the files (STL per part, 3MF per print bed) and puts them on
print lists — as a plugin for [Meshgrove](https://github.com/DonDavis-vibe/MeshGrove), from the
command line, or through an AI such as Claude.

*Deutsche Fassung: [README.de.md](README.de.md)*

## The margin problem

A 42 mm grid rarely fits a drawer. A 250 × 305 mm drawer takes a 5 × 7 grid (210 × 294 mm), which
leaves **19.5 mm on the left and right** — a solid plate margin that holds nothing.

Bingrove keeps the grid where it is and lets the bins next to a wide margin reach into it
("edge bins", *Randbins*). The inner bins stay 100 % Gridfinity, so any bin from anywhere still
fits; only the bins at the edge are a little wider than 42 n. For that drawer the usable area goes
from 83 % to 97 %.

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

Two ways to carry the overhang:

- **To the floor** (default): the plate ends just past the grid, the edge bin's overhang goes
  down to the drawer floor on its own skirt. Prints without supports and gains 4.75 mm of usable
  height in the margin.
- **Resting**: the plate keeps its margin and the overhang rests on it. Needs supports; for
  drawers where the plate must hold itself tight.
- **None**: no edge bins at all. The margins stay solid plate (hollowed from below when wide
  enough) and every bin is a plain 42 n one, as the original script made them.

Margins narrower than a threshold (10 mm by default) stay solid plate, which keeps the insert
from sliding around.

## What it makes

- **Baseplate** pieces split along grid lines to fit the printer bed, open frame or with a floor
  and magnet holes, hollowed from below in wide margins.
- **Bins** with stacking lip, dividers (equal or on grid lines), label shelf, finger scoop,
  magnet holes (corners or every cell) and pockets cut for particular things (box, cylinder,
  polygon).
- **Edge bins** on any side, including corners.
- **Holders**: tape rolls on ring posts, batteries (AA, AAA, C, D, 18650, 21700, custom), 1/4"
  bits, pens standing or lying, screwdrivers in troughs with a wider handle rest.
- **Calibration pieces**: a magnet-hole ladder labelled with its diameters and a fit test with
  three bins of different clearance, to dial in a printer.
- **Output**: one STL per distinct part with quantities, 3MF files with the parts arranged per
  print bed, the assembled drawer as 3MF to look at, and a `*.gridfinity.json` project file that
  reopens the drawer.

## As a Meshgrove plugin

Bingrove is a WebAssembly component (`wasm32-wasip2`) that Meshgrove runs in a sandbox.

```sh
rustup target add wasm32-wasip2          # once
scripts/build.sh                         # → target/plugin/bingrove.wasm
scripts/install-into-meshgrove.sh        # copies it into Meshgrove's plugin folder
```

In Meshgrove: *Settings → Plugins* — install, grant "AI tools" and "files into the library" —
then *Plugins ▾ → Bingrove*. The panel has the drawer's form on the left, the plan as a grid
editor in the middle (move, resize, merge, split, heights, dividers, holders by context menu),
the 3D preview and parts list on the right. **Generieren + Druckliste** writes everything into
`Gridfinity/<drawer>` in the library and makes the print lists `<drawer> – Teile` (STL with
quantities) and `<drawer> – Druckbetten` (3MF per bed); one click in the history undoes it.

Project files (`*.gridfinity.json`) open with "Mit Bingrove öffnen" from the gallery.

### With an AI

Meshgrove's chat and its MCP server (`meshgrove-mcp`, for Claude Code, Claude Desktop, LM
Studio …) see three tools:

| Tool | What it does |
|---|---|
| `bingrove_options` | presets, the user's printers, holder catalogue, defaults, limits |
| `bingrove_plan` | plans a drawer without making files: grid, margins, parts, a text plan, warnings; fast enough to iterate |
| `bingrove_generate` | makes the files and hands them to Meshgrove as a bundle with the print lists |

A request like *"250 × 305 × 100 mm drawer: pens, two cable bins at the back, a 60 × 35 × 25 mm
bit set, six 18650 cells"* becomes a plan, then a suggestion the user accepts in Meshgrove.
Photos work through the AI client: it reads the sizes off the picture and passes numbers.

## From the command line

```sh
cargo build --release
target/release/bingrove presets
target/release/bingrove drawer 250x305x100 --name Kleinkram \
    --inhalt "Stifte:20x180x20" --inhalt "2*Kabel:bin 2x2x12" --inhalt "Bits:60x35x25" --zusammenbau
target/release/bingrove --schublade 250x305            # the preset with its fixed layout
target/release/bingrove bin 3 1 -d 3 1 --magnete ecken  # one bin
target/release/bingrove tape 103 -n 2 1 -s 3            # a tape holder
```

Output goes to `out/<name>/`. `--plan` only prints the plan.

## Repository

```
crates/bingrove-core     planning (grid, margins, split, solver, layout repair, parts list) and,
                         with the feature "geometry", the solids and STL/3MF export
crates/bingrove-cli      the bingrove command
crates/bingrove-plugin   the Meshgrove plugin (cdylib → bingrove.wasm)
assets/                  icon and banner
scripts/                 build.sh, install-into-meshgrove.sh, check.sh
```

Geometry runs on [manifold-rust](https://crates.io/crates/manifold-rust), a pure-Rust port of
Manifold, so there is no C++ toolchain and the same code builds for the browser-sized sandbox.
The Gridfinity profiles follow
[gridfinity-rebuilt-openscad](https://github.com/kennetek/gridfinity-rebuilt-openscad); the
drawer logic grew out of a Python script for two particular drawers, whose 25 parts the tests
reproduce to ±0.01 mm.

```sh
scripts/check.sh    # fmt, clippy (native and wasm), tests, plugin build
```

The plugin depends on `meshgrove-plugin-api` from a Meshgrove checkout next to this repository
until that crate is published.

## Status

Working: planning, edge bins, holders, cutouts (including the silhouette of a library model),
calibration, STL/3MF/project export, CLI, plugin tools and panel. Coming: a browser build, more
holder kinds, concave silhouettes.

## License

MIT or Apache-2.0, at your option (`LICENSE-MIT`, `LICENSE-APACHE`). The Gridfinity profiles
follow the published spec; Gridfinity is by Zack Freedman, and Bingrove is not affiliated with
him or with gridfinity.xyz.

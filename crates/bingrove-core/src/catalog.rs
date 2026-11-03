//! What holders there are and how big they come out, for the solver, the panel and the AI:
//! single holders (`catalog`) and sets of sizes for rows of holes (`sets`).

use serde::Serialize;

use crate::model::{
    BIT_ACROSS_FLATS, BIT_CLEARANCE, CradleSlot, CradleSpec, Drive, HolderSpec, HoleShape, HolesSpec, RingSpec,
    RoundItem, RowHole, RowSpec, SlotSpec, TroughSpec, VSlot, VSlotSpec, WallCradleSpec, WallHolesSpec, WallVSlotSpec,
};
use crate::spec::{GRID, WALL, bin_outer};

/// Wall between two holes of a pattern.
pub const HOLE_WALL: f64 = 1.6;
/// Air between the pattern and the bin wall.
pub const PATTERN_MARGIN: f64 = 0.8;
/// Room in front of a row of holes for the engraved sizes.
pub const ROW_LABEL_STRIP: f64 = 5.5;

/// The inside of a bin of `cells` along one axis, less the pattern margin.
fn inner(cells: u32) -> f64 {
    bin_outer(cells) - 2.0 * WALL - 2.0 * PATTERN_MARGIN
}

/// How many of a pitch fit into the inside of a bin of `cells` along one axis.
fn fit(cells: u32, pitch: f64) -> u32 {
    ((inner(cells) + HOLE_WALL) / pitch).floor().max(0.0) as u32
}

/// Cells needed along one axis for `mm` of pattern inside the walls.
fn cells_needed(mm: f64) -> u32 {
    ((mm + 2.0 * WALL + 2.0 * PATTERN_MARGIN + GRID - bin_outer(1)) / GRID).ceil().max(1.0) as u32
}

/// The widest extent of a hole of shape `shape` and size `d` (across flats for hex).
pub fn across(shape: HoleShape, d: f64) -> f64 {
    match shape {
        HoleShape::Round | HoleShape::Square => d,
        HoleShape::Hex => d * 2.0 / 3f64.sqrt(),
    }
}

/// Hole pitch of a holes pattern.
pub fn hole_pitch(spec: &HolesSpec) -> f64 {
    spec.pitch.unwrap_or_else(|| across(spec.shape, spec.item.size().0 + spec.clearance) + HOLE_WALL)
}

pub fn bit_pitch() -> f64 {
    // Across corners of the hex plus the wall.
    (BIT_ACROSS_FLATS + BIT_CLEARANCE) / 3f64.sqrt() * 2.0 + HOLE_WALL
}

/// Holes across and down for a holder of `cells`.
pub fn holes_grid(spec: &HolesSpec) -> [u32; 2] {
    spec.count.unwrap_or_else(|| {
        let pitch = hole_pitch(spec);
        spec.cells.map(|c| fit(c, pitch))
    })
}

pub fn bits_grid(cells: [u32; 2]) -> [u32; 2] {
    cells.map(|c| fit(c, bit_pitch()))
}

/// Troughs across the bin (along y) for a trough holder.
pub fn trough_count(spec: &TroughSpec) -> u32 {
    spec.count.unwrap_or_else(|| {
        let pitch = spec.handle.unwrap_or(spec.diameter) + 1.0 + HOLE_WALL;
        fit(spec.cells[1], pitch)
    })
}

/// Slots across the bin (along x) for a slot holder.
pub fn slot_count(spec: &SlotSpec) -> u32 {
    spec.count.unwrap_or_else(|| {
        let room = inner(spec.cells[0]);
        if room < spec.width { 0 } else { ((room - spec.width) / spec.pitch).floor() as u32 + 1 }
    })
}

/// Front-to-back pitch of one row of a row holder: its widest hole plus wall, plus the label
/// strip when the holder carries text.
pub fn row_pitch(spec: &RowSpec, row: &[RowHole]) -> f64 {
    let widest = row.iter().map(|h| across(spec.shape, h.d + spec.clearance)).fold(0.0, f64::max);
    widest + HOLE_WALL + if spec.has_labels() { ROW_LABEL_STRIP } else { 0.0 }
}

/// Width and depth the rows of a row holder need inside the bin.
pub fn row_extent(spec: &RowSpec) -> [f64; 2] {
    let width = spec
        .rows
        .iter()
        .map(|r| r.iter().map(|h| across(spec.shape, h.d + spec.clearance) + HOLE_WALL).sum::<f64>() + HOLE_WALL)
        .fold(0.0, f64::max);
    let depth = spec.rows.iter().map(|r| row_pitch(spec, r)).sum::<f64>() + HOLE_WALL;
    [width, depth]
}

/// The smallest footprint for a row holder.
pub fn row_cells(spec: &RowSpec) -> [u32; 2] {
    row_extent(spec).map(cells_needed)
}

/// Capacity of a holder: holes, bits, troughs, slots or tape stacks.
pub fn capacity(h: &HolderSpec) -> u32 {
    match h {
        HolderSpec::Tape(t) => t.stacks[0] * t.stacks[1],
        HolderSpec::Holes(s) => holes_grid(s).iter().product(),
        HolderSpec::Bits(b) => bits_grid(b.cells).iter().product(),
        HolderSpec::Troughs(t) => trough_count(t),
        HolderSpec::Row(r) => r.hole_count(),
        HolderSpec::Slots(s) => slot_count(s),
        HolderSpec::Cradles(c) => c.slots.len() as u32,
        HolderSpec::VSlots(v) => v.slots.len() as u32,
        HolderSpec::Rings(r) => rings_grid(r).iter().product(),
        HolderSpec::WallHoles(w) => w.count.iter().product(),
        HolderSpec::WallCradles(w) => w.slots.len() as u32,
        HolderSpec::WallVSlots(w) => w.slots.len() as u32,
    }
}

/// The holder made for `count` things: a footprint that holds them, or for wall holes as many
/// holes (in rows of at most eight) on the smallest part.
pub fn for_count(h: HolderSpec, count: u32) -> HolderSpec {
    match h {
        HolderSpec::WallHoles(w) => {
            let n = count.max(1);
            let rows = n.div_ceil(8);
            HolderSpec::WallHoles(WallHolesSpec { count: [n.div_ceil(rows), rows], ..w }.fit())
        }
        HolderSpec::WallCradles(_) | HolderSpec::WallVSlots(_) => h,
        _ => {
            let cells = cells_for(&h, count);
            h.with_cells(cells)
        }
    }
}

/// Width and depth the slots of a V-slot holder need inside the bin.
pub fn vslot_extent(spec: &VSlotSpec) -> [f64; 2] {
    let width = spec.slots.iter().map(|s| s.width + spec.clearance + HOLE_WALL).sum::<f64>() + HOLE_WALL;
    [width, 20.0]
}

pub fn vslot_cells(spec: &VSlotSpec) -> [u32; 2] {
    let [w, _] = vslot_extent(spec);
    [cells_needed(w), spec.cells[1].max(1)]
}

/// Centre distance of the rings of a ring holder.
pub fn ring_pitch(spec: &RingSpec) -> f64 {
    spec.d + spec.clearance + 2.0 * RING_WALL + HOLE_WALL
}

/// Rings across and down for a ring holder of `cells`.
pub fn rings_grid(spec: &RingSpec) -> [u32; 2] {
    spec.count.unwrap_or_else(|| spec.cells.map(|c| fit(c, ring_pitch(spec))))
}

/// Wall of a ring.
pub const RING_WALL: f64 = 1.6;

/// Width and depth the places of a cradle holder need inside the bin.
pub fn cradle_extent(spec: &CradleSpec) -> [f64; 2] {
    let width = spec.slots.iter().map(|s| s.width + spec.clearance + HOLE_WALL).sum::<f64>() + HOLE_WALL;
    let depth = spec.slots.iter().map(|s| s.length).fold(0.0, f64::max) + 2.0;
    [width, depth]
}

/// The smallest footprint for a cradle holder.
pub fn cradle_cells(spec: &CradleSpec) -> [u32; 2] {
    cradle_extent(spec).map(cells_needed)
}

/// The smallest footprint (by area, then the squarer one) that holds `count` things; a row
/// holder's footprint follows its holes.
pub fn cells_for(h: &HolderSpec, count: u32) -> [u32; 2] {
    match h {
        HolderSpec::Tape(_) => return h.cells(),
        HolderSpec::Row(r) => return row_cells(r),
        HolderSpec::Cradles(c) => return cradle_cells(c),
        HolderSpec::VSlots(v) => return vslot_cells(v),
        h if h.is_wall() => return h.cells(),
        _ => {}
    }
    let mut best: Option<[u32; 2]> = None;
    for area in 1..=36u32 {
        for w in 1..=6u32 {
            if area % w != 0 || area / w > 6 {
                continue;
            }
            let cells = [w, area / w];
            // Troughs want length along x: keep at least the given x cells.
            if let HolderSpec::Troughs(t) = h
                && cells[0] < t.cells[0]
            {
                continue;
            }
            if capacity(&h.clone().with_cells(cells)) >= count
                && best.is_none_or(|b| (cells[0] as i64 - cells[1] as i64).abs() < (b[0] as i64 - b[1] as i64).abs())
            {
                best = Some(cells);
            }
        }
        if best.is_some() {
            break;
        }
    }
    best.unwrap_or([6, 6])
}

/// Cells along x a lying thing of `length` wants (about 60 % of it supported).
pub fn trough_cells_x(length: f64) -> u32 {
    ((length * 0.6 + 2.0 * WALL) / GRID).ceil().max(1.0) as u32
}

/// One entry of the catalogue, for the AI and the panel.
#[derive(Debug, Clone, Serialize)]
pub struct CatalogEntry {
    pub id: &'static str,
    pub label: &'static str,
    pub holder: HolderSpec,
}

pub fn catalog() -> Vec<CatalogEntry> {
    let holes = |item: RoundItem| HolderSpec::Holes(HolesSpec { item, ..HolesSpec::default() });
    let shank = |d: f64, shape: HoleShape, depth: f64| {
        HolderSpec::Holes(HolesSpec {
            item: RoundItem::Custom { d, h: depth / 0.55 },
            shape,
            clearance: 0.5,
            depth: Some(depth),
            ..HolesSpec::default()
        })
    };
    let slots =
        |width: f64, depth: f64, pitch: f64| HolderSpec::Slots(SlotSpec { width, depth, pitch, ..SlotSpec::default() });
    vec![
        CatalogEntry { id: "tape", label: "Tape-Rollen", holder: HolderSpec::Tape(Default::default()) },
        CatalogEntry { id: "aa", label: "AA-Zellen", holder: holes(RoundItem::Aa) },
        CatalogEntry { id: "aaa", label: "AAA-Zellen", holder: holes(RoundItem::Aaa) },
        CatalogEntry { id: "c", label: "C-Zellen", holder: holes(RoundItem::C) },
        CatalogEntry { id: "d", label: "D-Zellen", holder: holes(RoundItem::D) },
        CatalogEntry { id: "18650", label: "18650-Akkus", holder: holes(RoundItem::Cell18650) },
        CatalogEntry { id: "21700", label: "21700-Akkus", holder: holes(RoundItem::Cell21700) },
        CatalogEntry { id: "bits", label: "1/4\"-Bits", holder: HolderSpec::Bits(Default::default()) },
        CatalogEntry { id: "pens", label: "Stifte (stehend)", holder: holes(RoundItem::Custom { d: 12.0, h: 140.0 }) },
        CatalogEntry {
            id: "pens_lying",
            label: "Stifte (liegend)",
            holder: HolderSpec::Troughs(TroughSpec { diameter: 12.0, length: 140.0, ..TroughSpec::default() }),
        },
        CatalogEntry {
            id: "screwdrivers",
            label: "Schraubendreher (liegend)",
            holder: HolderSpec::Troughs(TroughSpec {
                diameter: 8.0,
                length: 200.0,
                handle: Some(30.0),
                cells: [4, 1],
                ..TroughSpec::default()
            }),
        },
        CatalogEntry { id: "shank_6", label: "Fräserschäfte 6 mm", holder: shank(6.0, HoleShape::Round, 22.0) },
        CatalogEntry { id: "shank_6_35", label: "Fräserschäfte 1/4\"", holder: shank(6.35, HoleShape::Round, 22.0) },
        CatalogEntry { id: "shank_8", label: "Fräserschäfte 8 mm", holder: shank(8.0, HoleShape::Round, 25.0) },
        CatalogEntry { id: "shank_12", label: "Fräserschäfte 12 mm", holder: shank(12.0, HoleShape::Round, 30.0) },
        CatalogEntry { id: "shank_12_7", label: "Fräserschäfte 1/2\"", holder: shank(12.7, HoleShape::Round, 30.0) },
        CatalogEntry { id: "dremel", label: "Dremel-Schäfte 3.2 mm", holder: shank(3.2, HoleShape::Round, 15.0) },
        CatalogEntry { id: "hex_5_16", label: "Hex-Schäfte 5/16\"", holder: shank(7.94, HoleShape::Hex, 12.0) },
        CatalogEntry { id: "hex_3_8", label: "Hex-Schäfte 3/8\"", holder: shank(9.53, HoleShape::Hex, 14.0) },
        CatalogEntry { id: "sd", label: "SD-Karten (Schlitze)", holder: slots(2.8, 16.0, 5.0) },
        CatalogEntry { id: "usb", label: "USB-Sticks (Schlitze)", holder: slots(6.0, 14.0, 10.0) },
        CatalogEntry { id: "cr2032", label: "Knopfzellen (Schlitze)", holder: slots(3.8, 14.0, 6.0) },
        CatalogEntry { id: "swatches", label: "Filament-Swatches (Schlitze)", holder: slots(3.0, 24.0, 6.0) },
        CatalogEntry { id: "blades", label: "Sägeblätter (Schlitze)", holder: slots(2.0, 40.0, 5.0) },
        CatalogEntry {
            id: "cradles",
            label: "Wiegen (liegende Werkzeuge)",
            holder: HolderSpec::Cradles(CradleSpec::default()),
        },
        CatalogEntry {
            id: "wall_holes",
            label: "Wand: Löcher stehend",
            holder: HolderSpec::WallHoles(WallHolesSpec::default().fit()),
        },
        CatalogEntry {
            id: "wall_cradles",
            label: "Wand: Wiegen (Stäbe lehnen an der Wand)",
            holder: HolderSpec::WallCradles(WallCradleSpec::default().fit()),
        },
        CatalogEntry {
            id: "wall_vslots",
            label: "Wand: V-Schlitze (Zangen hängend)",
            holder: HolderSpec::WallVSlots(WallVSlotSpec::default().fit()),
        },
        CatalogEntry {
            id: "row",
            label: "Lochreihe (eigene Größen)",
            holder: HolderSpec::Row(RowSpec {
                rows: vec![
                    [6.0, 8.0, 10.0, 12.0, 14.0]
                        .iter()
                        .map(|&d| RowHole { d, depth: None, label: crate::names::mm(d) })
                        .collect(),
                ],
                ..RowSpec::default()
            }),
        },
    ]
}

/// A set of sizes for a row holder: sockets by drive, screwdrivers, hex keys, drills.
#[derive(Debug, Clone, Serialize)]
pub struct HolderSet {
    pub id: &'static str,
    pub label: &'static str,
    pub holder: HolderSpec,
    /// The drive of a socket set, so it can be offered on pegs too.
    #[serde(skip)]
    pub drive: Option<Drive>,
}

/// Typical outside diameter of a six-point socket of nominal `size` mm on a drive, mm.
/// Brands differ by a millimetre or two; the clearance and every hole stay editable.
pub fn socket_od(drive: Drive, size_mm: f64) -> f64 {
    let (min, add) = match drive {
        Drive::Quarter => (11.8, 5.0),
        Drive::ThreeEighths => (16.8, 7.0),
        Drive::Half => (23.5, 8.5),
    };
    ((size_mm + add).max(min) * 10.0).round() / 10.0
}

/// An inch fraction like "1 1/16" as mm.
fn inch(text: &str) -> f64 {
    let mut total = 0.0;
    for part in text.split(' ') {
        total += match part.split_once('/') {
            Some((n, d)) => n.parse::<f64>().unwrap_or(0.0) / d.parse::<f64>().unwrap_or(1.0),
            None => part.parse::<f64>().unwrap_or(0.0),
        };
    }
    total * 25.4
}

fn socket_row(drive: Drive, sizes: &[(&str, f64)]) -> Vec<RowHole> {
    sizes
        .iter()
        .map(|&(label, mm)| RowHole { d: socket_od(drive, mm), depth: None, label: label.to_string() })
        .collect()
}

fn metric(sizes: &[f64]) -> Vec<(String, f64)> {
    sizes.iter().map(|&s| (crate::names::mm(s), s)).collect()
}

fn imperial(sizes: &[&str]) -> Vec<(String, f64)> {
    sizes.iter().map(|&s| (s.to_string(), inch(s))).collect()
}

fn row(rows: Vec<Vec<RowHole>>, title: &str, shape: HoleShape, stepped: bool) -> HolderSpec {
    let mut spec = RowSpec { rows, shape, stepped, title: Some(title.into()), ..RowSpec::default() };
    spec.cells = row_cells(&spec);
    HolderSpec::Row(spec)
}

/// The socket sets again as peg holders, plus cradles for lying tools.
fn more_sets(socket_sets: &[HolderSet]) -> Vec<HolderSet> {
    let mut out = Vec::new();
    for set in socket_sets {
        let HolderSpec::Row(r) = &set.holder else { continue };
        let Some(drive) = set.drive else { continue };
        let mut spec = RowSpec { pegs: Some(drive), ..r.clone() };
        spec.cells = row_cells(&spec);
        let id: &'static str = Box::leak(set.id.replace("sockets", "pegs").into_boxed_str());
        let label: &'static str = Box::leak(format!("{} auf Steckern", set.label).into_boxed_str());
        out.push(HolderSet { id, label, holder: HolderSpec::Row(spec), drive: Some(drive) });
    }
    let cradles = |slots: Vec<(f64, f64, &str)>, title: &str, stepped: bool| {
        let slots = slots
            .into_iter()
            .map(|(width, length, label)| CradleSlot { width, length, label: label.to_string() })
            .collect();
        let mut spec = CradleSpec { slots, stepped, title: Some(title.into()), ..CradleSpec::default() };
        spec.cells = cradle_cells(&spec);
        HolderSpec::Cradles(spec)
    };
    out.push(HolderSet {
        id: "extensions",
        label: "Verlängerungen 1/4\"–1/2\" (Wiegen)",
        holder: cradles(
            vec![
                (13.0, 100.0, "1/4"),
                (13.0, 150.0, "1/4"),
                (18.0, 125.0, "3/8"),
                (18.0, 250.0, "3/8"),
                (24.0, 250.0, "1/2"),
            ],
            "Verlängerungen",
            false,
        ),
        drive: None,
    });
    out.push(HolderSet {
        id: "ratchets",
        label: "Ratschen (Wiegen, gestuft)",
        holder: cradles(vec![(22.0, 150.0, "1/4"), (30.0, 200.0, "3/8"), (40.0, 250.0, "1/2")], "Ratschen", true),
        drive: None,
    });
    let vslots = |slots: Vec<(f64, f64, &str)>, title: &str, ramp: bool, deep: u32| {
        let slots =
            slots.into_iter().map(|(width, depth, label)| VSlot { width, depth, label: label.into() }).collect();
        let mut spec = VSlotSpec { slots, ramp, title: Some(title.into()), cells: [1, deep], ..VSlotSpec::default() };
        spec.cells = vslot_cells(&spec);
        HolderSpec::VSlots(spec)
    };
    out.push(HolderSet {
        id: "pliers",
        label: "Zangen (V-Schlitze)",
        holder: vslots(
            vec![(14.0, 40.0, ""), (16.0, 40.0, ""), (20.0, 45.0, ""), (24.0, 45.0, "")],
            "Zangen",
            false,
            3,
        ),
        drive: None,
    });
    out.push(HolderSet {
        id: "scissors",
        label: "Scheren und Seitenschneider (V-Schlitze)",
        holder: vslots(vec![(10.0, 35.0, ""), (10.0, 35.0, ""), (12.0, 35.0, "")], "Scheren", false, 3),
        drive: None,
    });
    out.push(HolderSet {
        id: "wrenches",
        label: "Maulschlüssel 6–24 (Rampe)",
        holder: vslots(
            [6.0, 7.0, 8.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 17.0, 19.0, 22.0, 24.0]
                .iter()
                .map(|&k| (k * 0.45 + 3.0, 20.0 + k * 0.6, crate::names::mm(k)))
                .map(|(w, d, l)| (w, d, Box::leak(l.into_boxed_str()) as &str))
                .collect(),
            "Maulschluessel",
            true,
            4,
        ),
        drive: None,
    });
    let tilted = |sizes: &[f64], add: f64, depth: f64, title: &str, shape: HoleShape| {
        let holes =
            sizes.iter().map(|&s| RowHole { d: s + add, depth: Some(depth), label: crate::names::mm(s) }).collect();
        let mut spec =
            RowSpec { rows: vec![holes], shape, tilt: 20.0, title: Some(title.into()), ..RowSpec::default() };
        spec.cells = row_cells(&spec);
        HolderSpec::Row(spec)
    };
    out.push(HolderSet {
        id: "hex_keys_tilted",
        label: "Innensechskant schräg (20°)",
        holder: tilted(&[1.5, 2.0, 2.5, 3.0, 4.0, 5.0, 6.0, 8.0, 10.0], 0.0, 20.0, "Inbus", HoleShape::Hex),
        drive: None,
    });
    out.push(HolderSet {
        id: "pens_tilted",
        label: "Stifte schräg (20°)",
        holder: tilted(&[12.0, 12.0, 12.0, 12.0, 12.0, 12.0], 1.0, 30.0, "Stifte", HoleShape::Round),
        drive: None,
    });
    let rings = |d: f64, height: f64, label: &str| {
        HolderSpec::Rings(RingSpec { d, height, label: Some(label.into()), ..RingSpec::default() })
    };
    out.push(HolderSet {
        id: "spray_cans",
        label: "Spraydosen Ø 66 (Ringe)",
        holder: rings(66.0, 30.0, "Spray"),
        drive: None,
    });
    out.push(HolderSet {
        id: "bottles",
        label: "Flaschen Ø 30 (Ringe)",
        holder: rings(30.0, 25.0, "Flaschen"),
        drive: None,
    });
    out.extend(wall_sets());
    out.push(HolderSet {
        id: "spools",
        label: "Kabelrollen Ø 90 (Ringe)",
        holder: rings(90.0, 20.0, "Kabel"),
        drive: None,
    });
    out
}

pub fn sets() -> Vec<HolderSet> {
    let mut all = base_sets();
    all.extend(more_sets(&all));
    all
}

fn base_sets() -> Vec<HolderSet> {
    let sockets = |drive: Drive, sizes: Vec<(String, f64)>| {
        let refs: Vec<(&str, f64)> = sizes.iter().map(|(l, mm)| (l.as_str(), *mm)).collect();
        socket_row(drive, &refs)
    };
    let plain = |sizes: &[f64], add: f64, depth: f64| -> Vec<RowHole> {
        sizes.iter().map(|&s| RowHole { d: s + add, depth: Some(depth), label: crate::names::mm(s) }).collect()
    };
    vec![
        HolderSet {
            id: "sockets_1_4_mm",
            label: "Nüsse 1/4\" metrisch (4–14)",
            holder: row(
                vec![sockets(
                    Drive::Quarter,
                    metric(&[4.0, 4.5, 5.0, 5.5, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0]),
                )],
                "1/4 Zoll",
                HoleShape::Round,
                false,
            ),
            drive: Some(Drive::Quarter),
        },
        HolderSet {
            id: "sockets_3_8_mm",
            label: "Nüsse 3/8\" metrisch (8–22)",
            holder: row(
                vec![sockets(
                    Drive::ThreeEighths,
                    metric(&[8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0, 20.0, 21.0, 22.0]),
                )],
                "3/8 Zoll",
                HoleShape::Round,
                false,
            ),
            drive: Some(Drive::ThreeEighths),
        },
        HolderSet {
            id: "sockets_1_2_mm",
            label: "Nüsse 1/2\" metrisch (10–32)",
            holder: row(
                vec![sockets(
                    Drive::Half,
                    metric(&[
                        10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0, 20.0, 21.0, 22.0, 24.0, 27.0, 30.0,
                        32.0,
                    ]),
                )],
                "1/2 Zoll",
                HoleShape::Round,
                false,
            ),
            drive: Some(Drive::Half),
        },
        HolderSet {
            id: "sockets_1_4_in",
            label: "Nüsse 1/4\" Zoll (5/32–9/16)",
            holder: row(
                vec![sockets(
                    Drive::Quarter,
                    imperial(&["5/32", "3/16", "7/32", "1/4", "9/32", "5/16", "11/32", "3/8", "7/16", "1/2", "9/16"]),
                )],
                "1/4 inch",
                HoleShape::Round,
                false,
            ),
            drive: Some(Drive::Quarter),
        },
        HolderSet {
            id: "sockets_3_8_in",
            label: "Nüsse 3/8\" Zoll (3/8–7/8)",
            holder: row(
                vec![sockets(
                    Drive::ThreeEighths,
                    imperial(&["3/8", "7/16", "1/2", "9/16", "5/8", "11/16", "3/4", "13/16", "7/8"]),
                )],
                "3/8 inch",
                HoleShape::Round,
                false,
            ),
            drive: Some(Drive::ThreeEighths),
        },
        HolderSet {
            id: "sockets_1_2_in",
            label: "Nüsse 1/2\" Zoll (7/16–1 1/4)",
            holder: row(
                vec![sockets(
                    Drive::Half,
                    imperial(&[
                        "7/16", "1/2", "9/16", "5/8", "11/16", "3/4", "13/16", "7/8", "15/16", "1", "1 1/16", "1 1/8",
                        "1 1/4",
                    ]),
                )],
                "1/2 inch",
                HoleShape::Round,
                false,
            ),
            drive: Some(Drive::Half),
        },
        HolderSet {
            id: "sockets_1_4_both",
            label: "Nüsse 1/4\" metrisch + Zoll (zwei Reihen, gestuft)",
            holder: row(
                vec![
                    sockets(
                        Drive::Quarter,
                        imperial(&[
                            "5/32", "3/16", "7/32", "1/4", "9/32", "5/16", "11/32", "3/8", "7/16", "1/2", "9/16",
                        ]),
                    ),
                    sockets(
                        Drive::Quarter,
                        metric(&[4.0, 4.5, 5.0, 5.5, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0]),
                    ),
                ],
                "1/4",
                HoleShape::Round,
                true,
            ),
            drive: Some(Drive::Quarter),
        },
        HolderSet {
            id: "hex_keys",
            label: "Innensechskant 1.5–10 (stehend)",
            holder: row(
                vec![plain(&[1.5, 2.0, 2.5, 3.0, 4.0, 5.0, 6.0, 8.0, 10.0], 0.0, 20.0)],
                "Inbus",
                HoleShape::Hex,
                false,
            ),
            drive: None,
        },
        HolderSet {
            id: "drills",
            label: "Bohrer 1–10 × 0.5 (zwei Reihen)",
            holder: row(
                vec![
                    plain(&[5.5, 6.0, 6.5, 7.0, 7.5, 8.0, 8.5, 9.0, 9.5, 10.0], 0.4, 18.0),
                    plain(&[1.0, 1.5, 2.0, 2.5, 3.0, 3.5, 4.0, 4.5, 5.0], 0.4, 14.0),
                ],
                "Bohrer",
                HoleShape::Round,
                true,
            ),
            drive: None,
        },
        HolderSet {
            id: "screwdrivers_standing",
            label: "Schraubendreher stehend (Klinge 2–8)",
            holder: row(
                vec![plain(&[2.0, 2.5, 3.0, 4.0, 5.0, 6.0, 8.0], 1.5, 30.0)],
                "Schraubendreher",
                HoleShape::Round,
                false,
            ),
            drive: None,
        },
    ]
}

/// Sets for a wall: extension bars leaning in stepped sockets, screwdrivers hanging through,
/// pens and hex keys, pliers, scissors and wrenches standing in V-slots.
fn wall_sets() -> Vec<HolderSet> {
    let leak = |s: String| -> &'static str { Box::leak(s.into_boxed_str()) };
    let bars = |sizes: &[(f64, &str)], title: &str| {
        let slots = sizes.iter().map(|&(d, l)| RowHole { d, depth: Some(20.0), label: l.into() }).collect();
        HolderSpec::WallCradles(WallCradleSpec { slots, title: Some(title.into()), ..WallCradleSpec::default() }.fit())
    };
    let drives = [(Drive::Quarter, 12.0, "1/4"), (Drive::ThreeEighths, 17.0, "3/8"), (Drive::Half, 22.0, "1/2")];
    let mut out: Vec<HolderSet> = drives
        .iter()
        .map(|&(drive, d, label)| HolderSet {
            id: leak(format!("wall_extensions_{}", label.replace('/', "_"))),
            label: leak(format!("Wand: Verlängerungen {label}\" (Wiegen, gestuft)")),
            holder: bars(&[(d, label); 4], "Verlaengerungen"),
            drive: Some(drive),
        })
        .collect();
    out.push(HolderSet {
        id: "wall_extensions",
        label: "Wand: Verlängerungen 1/4\"–1/2\" (Wiegen, gestuft)",
        holder: bars(
            &[(12.0, "1/4"), (12.0, "1/4"), (17.0, "3/8"), (17.0, "3/8"), (22.0, "1/2"), (22.0, "1/2")],
            "Verlaengerungen",
        ),
        drive: None,
    });
    let hex: Vec<(f64, &str)> = [1.5, 2.0, 2.5, 3.0, 4.0, 5.0, 6.0, 8.0, 10.0]
        .iter()
        .map(|&s: &f64| (s * 2.0 / 3f64.sqrt() + 0.3, leak(crate::names::mm(s))))
        .collect();
    out.push(HolderSet {
        id: "wall_hex_keys",
        label: "Wand: Innensechskant (lehnend)",
        holder: {
            let slots = hex.iter().map(|&(d, l)| RowHole { d, depth: Some(15.0), label: l.into() }).collect();
            HolderSpec::WallCradles(
                WallCradleSpec { slots, stepped: false, title: Some("Inbus".into()), ..WallCradleSpec::default() }
                    .fit(),
            )
        },
        drive: None,
    });
    let holes = |d: f64, depth: f64, n: u32, through: bool, title: &str| {
        HolderSpec::WallHoles(
            WallHolesSpec { d, depth, count: [n, 1], through, title: Some(title.into()), ..WallHolesSpec::default() }
                .fit(),
        )
    };
    out.push(HolderSet {
        id: "wall_screwdrivers",
        label: "Wand: Schraubendreher (hängen durch)",
        holder: holes(8.0, 12.0, 8, true, "Schraubendreher"),
        drive: None,
    });
    out.push(HolderSet {
        id: "wall_pens",
        label: "Wand: Stifte (stehend)",
        holder: holes(12.0, 30.0, 6, false, "Stifte"),
        drive: None,
    });
    let vslots = |slots: Vec<(f64, f64, &'static str)>, title: &str| {
        let slots = slots.into_iter().map(|(width, depth, l)| VSlot { width, depth, label: l.into() }).collect();
        HolderSpec::WallVSlots(WallVSlotSpec { slots, title: Some(title.into()), ..WallVSlotSpec::default() }.fit())
    };
    out.push(HolderSet {
        id: "wall_pliers",
        label: "Wand: Zangen (V-Schlitze)",
        holder: vslots(vec![(14.0, 25.0, ""), (16.0, 25.0, ""), (20.0, 30.0, ""), (24.0, 30.0, "")], "Zangen"),
        drive: None,
    });
    out.push(HolderSet {
        id: "wall_scissors",
        label: "Wand: Scheren (V-Schlitze)",
        holder: vslots(vec![(10.0, 25.0, ""), (10.0, 25.0, ""), (12.0, 25.0, "")], "Scheren"),
        drive: None,
    });
    out.push(HolderSet {
        id: "wall_wrenches",
        label: "Wand: Maulschlüssel 6–24 (V-Schlitze)",
        holder: vslots(
            [6.0, 7.0, 8.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 17.0, 19.0, 22.0, 24.0]
                .iter()
                .map(|&k: &f64| (k * 0.45 + 3.0, 12.0 + k * 0.4, leak(crate::names::mm(k))))
                .collect(),
            "Maulschluessel",
        ),
        drive: None,
    });
    out
}

/// The English name of a catalogue entry or set, by its id; `catalog` and `sets` carry the
/// German one.
pub fn english_label(id: &str) -> Option<String> {
    if let Some(sockets) = id.strip_prefix("pegs_") {
        return english_label(&format!("sockets_{sockets}")).map(|l| format!("{l} on pegs"));
    }
    let name = match id {
        "tape" => "Tape rolls",
        "aa" => "AA cells",
        "aaa" => "AAA cells",
        "c" => "C cells",
        "d" => "D cells",
        "18650" => "18650 cells",
        "21700" => "21700 cells",
        "bits" => "1/4\" bits",
        "pens" => "Pens (standing)",
        "pens_lying" => "Pens (lying)",
        "screwdrivers" => "Screwdrivers (lying)",
        "shank_6" => "Router shanks 6 mm",
        "shank_6_35" => "Router shanks 1/4\"",
        "shank_8" => "Router shanks 8 mm",
        "shank_12" => "Router shanks 12 mm",
        "shank_12_7" => "Router shanks 1/2\"",
        "dremel" => "Dremel shanks 3.2 mm",
        "hex_5_16" => "Hex shanks 5/16\"",
        "hex_3_8" => "Hex shanks 3/8\"",
        "sd" => "SD cards (slots)",
        "usb" => "USB sticks (slots)",
        "cr2032" => "Coin cells (slots)",
        "swatches" => "Filament swatches (slots)",
        "blades" => "Saw blades (slots)",
        "cradles" => "Cradles (lying tools)",
        "vslots" => "V-slots",
        "rings" => "Rings",
        "wall_holes" => "Wall: standing holes",
        "wall_cradles" => "Wall: cradles (bars lean against the wall)",
        "wall_vslots" => "Wall: V-slots (pliers)",
        "row" => "Row of holes (your own sizes)",
        "extensions" => "Extensions 1/4\"–1/2\" (cradles)",
        "ratchets" => "Ratchets (cradles, stepped)",
        "pliers" => "Pliers (V-slots)",
        "scissors" => "Scissors and side cutters (V-slots)",
        "wrenches" => "Wrenches 6–24 (ramp)",
        "hex_keys_tilted" => "Hex keys tilted (20°)",
        "pens_tilted" => "Pens tilted (20°)",
        "spray_cans" => "Spray cans Ø 66 (rings)",
        "bottles" => "Bottles Ø 30 (rings)",
        "spools" => "Cable reels Ø 90 (rings)",
        "sockets_1_4_mm" => "Sockets 1/4\" metric (4–14)",
        "sockets_3_8_mm" => "Sockets 3/8\" metric (8–22)",
        "sockets_1_2_mm" => "Sockets 1/2\" metric (10–32)",
        "sockets_1_4_in" => "Sockets 1/4\" inch (5/32–9/16)",
        "sockets_3_8_in" => "Sockets 3/8\" inch (3/8–7/8)",
        "sockets_1_2_in" => "Sockets 1/2\" inch (7/16–1 1/4)",
        "sockets_1_4_both" => "Sockets 1/4\" metric + inch (two rows, stepped)",
        "hex_keys" => "Hex keys 1.5–10 (standing)",
        "drills" => "Drills 1–10 × 0.5 (two rows)",
        "screwdrivers_standing" => "Screwdrivers standing (blade 2–8)",
        "wall_extensions_1_4" => "Wall: extensions 1/4\" (cradles, stepped)",
        "wall_extensions_3_8" => "Wall: extensions 3/8\" (cradles, stepped)",
        "wall_extensions_1_2" => "Wall: extensions 1/2\" (cradles, stepped)",
        "wall_extensions" => "Wall: extensions 1/4\"–1/2\" (cradles, stepped)",
        "wall_hex_keys" => "Wall: hex keys (leaning)",
        "wall_screwdrivers" => "Wall: screwdrivers (hanging through)",
        "wall_pens" => "Wall: pens (standing)",
        "wall_pliers" => "Wall: pliers (V-slots)",
        "wall_scissors" => "Wall: scissors (V-slots)",
        "wall_wrenches" => "Wall: wrenches 6–24 (V-slots)",
        _ => return None,
    };
    Some(name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_holder_and_set_has_an_english_name() {
        let ids = catalog().into_iter().map(|c| c.id).chain(sets().into_iter().map(|s| s.id));
        for id in ids {
            let name = english_label(id).unwrap_or_else(|| panic!("no English name for {id}"));
            assert!(name.is_ascii() || !name.contains(['ä', 'ö', 'ü', 'ß']), "{id}: {name}");
        }
    }

    #[test]
    fn capacities_and_sizes() {
        // 39.1 mm inside a 1x1 bin minus margins: two AA holes (pitch 16.7) fit each way.
        let aa = HolderSpec::Holes(HolesSpec { item: RoundItem::Aa, ..HolesSpec::default() });
        assert_eq!(capacity(&aa), 4);
        assert_eq!(cells_for(&aa, 4), [1, 1]);
        // 1x2 holds 2 x 4 = 8, so twelve need three cells (2 x 6).
        let twelve = cells_for(&aa, 12);
        assert!(capacity(&aa.clone().with_cells(twelve)) >= 12 && twelve[0] * twelve[1] == 3, "{twelve:?}");
        let bits = HolderSpec::Bits(Default::default());
        assert_eq!(capacity(&bits), 16, "4 x 4 bits at 9.1 mm pitch in a 1x1 bin");
        let sd = catalog().into_iter().find(|c| c.id == "screwdrivers").unwrap().holder;
        assert_eq!(capacity(&sd), 1);
        assert_eq!(cells_for(&sd, 3)[0], 4, "troughs keep their length");
        assert!(sd.units() >= 5);
        assert_eq!(aa.units(), 6, "half an AA cell shows above the holder");
    }

    #[test]
    fn overrides_and_new_kinds() {
        let fixed = HolesSpec { count: Some([3, 2]), pitch: Some(20.0), ..HolesSpec::default() };
        assert_eq!(holes_grid(&fixed), [3, 2]);
        assert_eq!(hole_pitch(&fixed), 20.0);
        // Hex holes need their across-corners width.
        let hex =
            HolesSpec { shape: HoleShape::Hex, item: RoundItem::Custom { d: 8.0, h: 20.0 }, ..Default::default() };
        assert!(hole_pitch(&hex) > 8.0 + 0.6 + HOLE_WALL);
        // Swatch slots at 6 mm pitch: a 2-cell bin is 81.5 mm wide, 75.1 inside the margins.
        let swatches = catalog().into_iter().find(|c| c.id == "swatches").unwrap().holder;
        assert_eq!(capacity(&swatches), 13);
        assert!(capacity(&swatches.clone().with_cells(cells_for(&swatches, 20))) >= 20);
        // A row holder's footprint follows its holes.
        let set = sets().into_iter().find(|s| s.id == "sockets_1_4_mm").unwrap().holder;
        let HolderSpec::Row(r) = &set else { panic!() };
        assert_eq!(r.hole_count(), 13);
        assert_eq!(r.cells[1], 1, "one row with labels fits a single cell deep");
        assert!(r.cells[0] >= 5, "thirteen sockets need a long row: {:?}", r.cells);
        assert_eq!(cells_for(&set, 1), r.cells);
        assert_eq!(socket_od(Drive::Quarter, 13.0), 18.0);
        assert_eq!(socket_od(Drive::Half, 10.0), 23.5);
        assert!((inch("1 1/16") - 26.9875).abs() < 1e-9);
        let two = sets().into_iter().find(|s| s.id == "sockets_1_4_both").unwrap().holder;
        assert!(two.units() > set.units(), "the stepped second row adds a unit");
    }
}

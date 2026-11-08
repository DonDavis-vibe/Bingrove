//! Gridfinity dimensions, as in gridfinity-rebuilt-openscad (`src/core/standard.scad`) and the
//! original `gridfinity_schublade.py`. All lengths in mm.
//!
//! Profiles are `(z, inset)` pairs: the height and how far the outline is pulled in from the
//! nominal rounded rectangle at that height. Consecutive pairs are hulled into one ring.

/// Grid pitch.
pub const GRID: f64 = 42.0;
/// One height unit.
pub const UNIT: f64 = 7.0;

/// Clearance between the plate and the drawer wall, in total per axis (plate = drawer - this).
pub const CLEARANCE: f64 = 1.0;
/// Corner radius of the plate at the drawer corners.
pub const CORNER_R: f64 = 4.0;
/// Solid margins at least this wide are hollowed out from below to save filament.
pub const POCKET_MIN: f64 = 8.0;
/// Ceiling above those pockets.
pub const POCKET_TOP: f64 = 1.2;
/// Wall towards the drawer and the grid, and the ribs on the grid lines between pockets.
pub const POCKET_WALL: f64 = 1.6;

/// Baseplate height (profile only, open bottom).
pub const BP_H: f64 = 4.65;
/// Radius of a cell opening at the top of the baseplate.
pub const BP_R: f64 = 4.0;
/// Cell cut-out of the baseplate; it overshoots at the top and bottom so the cut goes clean through.
pub const BP_PROFILE: [(f64, f64); 6] =
    [(BP_H + 1.0, -1.0), (BP_H, 0.0), (2.5, 2.15), (0.7, 2.15), (0.0, 2.85), (-1.0, 2.85)];

/// Outer size of a bin per cell (0.25 mm air on each side).
pub const BIN_SIZE: f64 = 41.5;
/// Outer corner radius of a bin.
pub const BIN_R: f64 = 3.75;
/// Height of the foot profile.
pub const FOOT_H: f64 = 4.75;
/// Foot of one cell, from the build plate up.
pub const FOOT_PROFILE: [(f64, f64); 4] = [(0.0, 2.95), (0.8, 2.15), (2.6, 2.15), (FOOT_H, 0.0)];
/// Wall thickness (three lines with a 0.4 mm nozzle).
pub const WALL: f64 = 1.2;
/// Floor above the feet.
pub const FLOOR: f64 = 1.2;
/// Height of the stacking lip above the body.
pub const LIP_H: f64 = 4.4;
/// Dividers stop this far below the top of the body so bins still stack.
pub const DIVIDER_GAP: f64 = 0.8;

/// Interior floor of a bin (top of the feet plus the floor).
pub const FLOOR_Z: f64 = FOOT_H + FLOOR;

/// Room left above the tallest bins so they can be lifted out.
pub const HEADROOM: f64 = 5.0;
/// Air between an edge bin's outer face and the outline of the plate.
pub const EDGE_GAP: f64 = 0.5;
/// Air between the skirt of a ToFloor edge bin and the rim of the plate.
pub const SKIRT_GAP: f64 = 0.5;
/// Air between neighbouring bins, per side (41.5 mm bins in a 42 mm grid).
pub const BIN_GAP: f64 = (GRID - BIN_SIZE) / 2.0;
/// Depth of the label shelf from the back wall, and the angle of its support.
pub const LABEL_DEPTH: f64 = 13.0;
pub const LABEL_ANGLE: f64 = 36.0;

/// Inside diameter of a 3" tape core (parcel tape, gaffer, masking tape ...).
pub const TAPE_CORE: f64 = 76.0;
/// Air on each side between post and core.
pub const TAPE_POST_GAP: f64 = 1.5;
/// Roll width, i.e. the height each roll adds to a stack.
pub const TAPE_W: f64 = 50.0;
/// How far the post reaches into the top roll.
pub const TAPE_POST_H: f64 = 12.0;
/// Wall of the ring-shaped post.
pub const TAPE_POST_WALL: f64 = 3.0;
/// Lead-in chamfer at the top of the post.
pub const TAPE_CHAMFER: f64 = 2.0;
/// 45° reinforcement inside the foot of the post.
pub const TAPE_FILLET: f64 = 5.0;
/// Air between rolls and to the bin edge.
pub const TAPE_ROLL_GAP: f64 = 1.0;

/// A lid's plug reaching down into the bin's lip, the air around it and the click bumps.
pub const LID_PLUG: f64 = 3.5;
pub const LID_GAP: f64 = 0.2;
pub const LID_CLICK: f64 = 0.3;

/// Socket pegs: the base plate they stand on above the bin floor, and their height.
pub const PEG_BASE: f64 = 2.0;
pub const PEG_H: f64 = 8.0;

/// Screw hole through a plate floor (M4 wood screw) and its countersink at the top.
pub const SCREW_D: f64 = 4.2;
pub const SCREW_HEAD: f64 = 8.5;
/// Dovetail connector between plate pieces: neck at the cut line, head at its end, how far it
/// reaches into the neighbour, air around the tab in its pocket, and how far it stays off the
/// floor's top and bottom.
pub const CONNECTOR_NECK: f64 = 8.0;
pub const CONNECTOR_HEAD: f64 = 11.0;
pub const CONNECTOR_DEPTH: f64 = 4.0;
pub const CONNECTOR_GAP: f64 = 0.15;
pub const CONNECTOR_INSET: f64 = 0.4;
/// Floor kept around a connector's pocket where a skeleton floor has its cross cut-out.
pub const CONNECTOR_WALL: f64 = 1.2;

/// Wall plates: the two hooks standing on every cell's pocket floor, in the plate's frame (x
/// across the cell from its centre, y up the wall from the cell's lower grid line, z out of the
/// wall from the pocket floor). One sits low and one high in the cell, so a part can tip out
/// neither at the top nor at the bottom. Each stem runs from its `HOOK_YS` to `+ HOOK_STEM`, the
/// finger on top reaches up to `+ HOOK_REACH` and is `HOOK_FINGER` thick; `HOOK_H` tall in all.
/// Under the finger a 45° gusset `HOOK_GUSSET` long joins it to the stem, so the plate prints the
/// finger with only a short free overhang.
pub const HOOK_W: f64 = 10.0;
pub const HOOK_YS: [f64; 2] = [10.0, 27.5];
pub const HOOK_STEM: f64 = 2.5;
pub const HOOK_REACH: f64 = 5.0;
pub const HOOK_FINGER: f64 = 1.4;
pub const HOOK_H: f64 = 3.4;
pub const HOOK_GUSSET: f64 = 1.0;
/// How far a wall part is lifted to go in past the hooks; it is that much shorter at the top.
pub const HOOK_LIFT: f64 = 4.0;
/// How far the keyhole runs past the lifted position, so a part pushed up to its stop still
/// goes in.
pub const HOOK_OVERTRAVEL: f64 = 1.0;

/// Wall plates: the spring blade of the latch, in the same frame as the hooks, `DETENT_X` across
/// the cell beside them. `DETENT_T` thick, it stands from the bed through a slot in the floor
/// to `DETENT_H` above the pocket floor. Its root, `DETENT_ROOT` long below `DETENT_Y0`, is
/// joined to the floor; from there it is free for `DETENT_LEN` up the wall, where a nose sticks
/// out `DETENT_NOSE` towards +x. The plate prints it standing, so it bends along its layers.
pub const DETENT_X: f64 = -8.0;
pub const DETENT_T: f64 = 1.2;
pub const DETENT_H: f64 = 3.0;
pub const DETENT_Y0: f64 = 12.0;
pub const DETENT_ROOT: f64 = 2.0;
pub const DETENT_LEN: f64 = 10.0;
pub const DETENT_NOSE: f64 = 0.8;
/// Flat between the flanks of the nose and of the rib that catches it.
pub const DETENT_FLAT: f64 = 0.2;
/// Room beside the blade in a wall part's foot, and in the plate's floor, for it to bend away.
pub const DETENT_ROOM: f64 = 1.0;
/// Air around the free blade in the slot through the plate's floor, apart from `DETENT_ROOM`.
pub const DETENT_RELIEF: f64 = 0.4;

/// A wall hopper's front, as a share of its height; its sides slope up from there to full
/// height at `HOPPER_SLOPE` of the way back to the wall.
pub const HOPPER_FRONT: f64 = 0.4;
pub const HOPPER_SLOPE: f64 = 0.6;

/// Outer size of a bin covering `cells` grid cells along one axis.
pub fn bin_outer(cells: u32) -> f64 {
    cells as f64 * GRID - (GRID - BIN_SIZE)
}

/// Height of a bin body (without lip) for `units` height units.
pub fn body_height(units: u32) -> f64 {
    units as f64 * UNIT
}

/// Outer size of a wall part as printed, lying on its bottom face: width, depth out of the wall
/// (its height in units), height up the wall (`HOOK_LIFT` short of the cells).
pub fn wall_size(cells: [u32; 2], units: u32) -> [f64; 3] {
    [bin_outer(cells[0]), body_height(units), bin_outer(cells[1]) - HOOK_LIFT]
}

/// IKEA Skadis boards: slots 5 mm wide and 15 mm tall, 40 mm apart in a row, the board 5 mm
/// thick. A hook for a wall plate: a dovetail tab that slides into a pocket in the plate's top
/// edge (from behind), a strip `SKADIS_STEM` long above the edge and `SKADIS_STRIP` thick, a bar
/// `SKADIS_W` wide and `SKADIS_BAR` tall through the slot, and a leg `SKADIS_LEG` down behind
/// the board, `SKADIS_T` thick.
pub const SKADIS_PITCH: f64 = 40.0;
pub const SKADIS_BOARD: f64 = 5.0;
pub const SKADIS_W: f64 = 4.6;
pub const SKADIS_STEM: f64 = 8.0;
pub const SKADIS_STRIP: f64 = 5.0;
pub const SKADIS_BAR: f64 = 4.0;
pub const SKADIS_LEG: f64 = 10.0;
pub const SKADIS_T: f64 = 3.0;
/// Air behind the board for the leg.
pub const SKADIS_AIR: f64 = 0.6;

/// Where the Skadis hooks of a plate piece `width` wide go, from its middle: a pair as far
/// apart as whole slot pitches allow with 15 mm to the ends, or one in the middle.
pub fn skadis_offsets(width: f64) -> Vec<f64> {
    let k = ((width - 30.0) / SKADIS_PITCH).floor();
    if k < 1.0 { vec![0.0] } else { vec![-k * SKADIS_PITCH / 2.0, k * SKADIS_PITCH / 2.0] }
}

/// A Skadis hook's size as printed, lying on its side.
pub fn skadis_hook_size() -> [f64; 3] {
    let low = (-CONNECTOR_DEPTH).min(SKADIS_STEM - SKADIS_BAR - SKADIS_LEG);
    [SKADIS_STRIP + SKADIS_BOARD + SKADIS_AIR + SKADIS_T, SKADIS_STEM - low, CONNECTOR_HEAD]
}

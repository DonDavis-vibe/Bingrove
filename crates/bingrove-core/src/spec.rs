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

/// Outer size of a bin covering `cells` grid cells along one axis.
pub fn bin_outer(cells: u32) -> f64 {
    cells as f64 * GRID - (GRID - BIN_SIZE)
}

/// Height of a bin body (without lip) for `units` height units.
pub fn body_height(units: u32) -> f64 {
    units as f64 * UNIT
}

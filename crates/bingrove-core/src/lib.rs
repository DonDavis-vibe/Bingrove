//! Bingrove: Gridfinity drawer inserts that use the whole drawer.
//!
//! The planning part (types, presets, layout checks, names) builds everywhere; meshes and file
//! export need the `geometry` feature.

pub mod catalog;
pub mod grid;
pub mod layout;
pub mod model;
pub mod names;
pub mod plan;
pub mod presets;
pub mod profile;
pub mod silhouette;
pub mod solver;
pub mod spec;
pub mod split;
pub mod text;
pub mod warn;

#[cfg(feature = "geometry")]
pub mod export;
#[cfg(feature = "geometry")]
pub mod geom;
#[cfg(feature = "geometry")]
pub mod mesh;

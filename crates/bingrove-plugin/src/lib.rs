//! Bingrove as a Meshgrove plugin.
//!
//! - Tools for the AI: `options` (what there is), `plan` (a drawer planned, no geometry, fast)
//!   and `generate` (everything written and handed over as a bundle with print lists).
//! - Panel "drawer": the drawer's form, the plan as a grid, the parts list and a 3D preview;
//!   "Generieren + Druckliste" runs the export as a job and puts the files into the library.
//! - Opens `*.gridfinity.json` project files in the panel.

mod panel;
mod preview;

use bingrove_core::export::{self, ExportOptions, FileKind, PROJECT_FORMAT, ProjectFile};
use bingrove_core::model::{Align, DrawerProject, DrawerSpec, EdgeMode, PlateFloor, PlateOptions};
use bingrove_core::plan::{Plan, plan};
use bingrove_core::presets::Preset;
use bingrove_core::profile::Profile;
use meshgrove_plugin_api::guest::{self, Guest, PluginError, ToolOutput};
use meshgrove_plugin_api::ui::Effect;
use meshgrove_plugin_api::{
    API_VERSION, Bundle, BundleFile, BundleItem, BundleList, FileType, ListKind, Manifest, PanelDef, Permission,
    ToolDef,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub(crate) struct Bingrove;

/// What goes on the print lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ListMode {
    /// One list "<name> – Teile": every STL with its quantity.
    Parts,
    /// One list "<name> – Druckbetten": the 3MF of every bed.
    Beds,
    /// Both lists.
    #[default]
    Both,
}

/// The arguments of `plan` and `generate`: a project, a few fields flattened for the AI.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct DrawerArgs {
    pub name: String,
    pub width: f64,
    pub depth: f64,
    pub height: f64,
    /// Fill a known drawer instead of giving sizes: "250x305" or "435x330".
    pub preset: Option<String>,
    pub items: Vec<bingrove_core::model::ItemRequest>,
    pub placements: Vec<bingrove_core::model::Placement>,
    pub edge_threshold: Option<f64>,
    pub edge_mode: Option<EdgeMode>,
    pub align: Option<[Align; 2]>,
    pub magnets: bool,
    pub printer: Option<String>,
    pub lists: ListMode,
}

impl DrawerArgs {
    pub(crate) fn project(&self) -> Result<DrawerProject, PluginError> {
        let mut project = DrawerProject::default();
        if let Some(preset) = &self.preset {
            let p = Preset::get(preset)
                .ok_or_else(|| PluginError::Invalid(format!("no preset {preset:?}; options lists them")))?;
            project.drawer.width = p.plate.drawer[0];
            project.drawer.depth = p.plate.drawer[1];
            project.drawer.name = p.name.clone();
            project.placements = p.placements.clone();
        }
        if self.width > 0.0 {
            project.drawer.width = self.width;
        }
        if self.depth > 0.0 {
            project.drawer.depth = self.depth;
        }
        project.drawer.height = self.height;
        if !self.name.trim().is_empty() {
            project.drawer.name = self.name.trim().to_string();
        }
        if !(42.0..=1500.0).contains(&project.drawer.width) || !(42.0..=1500.0).contains(&project.drawer.depth) {
            return Err(PluginError::Invalid("width and depth must be between 42 and 1500 mm".into()));
        }
        if self.items.len() > 100 || self.placements.len() > 300 {
            return Err(PluginError::Invalid("at most 100 items and 300 placements".into()));
        }
        if !self.placements.is_empty() {
            project.placements = self.placements.clone();
        }
        project.items = self.items.clone();
        project.options = PlateOptions {
            edge_threshold: self.edge_threshold.unwrap_or(10.0),
            edge_mode: self.edge_mode.unwrap_or_default(),
            align: self.align.unwrap_or_default(),
            floor: if self.magnets { PlateFloor::Magnets } else { PlateFloor::Open },
            ..PlateOptions::default()
        };
        project.bed = bed_of(self.printer.as_deref());
        Ok(project)
    }
}

/// The printer bed, from Meshgrove's printers: the named one, else the main one, else the default.
pub(crate) fn bed_of(printer: Option<&str>) -> bingrove_core::model::BedSpec {
    let info = guest::info();
    let chosen = match printer {
        Some(name) => info.printers.iter().find(|p| p.name.eq_ignore_ascii_case(name)),
        None => info.printers.first(),
    };
    match chosen {
        Some(p) => bingrove_core::model::BedSpec {
            name: p.name.clone(),
            x: f64::from(p.bed[0]),
            y: f64::from(p.bed[1]),
            z: f64::from(p.bed[2]),
            ..Default::default()
        },
        None => Default::default(),
    }
}

/// What the AI gets back from `plan`.
pub(crate) fn plan_result(plan: &Plan) -> Value {
    json!({
        "summary": format!(
            "{}: {} x {} cells, {} parts ({} pieces), {:.0}% of the plate usable",
            plan.name, plan.grid.cells[0], plan.grid.cells[1], plan.parts.len(), plan.part_count(), plan.usable_area()
        ),
        "text": bingrove_core::text::ascii_plan(plan),
        "grid": plan.grid,
        "split": plan.split,
        "heights": plan.heights,
        "placements": plan.placements.iter().map(|p| &p.placement).collect::<Vec<_>>(),
        "parts": plan.parts,
        "warnings": plan.warnings,
        "repairs": plan.repairs,
    })
}

/// Runs the export below `/out` and describes it as a bundle.
pub(crate) fn generate(project: &DrawerProject, lists: ListMode) -> Result<(Plan, Bundle, Value), PluginError> {
    let plan = plan(project).map_err(|e| PluginError::Invalid(e.to_string()))?;
    let opts = ExportOptions { assembly: false, ..ExportOptions::default() };
    let report = export::export(&plan, project, std::path::Path::new("/out"), opts, &|done, total, what| {
        guest::host::progress(done as f32 / total.max(1) as f32, what);
    })
    .map_err(|e| PluginError::Failed(e.to_string()))?;
    let bundle = bundle(&plan, &report, lists);
    let result = json!({
        "summary": bundle.summary,
        "files": report.files,
        "beds": report.beds,
        "warnings": plan.warnings,
        "text": bingrove_core::text::ascii_plan(&plan),
    });
    Ok((plan, bundle, result))
}

pub(crate) fn bundle(plan: &Plan, report: &export::ExportReport, lists: ListMode) -> Bundle {
    let name = &plan.name;
    let mut out = Bundle {
        title: format!("Gridfinity {name}"),
        summary: Some(format!(
            "{} parts ({} pieces) and {} print beds for a {:.0} x {:.0} mm drawer",
            plan.parts.len(),
            plan.part_count(),
            report.beds,
            plan.grid.plate[0] + 1.0,
            plan.grid.plate[1] + 1.0
        )),
        folder: Some(format!("Gridfinity/{name}")),
        tags: vec!["Gridfinity".into(), "Bingrove".into()],
        ..Bundle::default()
    };
    for f in &report.files {
        let note = match f.kind {
            FileKind::Part => plan
                .parts
                .iter()
                .find(|p| p.name == f.path)
                .map(|p| format!("{} x, {:.1} x {:.1} x {:.1} mm", p.quantity, p.size[0], p.size[1], p.size[2])),
            FileKind::Bed => Some("Druckbett, Teile fertig angeordnet".into()),
            FileKind::Assembly => Some("Zusammenbau zum Anschauen".into()),
            FileKind::Project => Some("Bingrove-Projekt, in Meshgrove mit Bingrove öffnen".into()),
        };
        let tags = match f.kind {
            FileKind::Bed => vec!["Druckbett".into()],
            _ => vec![],
        };
        out.files.push(BundleFile { path: f.path.clone(), tags, note });
    }
    let items = |kind: FileKind| -> Vec<BundleItem> {
        report
            .files
            .iter()
            .filter(|f| f.kind == kind)
            .map(|f| BundleItem { path: f.path.clone(), quantity: f.quantity.max(1) })
            .collect()
    };
    if lists != ListMode::Beds {
        out.lists.push(BundleList {
            name: format!("{name} – Teile"),
            kind: ListKind::Print,
            items: items(FileKind::Part),
        });
    }
    if lists != ListMode::Parts && report.beds > 0 {
        out.lists.push(BundleList {
            name: format!("{name} – Druckbetten"),
            kind: ListKind::Print,
            items: items(FileKind::Bed),
        });
    }
    out
}

fn drawer_schema(with_lists: bool) -> Value {
    let mut props = json!({
        "name": {"type": "string", "description": "name of the drawer, used for folder and list names"},
        "width": {"type": "number", "description": "inner width of the drawer in mm (x, left to right)"},
        "depth": {"type": "number", "description": "inner depth in mm (y, front to back)"},
        "height": {"type": "number", "description": "inner height in mm; 0 if unknown (bins then get 6 units)"},
        "preset": {"type": "string", "description": "a known drawer instead of sizes: see options"},
        "items": {"type": "array", "description": "what should go in; the solver places them (tall at the back) and fills the rest with bins", "items": {
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "quantity": {"type": "integer", "default": 1},
                "kind": {"type": "string", "enum": ["object", "bin", "tape", "holder"], "description": "object: size [x, y, z] mm of a thing to hold; bin: cells [w, d] and optional units (7 mm each); tape: od (roll diameter), stacks [nx, ny], height (rolls per stack); holder: a holder from options.holders (holes for batteries/pens, bits, troughs for lying things) with count"},
                "holder": {"type": "object", "description": "for kind holder: {kind: holes, item: {item: aa|aaa|c|d|cell18650|cell21700|custom, d, h}} or {kind: bits} or {kind: troughs, diameter, length, handle} or {kind: tape, od, stacks, height}"},
                "count": {"type": "integer", "description": "for kind holder: how many things it must hold; the footprint follows"},
                "size": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3},
                "clearance": {"type": "number", "default": 2},
                "cells": {"type": "array", "items": {"type": "integer"}, "minItems": 2, "maxItems": 2},
                "units": {"type": "integer"},
                "dividers": {"type": "array", "items": {"type": "integer"}, "minItems": 2, "maxItems": 2},
                "od": {"type": "number"}, "stacks": {"type": "array", "items": {"type": "integer"}}, "height": {"type": "integer"}
            },
            "required": ["kind"]
        }},
        "placements": {"type": "array", "description": "an exact layout instead of items: bins by cell (x from left, y from front, cell 0 0 front left) with cells [w, d], units, dividers [dx, dy]; anything that overlaps is moved, gaps are filled", "items": {
            "type": "object",
            "properties": {
                "x": {"type": "integer"}, "y": {"type": "integer"},
                "cells": {"type": "array", "items": {"type": "integer"}, "minItems": 2, "maxItems": 2},
                "units": {"type": "integer"},
                "dividers": {"type": "array", "items": {"type": "integer"}, "minItems": 2, "maxItems": 2},
                "label_tab": {"type": "boolean"}, "scoop": {"type": "integer", "description": "finger scoop in percent, 0 none"},
                "label": {"type": "string", "description": "what goes in"},
                "cutouts": {"type": "array", "description": "pockets for particular things, cut into a solid insert; see options.cutouts", "items": {"type": "object"}}
            },
            "required": ["x", "y", "cells", "units"]
        }},
        "edge_threshold": {"type": "number", "description": "margins at least this wide (mm) become edge zones that bins reach into; default 10"},
        "edge_mode": {"type": "string", "enum": ["to_floor", "resting", "none"], "description": "to_floor (default): edge bins reach the drawer floor, no supports; resting: they rest on the solid plate margin (needs supports); none: no edge bins at all, the margins stay solid plate and every bin is 42 n"},
        "align": {"type": "array", "items": {"type": "string", "enum": ["start", "center", "end"]}, "minItems": 2, "maxItems": 2, "description": "where the grid sits along x and y when the leftover is split: center (default), start (left/front wall), end"},
        "magnets": {"type": "boolean", "description": "plate with a floor and magnet holes under every cell corner"},
        "printer": {"type": "string", "description": "one of the user's printers (its bed splits the plate); default the main one"}
    });
    if with_lists {
        props["lists"] = json!({"type": "string", "enum": ["parts", "beds", "both"], "description": "print lists to make: parts (STL with quantities), beds (3MF per bed) or both (default)"});
    }
    json!({"type": "object", "properties": props})
}

impl Guest for Bingrove {
    type Panel = panel::DrawerPanel;

    fn manifest() -> String {
        guest::json(&Manifest {
            id: "bingrove".into(),
            name: "Bingrove".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            api: API_VERSION.into(),
            description: "Gridfinity drawer inserts that use the whole drawer: baseplate, bins and edge bins planned from the drawer's size, generated as STL and 3MF, put on print lists.".into(),
            authors: vec!["Davis (German Redneck Engineering)".into()],
            homepage: Some("https://github.com/DonDavis-vibe/Bingrove".into()),
            permissions: vec![Permission::AiTools, Permission::Bundles, Permission::LibraryRead],
            panels: vec![PanelDef { id: "drawer".into(), title: "Bingrove".into() }],
            file_types: vec![FileType { pattern: "*.gridfinity.json".into(), description: "A Bingrove drawer".into() }],
            memory_mb: Some(2048),
            ..Manifest::default()
        })
    }

    fn list_tools() -> String {
        guest::json(&[
            ToolDef {
                name: "options".into(),
                description: "What Bingrove knows: drawer presets, the user's printers, profiles, defaults and the item kinds plan and generate accept. Call it first.".into(),
                input_schema: json!({"type": "object", "properties": {}}),
                bundles: false,
            },
            ToolDef {
                name: "plan".into(),
                description: "Plans a Gridfinity insert for a drawer without making files: grid and margins, which margins become edge zones, the plate pieces for the printer, the bins (placed from items or checked from placements), the parts list and warnings, plus a text plan to show the user. Fast; iterate here, then call generate.".into(),
                input_schema: drawer_schema(false),
                bundles: false,
            },
            ToolDef {
                name: "generate".into(),
                description: "Makes the planned insert: one STL per part, a 3MF per print bed and a project file, and hands them to Meshgrove as a bundle (folder Gridfinity/<name>, print lists). Takes the same arguments as plan.".into(),
                input_schema: drawer_schema(true),
                bundles: true,
            },
        ])
    }

    fn call_tool(name: String, args: String) -> Result<ToolOutput, PluginError> {
        match name.as_str() {
            "options" => {
                let info = guest::info();
                let presets: Vec<Value> = Preset::all()
                    .iter()
                    .map(|p| json!({"name": p.name, "description": p.description, "drawer": p.plate.drawer, "cells": p.plate.cells()}))
                    .collect();
                Ok(guest::tool_output(
                    &json!({
                        "presets": presets,
                        "printers": info.printers,
                        "profiles": Profile::builtin().iter().map(|p| &p.name).collect::<Vec<_>>(),
                        "defaults": PlateOptions::default(),
                        "grid_mm": bingrove_core::spec::GRID,
                        "unit_mm": bingrove_core::spec::UNIT,
                        "item_kinds": ["object: size [x, y, z] mm", "bin: cells [w, d], units", "tape: od, stacks [nx, ny], height", "holder: holder {kind: holes|bits|troughs|tape, ...} and count"],
                        "holders": bingrove_core::catalog::catalog(),
                        "cutouts": ["box: at [x, y], size [w, d], depth", "cylinder: at [x, y], d, depth", "polygon: points [[x, y], ...], clearance, depth (mm from the bin's front-left corner)"],
                        "limits": {"drawer_mm": [42, 1500], "items": 100, "placements": 300},
                    }),
                    None,
                ))
            }
            "plan" => {
                let args: DrawerArgs = guest::parse(&args)?;
                let project = args.project()?;
                let plan = plan(&project).map_err(|e| PluginError::Invalid(e.to_string()))?;
                Ok(guest::tool_output(&plan_result(&plan), None))
            }
            "generate" => {
                let args: DrawerArgs = guest::parse(&args)?;
                let project = args.project()?;
                let (_, bundle, result) = generate(&project, args.lists)?;
                Ok(guest::tool_output(&result, Some(&bundle)))
            }
            _ => Err(PluginError::Invalid(format!("no tool {name:?}"))),
        }
    }

    fn run_job(kind: String, params: String) -> Result<String, PluginError> {
        match kind.as_str() {
            "generate" => {
                #[derive(Deserialize)]
                struct Params {
                    project: DrawerProject,
                    #[serde(default)]
                    lists: ListMode,
                }
                let p: Params = guest::parse(&params)?;
                let (_, bundle, result) = generate(&p.project, p.lists)?;
                Ok(guest::json(&json!({"bundle": bundle, "result": result})))
            }
            "calibration" => {
                #[derive(Deserialize)]
                struct Params {
                    #[serde(default)]
                    profile: Profile,
                    #[serde(default)]
                    name: String,
                }
                let p: Params = guest::parse(&params)?;
                let (bundle, result) = calibration(&p.profile, &p.name)?;
                Ok(guest::json(&json!({"bundle": bundle, "result": result})))
            }
            "preview" => {
                #[derive(Deserialize)]
                struct Params {
                    project: DrawerProject,
                    key: String,
                }
                let p: Params = guest::parse(&params)?;
                let parts = preview::compute(&p.project, &p.key)?;
                Ok(guest::json(&json!({"key": p.key, "parts": parts})))
            }
            _ => Err(PluginError::Invalid(format!("no job {kind:?}"))),
        }
    }

    fn open_file(name: String, bytes: Vec<u8>) -> Result<String, PluginError> {
        let file: ProjectFile = serde_json::from_slice(&bytes)?;
        if file.format != PROJECT_FORMAT {
            return Err(PluginError::Invalid(format!("{name} is not a Bingrove project")));
        }
        let state = panel::PanelState::from_project(file.project);
        Ok(guest::effects(&[Effect::OpenPanel { panel: "drawer".into(), state: Some(guest::json(&state)) }]))
    }
}

/// Writes the calibration pieces below `/out/Kalibrierung` and describes them as a bundle.
pub(crate) fn calibration(profile: &Profile, name: &str) -> Result<(Bundle, Value), PluginError> {
    let dir = std::path::Path::new("/out");
    let parts = bingrove_core::geom::calibration::parts(profile);
    let mut files = Vec::new();
    for (i, part) in parts.iter().enumerate() {
        guest::host::progress(i as f32 / parts.len() as f32, &part.name);
        let mesh = part.mesh().map_err(|e| PluginError::Failed(e.to_string()))?;
        export::save_stl(dir, &part.name, &mesh).map_err(|e| PluginError::Failed(e.to_string()))?;
        files.push(part.name.clone());
    }
    let list =
        if name.trim().is_empty() { "Bingrove Kalibrierung".to_string() } else { format!("{name} – Kalibrierung") };
    let bundle = Bundle {
        title: "Bingrove Kalibrierung".into(),
        summary: Some(format!(
            "Magnetloch-Leiter {:.1}–{:.1} mm und Passungstest mit {} Bins",
            profile.magnet.d - 0.3,
            profile.magnet.d + 0.3,
            bingrove_core::geom::calibration::FIT_GAPS.len()
        )),
        folder: Some("Gridfinity/Kalibrierung".into()),
        files: files.iter().map(|f| BundleFile { path: f.clone(), tags: vec![], note: None }).collect(),
        tags: vec!["Gridfinity".into(), "Bingrove".into(), "Kalibrierung".into()],
        lists: vec![BundleList {
            name: list,
            kind: ListKind::Print,
            items: files.iter().map(|f| BundleItem { path: f.clone(), quantity: 1 }).collect(),
        }],
    };
    let result = json!({"files": files, "profile": profile});
    Ok((bundle, result))
}

/// A fresh drawer for the panel.
pub(crate) fn default_project() -> DrawerProject {
    DrawerProject {
        drawer: DrawerSpec { name: "Schublade".into(), width: 250.0, depth: 305.0, height: 100.0 },
        bed: bed_of(None),
        ..DrawerProject::default()
    }
}

meshgrove_plugin_api::export_plugin!(Bingrove);

//! The drawer panel: form on the left, the plan as a grid in the middle, parts and a 3D view
//! on the right. Every change re-plans (cheap); geometry only runs for the 3D view and the
//! export job.

use std::cell::RefCell;
use std::cmp::Ordering;

use bingrove_core::catalog::{cradle_cells, holes_grid, rings_grid, row_cells, sets, vslot_cells};
use bingrove_core::grid::Zone;
use bingrove_core::model::{
    Align, BinSpec, Cover, CradleSlot, Cutout, DrawerProject, Drive, EdgeMode, HolderSpec, HoleShape, Kind, Placement,
    PlateFloor, RoundItem, RowHole, Side, VSlot,
};
use bingrove_core::names::mm;
use bingrove_core::plan::{PartSpec, Plan, plan};
use bingrove_core::silhouette::{self, Down};
use bingrove_core::spec::{GRID, UNIT};
use bingrove_core::suggest::{self, Mix};
use meshgrove_plugin_api::guest::library;
use meshgrove_plugin_api::guest::{self, GuestPanel, MeshData};
use meshgrove_plugin_api::ui::{
    Action, Choice, Effect, Event, FormRow, GridChange, GridEditor, GridItem, GridZone, JobSpec, Level, Node, Tab,
    TableColumn, TableRow, TextStyle, Unit, View,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{ListMode, default_project};
use bingrove_core::catalog::catalog;

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct PanelState {
    pub project: DrawerProject,
    pub lists: ListMode,
    /// Grows with every change, so the 3D view asks for the mesh anew.
    pub revision: u64,
    /// Part shown in the 3D view; None = the whole drawer.
    pub selected_part: Option<usize>,
    pub selected_items: Vec<String>,
    pub busy: bool,
    pub message: Option<(Level, String)>,
    /// A library model whose outline the next "Silhouette" action cuts into a bin.
    pub model_id: String,
    pub model_down: Down,
    /// The preview the 3D view shows (a key into `/data/preview`), the one a job is computing,
    /// and how far that job is.
    pub preview: Option<String>,
    pub preview_job: Option<String>,
    pub preview_progress: Option<(f32, String)>,
    /// How many bins the user asked for (0: not said), and a mix of sizes they typed that does
    /// not fit yet, with the reason.
    pub bins_wanted: u32,
    pub mix_edit: Option<(Mix, String)>,
    /// The left column's tab: 0 Projekt, 1 Belegung, 2 Ausgabe.
    pub tab: usize,
}

impl Default for PanelState {
    fn default() -> Self {
        PanelState {
            project: default_project(),
            lists: ListMode::Both,
            revision: 0,
            selected_part: None,
            selected_items: vec![],
            busy: false,
            message: None,
            model_id: String::new(),
            model_down: Down::Z,
            preview: None,
            preview_job: None,
            preview_progress: None,
            bins_wanted: 0,
            mix_edit: None,
            tab: 0,
        }
    }
}

/// The bins of a plan as a mix of footprints (holders left out).
fn bins_of(p: &Plan) -> Mix {
    Mix::of(p.placements.iter().filter_map(|pl| match &p.parts[pl.part].part {
        PartSpec::Bin { spec, .. } => Some(spec.cells),
        _ => None,
    }))
}

impl PanelState {
    pub(crate) fn from_project(project: DrawerProject) -> Self {
        PanelState { project, ..PanelState::default() }
    }
}

pub(crate) struct DrawerPanel {
    state: RefCell<PanelState>,
    plan: RefCell<Option<Result<Plan, String>>>,
}

/// A short name of a holder for its grid cell.
fn holder_label(h: &HolderSpec) -> String {
    match h {
        HolderSpec::Tape(_) => "Tape".into(),
        HolderSpec::Holes(s) => s.item.slug().to_uppercase(),
        HolderSpec::Bits(_) => "Bits".into(),
        HolderSpec::Troughs(t) => {
            if t.handle.is_some() {
                "Schraubendreher".into()
            } else {
                "liegend".into()
            }
        }
        HolderSpec::Row(r) => r.title.clone().unwrap_or_else(|| format!("{} Löcher", r.hole_count())),
        HolderSpec::Slots(_) => "Schlitze".into(),
        HolderSpec::Cradles(c) => c.title.clone().unwrap_or_else(|| format!("{} Wiegen", c.slots.len())),
        HolderSpec::VSlots(v) => v.title.clone().unwrap_or_else(|| format!("{} V-Schlitze", v.slots.len())),
        HolderSpec::Rings(r) => r.label.clone().unwrap_or_else(|| format!("Ringe {}", mm(r.d))),
    }
}

/// Makes placement `p` the catalogue holder `id` on its cells, or a plain bin for any other id.
fn set_holder(p: &mut Placement, id: &str) {
    match catalog().into_iter().find(|c| c.id == id) {
        Some(entry) => {
            let holder = entry.holder.with_cells(p.cells());
            p.bin.cells = holder.cells();
            p.bin.units = holder.units();
            p.holder = Some(holder);
        }
        None => p.holder = None,
    }
}

/// The holder fields of the selection form ("sel_" stripped) that carry numbers.
const HOLDER_FIELDS: [&str; 33] = [
    "window_w",
    "window_angle",
    "cradle_angle",
    "cradle_clear",
    "row_tilt",
    "vslot_clear",
    "ring_d",
    "ring_h",
    "ring_nx",
    "ring_ny",
    "ring_clear",
    "hole_d",
    "hole_h",
    "clear",
    "hole_depth",
    "hole_pitch",
    "hole_nx",
    "hole_ny",
    "hole_chamfer",
    "trough_d",
    "trough_len",
    "handle",
    "trough_n",
    "tape_od",
    "tape_h",
    "tape_sx",
    "tape_sy",
    "slot_w",
    "slot_depth",
    "slot_pitch",
    "slot_len",
    "slot_n",
    "row_clear",
];

/// Sets one numeric holder field of placement `p`; a changed hole size makes the item a custom one.
fn holder_field(p: &mut Placement, field: &str, v: f64) {
    let whole = |v: f64| v.round().max(0.0) as u32;
    if let Some(w) = &mut p.bin.window {
        match field {
            "window_w" => w.width = v.max(6.0),
            "window_angle" => w.angle = v.clamp(0.0, 45.0),
            _ => {}
        }
    }
    match (&mut p.holder, field) {
        (Some(HolderSpec::Holes(h)), "hole_d" | "hole_h") => {
            let (d, height) = h.item.size();
            h.item = if field == "hole_d" {
                RoundItem::Custom { d: v.max(1.0), h: height }
            } else {
                RoundItem::Custom { d, h: v.max(5.0) }
            };
        }
        (Some(HolderSpec::Holes(h)), "clear") => h.clearance = v.clamp(0.0, 5.0),
        (Some(HolderSpec::Holes(h)), "hole_depth") => h.depth = (v > 0.0).then_some(v),
        (Some(HolderSpec::Holes(h)), "hole_pitch") => h.pitch = (v > 0.0).then_some(v),
        (Some(HolderSpec::Holes(h)), "hole_nx" | "hole_ny") => {
            let mut c = h.count.unwrap_or_else(|| holes_grid(h));
            c[usize::from(field == "hole_ny")] = whole(v);
            h.count = (c[0] > 0 && c[1] > 0).then_some(c);
        }
        (Some(HolderSpec::Holes(h)), "hole_chamfer") => h.chamfer = v.clamp(0.0, 3.0),
        (Some(HolderSpec::Troughs(t)), "trough_d") => t.diameter = v.max(2.0),
        (Some(HolderSpec::Troughs(t)), "trough_len") => t.length = v.max(20.0),
        (Some(HolderSpec::Troughs(t)), "handle") => t.handle = (v > 0.0).then_some(v),
        (Some(HolderSpec::Troughs(t)), "trough_n") => t.count = (v >= 1.0).then(|| whole(v)),
        (Some(HolderSpec::Tape(t)), "tape_od") => t.od = v.max(30.0),
        (Some(HolderSpec::Tape(t)), "tape_h") => t.height = whole(v).clamp(1, 10),
        (Some(HolderSpec::Tape(t)), "tape_sx") => t.stacks[0] = whole(v).clamp(1, 6),
        (Some(HolderSpec::Tape(t)), "tape_sy") => t.stacks[1] = whole(v).clamp(1, 6),
        (Some(HolderSpec::Slots(s)), "slot_w") => s.width = v.max(0.5),
        (Some(HolderSpec::Slots(s)), "slot_depth") => s.depth = v.max(2.0),
        (Some(HolderSpec::Slots(s)), "slot_pitch") => s.pitch = v.max(1.0),
        (Some(HolderSpec::Slots(s)), "slot_len") => s.length = (v > 0.0).then_some(v),
        (Some(HolderSpec::Slots(s)), "slot_n") => s.count = (v >= 1.0).then(|| whole(v)),
        (Some(HolderSpec::Row(r)), "row_clear") => r.clearance = v.clamp(0.0, 5.0),
        (Some(HolderSpec::Cradles(c)), "cradle_angle") => c.angle = v.clamp(0.0, 30.0),
        (Some(HolderSpec::Cradles(c)), "cradle_clear") => c.clearance = v.clamp(0.0, 5.0),
        (Some(HolderSpec::Row(r)), "row_tilt") => r.tilt = v.clamp(0.0, 30.0),
        (Some(HolderSpec::VSlots(vs)), "vslot_clear") => vs.clearance = v.clamp(0.0, 5.0),
        (Some(HolderSpec::Rings(r)), "ring_d") => r.d = v.max(5.0),
        (Some(HolderSpec::Rings(r)), "ring_h") => r.height = v.max(3.0),
        (Some(HolderSpec::Rings(r)), "ring_nx" | "ring_ny") => {
            let mut c = r.count.unwrap_or_else(|| rings_grid(r));
            c[usize::from(field == "ring_ny")] = whole(v);
            r.count = (c[0] > 0 && c[1] > 0).then_some(c);
        }
        (Some(HolderSpec::Rings(r)), "ring_clear") => r.clearance = v.clamp(0.0, 5.0),
        _ => {}
    }
    refresh_holder(p);
}

/// "14x40=Zange, 10x35" as V-slots: thickness × depth, with a label after "=".
fn parse_vslots(text: &str) -> Vec<VSlot> {
    text.split([',', ';'])
        .filter_map(|part| {
            let part = part.trim();
            let (size, label) = match part.split_once('=') {
                Some((s, l)) => (s.trim(), l.trim()),
                None => (part, ""),
            };
            let (w, d) = size.split_once(['x', '×'])?;
            let (width, depth): (f64, f64) = (w.trim().parse().ok()?, d.trim().parse().ok()?);
            (width > 0.0 && depth > 0.0).then(|| VSlot { width, depth, label: label.to_string() })
        })
        .collect()
}

fn vslot_text(slots: &[VSlot]) -> String {
    slots
        .iter()
        .map(|s| {
            let size = format!("{}x{}", mm(s.width), mm(s.depth));
            if s.label.is_empty() { size } else { format!("{size}={}", s.label) }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// "22x150=1/4, 30x200=3/8" as cradle places: width × length, with a label after "=".
fn parse_cradles(text: &str) -> Vec<CradleSlot> {
    text.split([',', ';'])
        .filter_map(|part| {
            let part = part.trim();
            let (size, label) = match part.split_once('=') {
                Some((s, l)) => (s.trim(), l.trim()),
                None => (part, ""),
            };
            let (w, l) = size.split_once(['x', '×'])?;
            let (width, length): (f64, f64) = (w.trim().parse().ok()?, l.trim().parse().ok()?);
            (width > 0.0 && length > 0.0).then(|| CradleSlot { width, length, label: label.to_string() })
        })
        .collect()
}

fn cradle_text(slots: &[CradleSlot]) -> String {
    slots
        .iter()
        .map(|s| {
            let size = format!("{}x{}", mm(s.width), mm(s.length));
            if s.label.is_empty() { size } else { format!("{size}={}", s.label) }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// After a holder changed: a row's footprint follows its holes, tape its rolls, and the bin takes
/// the holder's cells and height.
fn refresh_holder(p: &mut Placement) {
    match &mut p.holder {
        Some(HolderSpec::Row(r)) => r.cells = row_cells(r),
        Some(HolderSpec::Cradles(c)) => c.cells = cradle_cells(c),
        Some(HolderSpec::VSlots(v)) => v.cells = vslot_cells(v),
        _ => {}
    }
    if let Some(h) = &p.holder {
        p.bin.cells = h.cells();
        p.bin.units = h.units();
    }
}

fn side_value(side: Side) -> &'static str {
    match side {
        Side::Back => "back",
        Side::Front => "front",
        Side::Left => "left",
        Side::Right => "right",
    }
}

fn side_of(text: &str) -> Side {
    match text {
        "front" => Side::Front,
        "left" => Side::Left,
        "right" => Side::Right,
        _ => Side::Back,
    }
}

fn shape_of(text: &str) -> HoleShape {
    match text {
        "hex" => HoleShape::Hex,
        "square" => HoleShape::Square,
        _ => HoleShape::Round,
    }
}

/// "11.8=4, 13=5.5, 15" as holes: a bare number is size and label at once.
fn parse_row(text: &str) -> Vec<RowHole> {
    text.split([',', ';'])
        .filter_map(|part| {
            let part = part.trim();
            let (d, label) = match part.split_once('=') {
                Some((d, l)) => (d.trim(), l.trim()),
                None => (part, part),
            };
            let d: f64 = d.parse().ok()?;
            (d > 0.0).then(|| RowHole { d, depth: None, label: label.to_string() })
        })
        .collect()
}

/// The holes of a row as the text the user edits.
fn row_text(holes: &[RowHole]) -> String {
    holes
        .iter()
        .map(|h| {
            let d = mm(h.d);
            if h.label.is_empty() || h.label == d { d } else { format!("{d}={}", h.label) }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// The catalogue id of a placement's holder: "none" for a plain bin, "custom" for a holder the
/// catalogue does not have (an AI's own sizes).
fn holder_id(p: &Placement) -> &'static str {
    match &p.holder {
        None => "none",
        Some(h) => {
            let fallback = match h {
                HolderSpec::Row(_) => "row",
                HolderSpec::Cradles(_) => "cradles",
                HolderSpec::VSlots(_) => "vslots",
                HolderSpec::Rings(_) => "rings",
                _ => "custom",
            };
            catalog().into_iter().find(|c| c.holder.clone().with_cells(h.cells()) == *h).map_or(fallback, |c| c.id)
        }
    }
}

/// Items are named by their front-left cell, so the selection survives re-planning; a moved
/// bin gets a new id, which the plugin selects itself.
fn item_id(p: &Placement) -> String {
    format!("{}_{}", p.x, p.y)
}

/// Colour of a bin by its height: low bins light, tall bins deep green; holders amber.
fn colour(units: u32, max: u32, holder: bool) -> String {
    if holder {
        return "#f9b25c".into();
    }
    let t = if max <= 1 { 1.0 } else { (units.min(max) as f32 - 1.0) / (max as f32 - 1.0) };
    let mix = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
    format!("#{:02x}{:02x}{:02x}", mix(0x5c, 0x0f), mix(0xc7, 0x5a), mix(0x9a, 0x50))
}

impl DrawerPanel {
    fn replan(&self) {
        let s = self.state.borrow();
        *self.plan.borrow_mut() = Some(plan(&s.project).map_err(|e| e.to_string()));
    }

    fn with_plan<T>(&self, f: impl FnOnce(&Plan) -> T) -> Option<T> {
        if self.plan.borrow().is_none() {
            self.replan();
        }
        match &*self.plan.borrow() {
            Some(Ok(p)) => Some(f(p)),
            _ => None,
        }
    }

    /// Takes the plan's (repaired, filled) placements back into the project, so edits start
    /// from what the user sees.
    fn adopt_plan(&self) {
        let placements = self.with_plan(|p| p.placements.iter().map(|q| q.placement.clone()).collect::<Vec<_>>());
        if let Some(placements) = placements {
            let mut s = self.state.borrow_mut();
            s.project.placements = placements;
            s.project.items.clear();
        }
    }

    fn changed(&self) {
        let mut s = self.state.borrow_mut();
        s.revision += 1;
        s.message = None;
        s.mix_edit = None;
        drop(s);
        self.replan();
    }

    /// The mixes offered for the number of bins the user wants (or has).
    fn mixes(&self, s: &PanelState, p: &Plan) -> Vec<Mix> {
        let count = if s.bins_wanted > 0 { s.bins_wanted } else { bins_of(p).count() };
        suggest::suggest(p.grid.cells, count, 4)
    }

    /// Lays the drawer out with `mix`, replacing every placement; when the mix does not fit, it
    /// stays in the form with the reason.
    fn apply_mix(&self, mix: Mix) -> Vec<Effect> {
        let Some((cells, h)) = self.with_plan(|p| (p.grid.cells, p.heights)) else { return Vec::new() };
        let area = cells[0] * cells[1];
        match suggest::place(cells, &mix, h) {
            Some(placements) => {
                let mut s = self.state.borrow_mut();
                s.project.placements = placements;
                s.project.items.clear();
                s.selected_items.clear();
                drop(s);
                self.changed();
            }
            None => {
                let why = match mix.area().cmp(&area) {
                    Ordering::Less => format!("{} Zellen bleiben frei", area - mix.area()),
                    Ordering::Greater => format!("{} Zellen zu viel", mix.area() - area),
                    Ordering::Equal => "so lassen sich die Größen nicht ins Raster legen".to_string(),
                };
                self.state.borrow_mut().mix_edit = Some((mix, why));
            }
        }
        Vec::new()
    }

    /// The count of one size ("2x1") in the mix form changed.
    fn mix_changed(&self, size: &str, n: u32) -> Vec<Effect> {
        let Some((w, d)) = size.split_once('x').and_then(|(w, d)| Some((w.parse().ok()?, d.parse().ok()?))) else {
            return Vec::new();
        };
        let base = match &self.state.borrow().mix_edit {
            Some((m, _)) => m.clone(),
            None => self.with_plan(bins_of).unwrap_or_default(),
        };
        self.apply_mix(base.with([w, d], n))
    }

    /// A suggested mix was clicked.
    fn mix_clicked(&self, index: &str) -> Vec<Effect> {
        let Ok(i) = index.parse::<usize>() else { return Vec::new() };
        let s = self.state.borrow();
        let mix = self.with_plan(|p| self.mixes(&s, p).into_iter().nth(i)).flatten();
        drop(s);
        mix.map_or_else(Vec::new, |m| self.apply_mix(m))
    }

    /// Keeps the 3D view's preview in step with the plan: one that is already there is shown,
    /// a missing one is computed by a job. Never here: a panel call has two seconds, a drawer
    /// takes longer in the sandbox. Runs after every event, so it must not repeat itself.
    fn preview_effects(&self) -> Vec<Effect> {
        let Some(wanted) = self.with_plan(|p| crate::preview::key(p, &self.state.borrow().project.profile)) else {
            return Vec::new();
        };
        let mut s = self.state.borrow_mut();
        let mut effects = Vec::new();
        if let Some(job) = s.preview_job.clone().filter(|j| *j != wanted) {
            s.preview_job = None;
            s.preview_progress = None;
            effects.push(Effect::CancelJob { job: format!("preview:{job}") });
        }
        let shown = s.preview.as_deref() == Some(wanted.as_str()) && crate::preview::ready(&wanted);
        if shown || s.preview_job.is_some() {
            return effects;
        }
        if crate::preview::ready(&wanted) {
            s.preview = Some(wanted);
            s.revision += 1;
            return effects;
        }
        effects.push(Effect::StartJobs {
            jobs: vec![JobSpec {
                id: format!("preview:{wanted}"),
                kind: "preview".into(),
                params: json!({"project": s.project, "key": wanted}),
                label: Some("Vorschau".into()),
            }],
        });
        s.preview_job = Some(wanted);
        effects
    }

    /// A preview job ended. Results of a job the panel no longer waits for (cancelled,
    /// overtaken by a change) are ignored.
    fn preview_finished(&self, key: &str, error: Option<String>) -> Vec<Effect> {
        let mut s = self.state.borrow_mut();
        if s.preview_job.as_deref() != Some(key) {
            return Vec::new();
        }
        s.preview_job = None;
        s.preview_progress = None;
        match error {
            None => {
                s.preview = Some(key.to_owned());
                s.revision += 1;
                Vec::new()
            }
            Some(e) => vec![Effect::Toast { text: format!("Vorschau fehlgeschlagen: {e}"), level: Level::Warn }],
        }
    }

    /// The export or calibration job ended: its bundle goes into the library.
    fn job_finished(&self, result: Option<Value>, error: Option<String>) -> Vec<Effect> {
        let mut s = self.state.borrow_mut();
        s.busy = false;
        if let Some(error) = error {
            s.message = Some((Level::Error, error.clone()));
            return vec![Effect::Toast { text: error, level: Level::Error }];
        }
        let bundle = result.map_or(Value::Null, |r| r["bundle"].clone());
        match serde_json::from_value::<meshgrove_plugin_api::Bundle>(bundle) {
            Ok(bundle) => {
                s.message = Some((Level::Success, bundle.summary.clone().unwrap_or_default()));
                vec![Effect::ApplyBundle { bundle }]
            }
            Err(e) => vec![Effect::Toast { text: e.to_string(), level: Level::Error }],
        }
    }

    fn form(&self, s: &PanelState, plan: Option<&Plan>) -> Node {
        let d = &s.project.drawer;
        let o = &s.project.options;
        let row = |label: &str, node: Node| FormRow { label: label.into(), node };
        let select = |id: &str, options: &[(&str, &str)], selected: &str| Node::Select {
            id: id.into(),
            options: options.iter().map(|(v, l)| Choice { value: (*v).into(), label: (*l).into() }).collect(),
            selected: selected.into(),
            enabled: true,
        };
        let align = |a: Align| match a {
            Align::Start => "start",
            Align::Center => "center",
            Align::End => "end",
        };
        let info = guest::info();
        let mut printers: Vec<(String, String)> = info
            .printers
            .iter()
            .map(|p| (p.name.clone(), format!("{} ({:.0} × {:.0} mm)", p.name, p.bed[0], p.bed[1])))
            .collect();
        if printers.is_empty() {
            printers.push((
                s.project.bed.name.clone(),
                format!("{} ({:.0} × {:.0} mm)", s.project.bed.name, s.project.bed.x, s.project.bed.y),
            ));
        }
        let surface = o.kind == Kind::Surface;
        let mut rows = vec![
            row(
                "Art",
                select(
                    "kind",
                    &[("drawer", "Schubladeneinsatz"), ("surface", "Fläche (Werkbank, Tisch)")],
                    if surface { "surface" } else { "drawer" },
                ),
            ),
            row("Name", Node::text_input("name", d.name.clone())),
        ];
        if surface {
            let cells = plan.map_or([0, 0], |p| p.grid.cells);
            let int = |id: &str, value: u32| Node::NumberInput {
                id: id.into(),
                value: f64::from(value),
                min: Some(1.0),
                max: Some(35.0),
                step: Some(1.0),
                unit: Unit::None,
                decimals: Some(0),
                enabled: true,
            };
            rows.push(row("Zellen quer × längs", Node::row(vec![int("cells_x", cells[0]), int("cells_y", cells[1])])));
            rows.push(row(
                "Platte",
                Node::Label {
                    text: format!("{:.0} × {:.0} mm, Zellen bleiben frei, wo nichts steht", d.width, d.depth),
                    style: TextStyle::Small,
                },
            ));
            rows.push(row("Höhe der Bins", Node::mm("height", d.height, 0.0, 1000.0)));
        } else {
            rows.push(row("Breite", Node::mm("width", d.width, 42.0, 1500.0)));
            rows.push(row("Tiefe", Node::mm("depth", d.depth, 42.0, 1500.0)));
            rows.push(row("Höhe innen", Node::mm("height", d.height, 0.0, 1000.0)));
            rows.push(row(
                "Belegung",
                Node::Checkbox {
                    id: "cover".into(),
                    label: "Zellen dürfen frei bleiben".into(),
                    checked: o.cover == Cover::Partial,
                    enabled: true,
                },
            ));
        }
        let drawer_rows = vec![
            row("Randzone ab", Node::mm("edge_threshold", o.edge_threshold, 0.0, 42.0)),
            row(
                "Randbins",
                select(
                    "edge_mode",
                    &[
                        ("to_floor", "bis zum Boden (ohne Stützen)"),
                        ("resting", "aufliegend (Stützen nötig)"),
                        ("none", "keine – Rand bleibt Platte"),
                    ],
                    match o.edge_mode {
                        EdgeMode::ToFloor => "to_floor",
                        EdgeMode::Resting => "resting",
                        EdgeMode::None => "none",
                    },
                ),
            ),
            row(
                "Raster X",
                select(
                    "align_x",
                    &[("start", "links bündig"), ("center", "mittig"), ("end", "rechts bündig")],
                    align(o.align[0]),
                ),
            ),
            row(
                "Raster Y",
                select(
                    "align_y",
                    &[("start", "vorne bündig"), ("center", "mittig"), ("end", "hinten bündig")],
                    align(o.align[1]),
                ),
            ),
        ];
        if !surface {
            rows.extend(drawer_rows);
        }
        let drawer = Node::Form { id: "drawer".into(), rows, submit: None };
        let has_floor = o.floor != PlateFloor::Open;
        let plate = Node::Form {
            id: "plate".into(),
            rows: vec![
                row(
                    "Platte",
                    select(
                        "floor",
                        &[("open", "offener Rahmen"), ("solid", "mit Boden"), ("magnets", "Boden mit Magnetlöchern")],
                        match o.floor {
                            PlateFloor::Open => "open",
                            PlateFloor::Solid => "solid",
                            PlateFloor::Magnets => "magnets",
                        },
                    ),
                ),
                row(
                    "Befestigung",
                    Node::row(vec![
                        Node::Checkbox {
                            id: "screws".into(),
                            label: "Schraublöcher (gesenkt, in den Ecken)".into(),
                            checked: has_floor && o.mount.screws,
                            enabled: has_floor,
                        },
                        Node::Checkbox {
                            id: "connectors".into(),
                            label: "Schwalbenschwanz zwischen den Teilen".into(),
                            checked: has_floor && o.mount.connectors,
                            enabled: has_floor,
                        },
                    ]),
                ),
            ],
            submit: None,
        };
        let output = Node::Form {
            id: "output".into(),
            rows: vec![
                row(
                    "Drucker",
                    Node::Select {
                        id: "printer".into(),
                        options: printers.iter().map(|(v, l)| Choice { value: v.clone(), label: l.clone() }).collect(),
                        selected: s.project.bed.name.clone(),
                        enabled: true,
                    },
                ),
                row(
                    "Drucklisten",
                    select(
                        "lists",
                        &[
                            ("both", "Teile und Druckbetten"),
                            ("parts", "nur Teile (STL × Anzahl)"),
                            ("beds", "nur Druckbetten (3MF)"),
                        ],
                        match s.lists {
                            ListMode::Both => "both",
                            ListMode::Parts => "parts",
                            ListMode::Beds => "beds",
                        },
                    ),
                ),
                row("Magnetloch Ø", Node::mm("magnet_d", s.project.profile.magnet.d, 2.0, 12.0)),
                row("Magnetloch Tiefe", Node::mm("magnet_h", s.project.profile.magnet.h, 0.5, 6.0)),
            ],
            submit: None,
        };
        let tab = |label: &str, children: Vec<Node>| Tab { label: label.into(), children };
        Node::Tabs {
            id: "left_tabs".into(),
            selected: s.tab.min(2),
            tabs: vec![
                tab("Projekt", vec![drawer, plate]),
                tab("Belegung", self.compartments(s, plan)),
                tab("Ausgabe", vec![output]),
            ],
        }
    }

    /// The "Fächer" group: how many bins the user wants, mixes of sizes that make that many, and
    /// the sizes of the layout with their counts to shift around.
    fn compartments(&self, s: &PanelState, plan: Option<&Plan>) -> Vec<Node> {
        let Some(p) = plan else { return Vec::new() };
        let cells = p.grid.cells;
        let area = cells[0] * cells[1];
        let current = bins_of(p);
        let shown = if s.bins_wanted > 0 { s.bins_wanted } else { current.count() };
        let count = |id: String, value: u32, min: f64| Node::NumberInput {
            id,
            value: f64::from(value),
            min: Some(min),
            max: Some(f64::from(area)),
            step: Some(1.0),
            unit: Unit::None,
            decimals: Some(0),
            enabled: true,
        };
        let small = |text: String| Node::Label { text, style: TextStyle::Small };
        let mut nodes = vec![Node::Form {
            id: "bins_form".into(),
            rows: vec![FormRow {
                label: "Gewünscht".into(),
                node: Node::row(vec![count("bins".into(), shown, 1.0), small(format!("derzeit {}", current.count()))]),
            }],
            submit: None,
        }];
        let mixes = self.mixes(s, p);
        if mixes.is_empty() {
            nodes.push(small(format!(
                "Für {shown} Fächer in {} × {} gibt es keine gleichmäßige Aufteilung",
                cells[0], cells[1]
            )));
        } else {
            nodes.push(small(format!("Vorschläge für {shown} Fächer (ein Klick belegt die Schublade neu):")));
            for (i, m) in mixes.iter().enumerate() {
                nodes.push(Node::button(format!("mix:{i}"), m.text()));
            }
        }
        // Sizes to shift counts between: those in the layout and those in the suggestions.
        let edit = s.mix_edit.as_ref().map_or(&current, |(m, _)| m);
        let mut sizes: Vec<[u32; 2]> = edit.sizes.iter().map(|(c, _)| *c).collect();
        for c in mixes.iter().flat_map(|m| m.sizes.iter().map(|(c, _)| *c)) {
            if !sizes.contains(&c) {
                sizes.push(c);
            }
        }
        sizes.sort_by_key(|c| std::cmp::Reverse((c[0] * c[1], c[0])));
        let rows = sizes
            .iter()
            .map(|&[w, d]| FormRow {
                label: format!("{w}×{d}"),
                node: count(format!("mixn:{w}x{d}"), edit.get([w, d]), 0.0),
            })
            .collect();
        nodes.push(Node::Form { id: "mix".into(), rows, submit: None });
        nodes.push(match &s.mix_edit {
            Some((m, why)) => Node::Notice {
                level: Level::Warn,
                text: format!("{} Fächer, {} von {area} Zellen: {why}", m.count(), m.area()),
            },
            None => small(format!("{} Fächer, {area} Zellen; eine Änderung belegt die Schublade neu", current.count())),
        });
        nodes
    }

    fn grid(&self, s: &PanelState, p: &Plan) -> Node {
        let g = &p.grid;
        let [nx, ny] = g.cells;
        let m = |side: Side| g.margin(side) as f32 / GRID as f32;
        let zone = |side: Side, x: f32, y: f32, w: f32, h: f32| GridZone {
            id: format!("zone_{}", side.letter()),
            x,
            y,
            w,
            h,
            label: Some(format!("{:.1} mm", g.margin(side))),
            color: Some(if g.zone(side) == Zone::Edge { "#2ea67e55".into() } else { "#56566355".into() }),
            accepts_items: g.zone(side) == Zone::Edge,
        };
        let zones = vec![
            zone(Side::Left, -m(Side::Left), 0.0, m(Side::Left), ny as f32),
            zone(Side::Right, nx as f32, 0.0, m(Side::Right), ny as f32),
            // The editor's y runs down the screen: the front zone sits below the grid.
            zone(Side::Front, 0.0, ny as f32, nx as f32, m(Side::Front)),
            zone(Side::Back, 0.0, -m(Side::Back), nx as f32, m(Side::Back)),
        ];
        let actions = vec![
            Action { id: "taller".into(), label: "Höher (+1 Einheit)".into() },
            Action { id: "lower".into(), label: "Niedriger (−1 Einheit)".into() },
            Action { id: "div_x".into(), label: "Fach teilen (quer)".into() },
            Action { id: "div_y".into(), label: "Fach teilen (längs)".into() },
            Action { id: "div_none".into(), label: "Fächer entfernen".into() },
            Action { id: "label_tab".into(), label: "Label-Lasche an/aus".into() },
            Action { id: "scoop".into(), label: "Griffmulde an/aus".into() },
            Action { id: "open_front".into(), label: "Vorne offen an/aus".into() },
            Action { id: "fingers".into(), label: "Fingerausschnitte an/aus".into() },
            Action { id: "window".into(), label: "Fenster vorne an/aus".into() },
            Action { id: "edges".into(), label: "Rand nutzen an/aus".into() },
            Action { id: "holder_none".into(), label: "Halter: keiner (normaler Bin)".into() },
            Action { id: "holder_aa".into(), label: "Halter: AA-Zellen".into() },
            Action { id: "holder_aaa".into(), label: "Halter: AAA-Zellen".into() },
            Action { id: "holder_18650".into(), label: "Halter: 18650-Akkus".into() },
            Action { id: "holder_bits".into(), label: "Halter: 1/4\"-Bits".into() },
            Action { id: "holder_pens".into(), label: "Halter: Stifte stehend".into() },
            Action { id: "holder_pens_lying".into(), label: "Halter: Stifte liegend".into() },
            Action { id: "holder_screwdrivers".into(), label: "Halter: Schraubendreher".into() },
            Action { id: "holder_tape".into(), label: "Halter: Tape-Rollen".into() },
            Action { id: "silhouette".into(), label: "Ausschnitt: Silhouette des Modells".into() },
            Action { id: "cutouts_none".into(), label: "Ausschnitte entfernen".into() },
        ];
        let items = p
            .placements
            .iter()
            .map(|q| {
                let pl = &q.placement;
                let [w, d] = pl.cells();
                let mut badges = vec![format!("{}u", pl.bin.units)];
                if pl.bin.dividers != [1, 1] {
                    badges.push(format!("{}×{}", pl.bin.dividers[0], pl.bin.dividers[1]));
                }
                if q.extents.iter().any(|&e| e > 0.0) {
                    badges.push("↔".into());
                }
                GridItem {
                    id: item_id(pl),
                    // The grid editor's y runs down the screen; the drawer's front is at the bottom.
                    x: pl.x as f32,
                    y: (ny - pl.y - d) as f32,
                    w: w as f32,
                    h: d as f32,
                    color: Some(colour(pl.bin.units, p.heights.max.min(24), pl.holder.is_some())),
                    label: Some(pl.label.clone().unwrap_or_else(|| match &pl.holder {
                        Some(h) => holder_label(h),
                        None => format!("{w}×{d}"),
                    })),
                    badges,
                    actions: actions.clone(),
                    locked: false,
                }
            })
            .collect();
        Node::GridEditor(GridEditor {
            id: "grid".into(),
            cols: nx,
            rows: ny,
            pitch: GRID as f32,
            snap_div: 1,
            zones,
            items,
            selected: s.selected_items.clone(),
            height: None,
        })
    }

    fn parts(&self, s: &PanelState, p: &Plan) -> Node {
        let rows = p
            .parts
            .iter()
            .enumerate()
            .map(|(i, b)| TableRow {
                id: i.to_string(),
                cells: vec![
                    b.name.clone(),
                    format!("{:.1} × {:.1} × {:.1}", b.size[0], b.size[1], b.size[2]),
                    b.quantity.to_string(),
                    if s.project.bed.fits(b.size[0], b.size[1]) { "✓".into() } else { "zu groß".into() },
                ],
            })
            .collect();
        Node::Table {
            id: "parts".into(),
            columns: vec![
                TableColumn { title: "Teil".into(), width: None, numeric: false },
                TableColumn { title: "mm".into(), width: None, numeric: false },
                TableColumn { title: "Anzahl".into(), width: Some(60.0), numeric: true },
                TableColumn { title: "Bett".into(), width: Some(60.0), numeric: false },
            ],
            rows,
            selected: s.selected_part.map(|i| i.to_string()),
            max_height: Some(260.0),
        }
    }

    /// Edits one placement by index (as the grid editor names items) and re-plans.
    fn edit(&self, index: usize, f: impl FnOnce(&mut Placement)) {
        self.adopt_plan();
        {
            let mut s = self.state.borrow_mut();
            if let Some(p) = s.project.placements.get_mut(index) {
                f(p);
            }
        }
        self.changed();
    }

    fn select(&self, ids: Vec<String>) {
        let mut s = self.state.borrow_mut();
        s.selected_items = ids;
        s.selected_part = None;
    }

    /// Applies `f` to every selected placement.
    fn edit_selected(&self, mut f: impl FnMut(&mut Placement)) {
        let ids = self.state.borrow().selected_items.clone();
        for id in ids {
            if let Some(i) = self.index_of(&id) {
                self.edit(i, &mut f);
            }
        }
    }

    /// A control of the selected bins changed: `field` is the id without "sel_".
    fn selected_changed(&self, field: &str, value: Value) -> Vec<Effect> {
        if let Some(v) = value.as_f64()
            && HOLDER_FIELDS.contains(&field)
        {
            self.edit_selected(|p| holder_field(p, field, v));
            return Vec::new();
        }
        let max = self.with_plan(|p| p.heights.max).unwrap_or(u32::MAX).max(1);
        let n = value.as_f64().map(|v| v.round().max(0.0) as u32);
        let on = value.as_bool();
        let text = value.as_str().map(|t| t.trim().to_owned());
        match (field, n, on, text) {
            ("units", Some(n), ..) => self.edit_selected(|p| p.bin.units = n.clamp(1, max)),
            ("div_x", Some(n), ..) => self.edit_selected(|p| p.bin.dividers[0] = n.clamp(1, 12)),
            ("div_y", Some(n), ..) => self.edit_selected(|p| p.bin.dividers[1] = n.clamp(1, 12)),
            ("label_tab", _, Some(on), _) => self.edit_selected(|p| p.bin.label_tab = on),
            ("label_side", _, _, Some(t)) => self.edit_selected(|p| p.bin.label_side = side_of(&t)),
            (f, _, Some(on), _) if f.starts_with("scoop_") => {
                let side = side_of(&f[6..]);
                self.edit_selected(|p| {
                    let mut sides = p.bin.scoops();
                    sides.retain(|s| *s != side);
                    if on {
                        sides.push(side);
                    }
                    p.bin.scoop = if sides.is_empty() { 0 } else { p.bin.scoop.max(60) };
                    p.bin.scoop_sides = sides;
                })
            }
            ("open_front", _, Some(on), _) => self.edit_selected(|p| p.bin.open_front = on),
            ("fingers", _, Some(on), _) => self.edit_selected(|p| p.bin.finger_cutouts = on),
            ("window", _, Some(on), _) => {
                self.edit_selected(|p| p.bin.window = on.then(bingrove_core::model::Window::default))
            }
            ("edges", _, Some(on), _) => self.edit_selected(|p| p.edges.off = [!on; 4]),
            ("label", _, _, Some(t)) => self.edit_selected(|p| p.label = (!t.is_empty()).then(|| t.clone())),
            ("holder", _, _, Some(id)) => self.edit_selected(|p| set_holder(p, &id)),
            ("engrave", _, _, Some(t)) => self.edit_selected(|p| p.bin.label_text = (!t.is_empty()).then(|| t.clone())),
            ("hole_label" | "slot_label", _, _, Some(t)) => self.edit_selected(|p| {
                let text = (!t.is_empty()).then(|| t.clone());
                match &mut p.holder {
                    Some(HolderSpec::Holes(h)) => h.label = text,
                    Some(HolderSpec::Slots(s)) => s.label = text,
                    _ => {}
                }
            }),
            ("hole_shape", _, _, Some(t)) => self.edit_selected(|p| {
                if let Some(HolderSpec::Holes(h)) = &mut p.holder {
                    h.shape = shape_of(&t);
                }
                refresh_holder(p);
            }),
            ("row_shape", _, _, Some(t)) => self.edit_selected(|p| {
                if let Some(HolderSpec::Row(r)) = &mut p.holder {
                    r.shape = shape_of(&t);
                }
                refresh_holder(p);
            }),
            ("row_title", _, _, Some(t)) => self.edit_selected(|p| {
                if let Some(HolderSpec::Row(r)) = &mut p.holder {
                    r.title = (!t.is_empty()).then(|| t.clone());
                }
                refresh_holder(p);
            }),
            ("row_pegs", _, _, Some(t)) => self.edit_selected(|p| {
                if let Some(HolderSpec::Row(r)) = &mut p.holder {
                    r.pegs = match t.as_str() {
                        "quarter" => Some(Drive::Quarter),
                        "three_eighths" => Some(Drive::ThreeEighths),
                        "half" => Some(Drive::Half),
                        _ => None,
                    };
                }
                refresh_holder(p);
            }),
            ("cradle_title", _, _, Some(t)) => self.edit_selected(|p| {
                if let Some(HolderSpec::Cradles(c)) = &mut p.holder {
                    c.title = (!t.is_empty()).then(|| t.clone());
                }
                refresh_holder(p);
            }),
            ("cradle_stepped", _, Some(on), _) => self.edit_selected(|p| {
                if let Some(HolderSpec::Cradles(c)) = &mut p.holder {
                    c.stepped = on;
                }
                refresh_holder(p);
            }),
            ("cradle_slots", _, _, Some(t)) => {
                let slots = parse_cradles(&t);
                if slots.is_empty() {
                    return vec![Effect::Toast {
                        text: "Keinen Platz erkannt: Breite×Länge=Text, durch Komma".into(),
                        level: Level::Warn,
                    }];
                }
                self.edit_selected(|p| {
                    if let Some(HolderSpec::Cradles(c)) = &mut p.holder {
                        c.slots = slots.clone();
                    }
                    refresh_holder(p);
                });
            }
            ("vslot_title", _, _, Some(t)) => self.edit_selected(|p| {
                if let Some(HolderSpec::VSlots(v)) = &mut p.holder {
                    v.title = (!t.is_empty()).then(|| t.clone());
                }
                refresh_holder(p);
            }),
            ("ring_label", _, _, Some(t)) => self.edit_selected(|p| {
                if let Some(HolderSpec::Rings(r)) = &mut p.holder {
                    r.label = (!t.is_empty()).then(|| t.clone());
                }
                refresh_holder(p);
            }),
            ("vslot_closed" | "vslot_ramp", _, Some(on), _) => self.edit_selected(|p| {
                if let Some(HolderSpec::VSlots(v)) = &mut p.holder {
                    if field == "vslot_ramp" {
                        v.ramp = on;
                    } else {
                        v.closed_end = on;
                    }
                }
                refresh_holder(p);
            }),
            ("vslot_slots", _, _, Some(t)) => {
                let slots = parse_vslots(&t);
                if slots.is_empty() {
                    return vec![Effect::Toast {
                        text: "Keinen Schlitz erkannt: Dicke×Tiefe=Text, durch Komma".into(),
                        level: Level::Warn,
                    }];
                }
                self.edit_selected(|p| {
                    if let Some(HolderSpec::VSlots(v)) = &mut p.holder {
                        v.slots = slots.clone();
                    }
                    refresh_holder(p);
                });
            }
            ("row_stepped", _, Some(on), _) => self.edit_selected(|p| {
                if let Some(HolderSpec::Row(r)) = &mut p.holder {
                    r.stepped = on;
                }
                refresh_holder(p);
            }),
            ("row_set", _, _, Some(id)) => {
                if let Some(set) = sets().into_iter().find(|s| s.id == id) {
                    self.edit_selected(|p| {
                        p.holder = Some(set.holder.clone());
                        refresh_holder(p);
                    });
                }
            }
            (f, _, _, Some(t)) if f.starts_with("row_holes:") => {
                let Ok(k) = f["row_holes:".len()..].parse::<usize>() else { return Vec::new() };
                let holes = parse_row(&t);
                if holes.is_empty() {
                    return vec![Effect::Toast {
                        text: "Keine Größe erkannt: Ø oder Ø=Text, durch Komma".into(),
                        level: Level::Warn,
                    }];
                }
                self.edit_selected(|p| {
                    if let Some(HolderSpec::Row(r)) = &mut p.holder
                        && let Some(row) = r.rows.get_mut(k)
                    {
                        *row = holes.clone();
                    }
                    refresh_holder(p);
                });
            }
            _ => {}
        }
        Vec::new()
    }

    /// Adds a row (a copy of the last) to or removes the last row from the selected row holders.
    fn row_count(&self, add: bool) -> Vec<Effect> {
        self.edit_selected(|p| {
            if let Some(HolderSpec::Row(r)) = &mut p.holder {
                if add {
                    r.rows.push(r.rows.last().cloned().unwrap_or_default());
                } else if r.rows.len() > 1 {
                    r.rows.pop();
                }
            }
            refresh_holder(p);
        });
        Vec::new()
    }

    /// The selected bins' settings under the grid: what the context menu does, as controls.
    fn selected_form(&self, s: &PanelState, p: &Plan) -> Option<Node> {
        let chosen: Vec<&Placement> = s
            .selected_items
            .iter()
            .filter_map(|id| p.placements.iter().find(|q| item_id(&q.placement) == *id))
            .map(|q| &q.placement)
            .collect();
        let first = *chosen.first()?;
        let [w, d] = first.cells();
        let title = match chosen.len() {
            1 => format!("Fach {w}×{d} in Spalte {}, Reihe {} von vorne", first.x + 1, first.y + 1),
            n => format!("{n} Fächer ausgewählt (Werte des ersten, Änderungen für alle)"),
        };
        let int = |id: &str, value: u32, min: u32, max: u32| Node::NumberInput {
            id: id.into(),
            value: f64::from(value),
            min: Some(f64::from(min)),
            max: Some(f64::from(max)),
            step: Some(1.0),
            unit: Unit::None,
            decimals: Some(0),
            enabled: true,
        };
        let check = |id: &str, label: &str, on: bool| Node::Checkbox {
            id: id.into(),
            label: label.into(),
            checked: on,
            enabled: true,
        };
        let small = |text: String| Node::Label { text, style: TextStyle::Small };
        let row = |label: &str, node: Node| FormRow { label: label.into(), node };
        let plain = first.holder.is_none();
        let mut rows = vec![row(
            "Höhe",
            Node::row(vec![
                int("sel_units", first.bin.units, 1, p.heights.max.max(1)),
                small(format!("Einheiten = {:.0} mm", f64::from(first.bin.units) * UNIT)),
            ]),
        )];
        if plain {
            rows.push(row(
                "Fächer quer × längs",
                Node::row(vec![
                    int("sel_div_x", first.bin.dividers[0], 1, 12),
                    int("sel_div_y", first.bin.dividers[1], 1, 12),
                ]),
            ));
            let side_select = |id: &str, side: Side| Node::Select {
                id: id.into(),
                options: [("back", "hinten"), ("front", "vorne"), ("left", "links"), ("right", "rechts")]
                    .iter()
                    .map(|(v, l)| Choice { value: (*v).into(), label: (*l).into() })
                    .collect(),
                selected: side_value(side).into(),
                enabled: true,
            };
            let mut label = vec![check("sel_label_tab", "Label-Lasche", first.bin.label_tab)];
            if first.bin.label_tab {
                label.push(side_select("sel_label_side", first.bin.label_side));
            }
            rows.push(row("Label", Node::row(label)));
            let scoops = first.bin.scoops();
            rows.push(row(
                "Griffmulde",
                Node::row(
                    [(Side::Front, "vorne"), (Side::Back, "hinten"), (Side::Left, "links"), (Side::Right, "rechts")]
                        .iter()
                        .map(|(side, l)| check(&format!("sel_scoop_{}", side_value(*side)), l, scoops.contains(side)))
                        .collect(),
                ),
            ));
            let mut extras = vec![
                check("sel_open_front", "vorne offen", first.bin.open_front),
                check("sel_fingers", "Fingerausschnitte", first.bin.finger_cutouts),
                check("sel_window", "Fenster vorne", first.bin.window.is_some()),
            ];
            if p.grid.zones.contains(&Zone::Edge) {
                extras.push(check("sel_edges", "Rand nutzen", !first.edges.off.iter().all(|&o| o)));
            }
            rows.push(row("Ausstattung", Node::row(extras)));
            if let Some(w) = &first.bin.window {
                rows.push(row(
                    "Fenster",
                    Node::row(vec![
                        Node::mm("sel_window_w", w.width, 6.0, 200.0),
                        small("breit".into()),
                        Node::mm("sel_window_angle", w.angle, 0.0, 45.0),
                        small("° schräg".into()),
                    ]),
                ));
            }
            rows.push(row(
                "Gravur",
                Node::TextInput {
                    id: "sel_engrave".into(),
                    value: first.bin.label_text.clone().unwrap_or_default(),
                    hint: Some("auf der Label-Lasche, sonst vorne".into()),
                    multiline: false,
                    enabled: true,
                },
            ));
        }
        if chosen.len() == 1 {
            rows.push(row(
                "Beschriftung",
                Node::TextInput {
                    id: "sel_label".into(),
                    value: first.label.clone().unwrap_or_default(),
                    hint: Some("Name im Plan".into()),
                    multiline: false,
                    enabled: true,
                },
            ));
        }
        let holder = holder_id(first);
        let mut options = vec![Choice { value: "none".into(), label: "keiner (normaler Bin)".into() }];
        options.extend(catalog().iter().map(|c| Choice { value: c.id.into(), label: c.label.into() }));
        if holder == "custom" {
            options.push(Choice { value: "custom".into(), label: "eigener Halter".into() });
        }
        rows.push(row(
            "Halter",
            Node::Select { id: "sel_holder".into(), options, selected: holder.into(), enabled: true },
        ));
        let shape_select = |id: &str, shape: HoleShape| Node::Select {
            id: id.into(),
            options: [("round", "rund"), ("hex", "Sechskant"), ("square", "quadratisch")]
                .iter()
                .map(|(v, l)| Choice { value: (*v).into(), label: (*l).into() })
                .collect(),
            selected: match shape {
                HoleShape::Round => "round",
                HoleShape::Hex => "hex",
                HoleShape::Square => "square",
            }
            .into(),
            enabled: true,
        };
        match &first.holder {
            Some(HolderSpec::Holes(h)) => {
                let (dia, height) = h.item.size();
                rows.push(row("Loch Ø", Node::mm("sel_hole_d", dia, 1.0, 80.0)));
                rows.push(row("Form", shape_select("sel_hole_shape", h.shape)));
                rows.push(row("Höhe des Dings", Node::mm("sel_hole_h", height, 5.0, 300.0)));
                rows.push(row(
                    "Tiefe (0 = aus der Höhe)",
                    Node::mm("sel_hole_depth", h.depth.unwrap_or(0.0), 0.0, 200.0),
                ));
                rows.push(row("Spiel je Loch", Node::mm("sel_clear", h.clearance, 0.0, 5.0)));
                rows.push(row("Teilung (0 = eng)", Node::mm("sel_hole_pitch", h.pitch.unwrap_or(0.0), 0.0, 200.0)));
                let [nx, ny] = h.count.unwrap_or([0, 0]);
                rows.push(row(
                    "Anzahl quer × längs (0 = so viele wie passen)",
                    Node::row(vec![int("sel_hole_nx", nx, 0, 40), int("sel_hole_ny", ny, 0, 40)]),
                ));
                rows.push(row("Fase", Node::mm("sel_hole_chamfer", h.chamfer, 0.0, 3.0)));
                rows.push(row(
                    "Gravur vorne",
                    Node::TextInput {
                        id: "sel_hole_label".into(),
                        value: h.label.clone().unwrap_or_default(),
                        hint: None,
                        multiline: false,
                        enabled: true,
                    },
                ));
            }
            Some(HolderSpec::Row(r)) => {
                let mut options = vec![Choice { value: String::new(), label: "Satz laden …".into() }];
                options.extend(sets().iter().map(|s| Choice { value: s.id.into(), label: s.label.into() }));
                rows.push(row(
                    "Satz",
                    Node::Select { id: "sel_row_set".into(), options, selected: String::new(), enabled: true },
                ));
                rows.push(row(
                    "Titel (graviert)",
                    Node::TextInput {
                        id: "sel_row_title".into(),
                        value: r.title.clone().unwrap_or_default(),
                        hint: Some("vorne am Halter".into()),
                        multiline: false,
                        enabled: true,
                    },
                ));
                rows.push(row(
                    "Halterung",
                    Node::Select {
                        id: "sel_row_pegs".into(),
                        options: [
                            ("holes", "Löcher"),
                            ("quarter", "Stecker 1/4\" (Nüsse)"),
                            ("three_eighths", "Stecker 3/8\" (Nüsse)"),
                            ("half", "Stecker 1/2\" (Nüsse)"),
                        ]
                        .iter()
                        .map(|(v, l)| Choice { value: (*v).into(), label: (*l).into() })
                        .collect(),
                        selected: match r.pegs {
                            None => "holes",
                            Some(Drive::Quarter) => "quarter",
                            Some(Drive::ThreeEighths) => "three_eighths",
                            Some(Drive::Half) => "half",
                        }
                        .into(),
                        enabled: true,
                    },
                ));
                if r.pegs.is_none() {
                    rows.push(row("Form", shape_select("sel_row_shape", r.shape)));
                }
                rows.push(row("Spiel je Loch", Node::mm("sel_row_clear", r.clearance, 0.0, 5.0)));
                if r.pegs.is_none() {
                    rows.push(row("Neigung nach hinten (°)", Node::mm("sel_row_tilt", r.tilt, 0.0, 30.0)));
                }
                rows.push(row("Stufen", check("sel_row_stepped", "hintere Reihen eine Einheit höher", r.stepped)));
                for (k, holes) in r.rows.iter().enumerate() {
                    rows.push(row(
                        &format!("Reihe {} (vorne = 1)", k + 1),
                        Node::TextInput {
                            id: format!("sel_row_holes:{k}"),
                            value: row_text(holes),
                            hint: Some("Ø oder Ø=Text, durch Komma getrennt".into()),
                            multiline: false,
                            enabled: true,
                        },
                    ));
                }
                rows.push(row(
                    "Reihen",
                    Node::row(vec![
                        Node::button("sel_row_add", "+ Reihe"),
                        Node::Button {
                            id: "sel_row_del".into(),
                            label: "− Reihe".into(),
                            primary: false,
                            enabled: r.rows.len() > 1,
                        },
                        small(format!("{} Löcher, {} × {} Zellen", r.hole_count(), r.cells[0], r.cells[1])),
                    ]),
                ));
            }
            Some(HolderSpec::Cradles(c)) => {
                let mut options = vec![Choice { value: String::new(), label: "Satz laden …".into() }];
                options.extend(
                    sets()
                        .iter()
                        .filter(|s| matches!(s.holder, HolderSpec::Cradles(_)))
                        .map(|s| Choice { value: s.id.into(), label: s.label.into() }),
                );
                rows.push(row(
                    "Satz",
                    Node::Select { id: "sel_row_set".into(), options, selected: String::new(), enabled: true },
                ));
                rows.push(row(
                    "Titel (graviert)",
                    Node::TextInput {
                        id: "sel_cradle_title".into(),
                        value: c.title.clone().unwrap_or_default(),
                        hint: Some("vorne am Halter".into()),
                        multiline: false,
                        enabled: true,
                    },
                ));
                rows.push(row(
                    "Plätze",
                    Node::TextInput {
                        id: "sel_cradle_slots".into(),
                        value: cradle_text(&c.slots),
                        hint: Some("Breite×Länge=Text je Platz, durch Komma".into()),
                        multiline: false,
                        enabled: true,
                    },
                ));
                rows.push(row("Neigung (°)", Node::mm("sel_cradle_angle", c.angle, 0.0, 30.0)));
                rows.push(row("Spiel", Node::mm("sel_cradle_clear", c.clearance, 0.0, 5.0)));
                rows.push(row(
                    "Stufen",
                    Node::row(vec![
                        check("sel_cradle_stepped", "jeder Platz weiter rechts eine Einheit höher", c.stepped),
                        small(format!("{} Plätze, {} × {} Zellen", c.slots.len(), c.cells[0], c.cells[1])),
                    ]),
                ));
            }
            Some(HolderSpec::VSlots(v)) => {
                let mut options = vec![Choice { value: String::new(), label: "Satz laden …".into() }];
                options.extend(
                    sets()
                        .iter()
                        .filter(|s| matches!(s.holder, HolderSpec::VSlots(_)))
                        .map(|s| Choice { value: s.id.into(), label: s.label.into() }),
                );
                rows.push(row(
                    "Satz",
                    Node::Select { id: "sel_row_set".into(), options, selected: String::new(), enabled: true },
                ));
                rows.push(row(
                    "Titel (graviert)",
                    Node::TextInput {
                        id: "sel_vslot_title".into(),
                        value: v.title.clone().unwrap_or_default(),
                        hint: Some("vorne am Halter".into()),
                        multiline: false,
                        enabled: true,
                    },
                ));
                rows.push(row(
                    "Schlitze",
                    Node::TextInput {
                        id: "sel_vslot_slots".into(),
                        value: vslot_text(&v.slots),
                        hint: Some("Dicke×Tiefe=Text je Schlitz, durch Komma".into()),
                        multiline: false,
                        enabled: true,
                    },
                ));
                rows.push(row("Spiel", Node::mm("sel_vslot_clear", v.clearance, 0.0, 5.0)));
                rows.push(row(
                    "Form",
                    Node::row(vec![
                        check("sel_vslot_closed", "hinten geschlossen", v.closed_end),
                        check("sel_vslot_ramp", "Rampe (Böden steigen nach hinten)", v.ramp),
                        small(format!("{} Schlitze, {} × {} Zellen", v.slots.len(), v.cells[0], v.cells[1])),
                    ]),
                ));
            }
            Some(HolderSpec::Rings(rg)) => {
                rows.push(row("Ring-Innen-Ø", Node::mm("sel_ring_d", rg.d, 5.0, 200.0)));
                rows.push(row("Ringhöhe", Node::mm("sel_ring_h", rg.height, 3.0, 100.0)));
                rows.push(row("Spiel", Node::mm("sel_ring_clear", rg.clearance, 0.0, 5.0)));
                let [nx, ny] = rg.count.unwrap_or([0, 0]);
                rows.push(row(
                    "Anzahl quer × längs (0 = so viele wie passen)",
                    Node::row(vec![int("sel_ring_nx", nx, 0, 20), int("sel_ring_ny", ny, 0, 20)]),
                ));
                rows.push(row(
                    "Gravur vorne",
                    Node::TextInput {
                        id: "sel_ring_label".into(),
                        value: rg.label.clone().unwrap_or_default(),
                        hint: None,
                        multiline: false,
                        enabled: true,
                    },
                ));
            }
            Some(HolderSpec::Slots(sl)) => {
                rows.push(row("Schlitzbreite", Node::mm("sel_slot_w", sl.width, 0.5, 30.0)));
                rows.push(row("Tiefe", Node::mm("sel_slot_depth", sl.depth, 2.0, 200.0)));
                rows.push(row("Teilung", Node::mm("sel_slot_pitch", sl.pitch, 1.0, 60.0)));
                rows.push(row(
                    "Länge (0 = ganze Tiefe)",
                    Node::mm("sel_slot_len", sl.length.unwrap_or(0.0), 0.0, 400.0),
                ));
                rows.push(row("Anzahl (0 = so viele wie passen)", int("sel_slot_n", sl.count.unwrap_or(0), 0, 60)));
                rows.push(row(
                    "Gravur vorne",
                    Node::TextInput {
                        id: "sel_slot_label".into(),
                        value: sl.label.clone().unwrap_or_default(),
                        hint: None,
                        multiline: false,
                        enabled: true,
                    },
                ));
            }
            Some(HolderSpec::Troughs(t)) => {
                rows.push(row("Durchmesser", Node::mm("sel_trough_d", t.diameter, 2.0, 60.0)));
                rows.push(row("Länge", Node::mm("sel_trough_len", t.length, 20.0, 600.0)));
                rows.push(row("Griff Ø (0 = ohne)", Node::mm("sel_handle", t.handle.unwrap_or(0.0), 0.0, 60.0)));
                rows.push(row("Anzahl (0 = so viele wie passen)", int("sel_trough_n", t.count.unwrap_or(0), 0, 20)));
            }
            Some(HolderSpec::Tape(t)) => {
                rows.push(row("Rollen-Ø", Node::mm("sel_tape_od", t.od, 30.0, 200.0)));
                rows.push(row(
                    "Stapel quer × längs",
                    Node::row(vec![int("sel_tape_sx", t.stacks[0], 1, 6), int("sel_tape_sy", t.stacks[1], 1, 6)]),
                ));
                rows.push(row("Rollen übereinander", int("sel_tape_h", t.height, 1, 10)));
            }
            Some(HolderSpec::Bits(_)) | None => {}
        }
        if plain {
            rows.push(row(
                "Modell (ID)",
                Node::TextInput {
                    id: "model_id".into(),
                    value: s.model_id.clone(),
                    hint: Some("Bibliothek/Ordner/datei.stl für die Silhouette".into()),
                    multiline: false,
                    enabled: true,
                },
            ));
            rows.push(row(
                "Modell liegt auf",
                Node::Select {
                    id: "model_down".into(),
                    options: [("z", "Unterseite (z unten)"), ("x", "Seite (x unten)"), ("y", "Seite (y unten)")]
                        .iter()
                        .map(|(v, l)| Choice { value: (*v).into(), label: (*l).into() })
                        .collect(),
                    selected: match s.model_down {
                        Down::Z => "z",
                        Down::X => "x",
                        Down::Y => "y",
                    }
                    .into(),
                    enabled: true,
                },
            ));
            let n = first.bin.cutouts.len();
            rows.push(row(
                "Ausschnitte",
                Node::row(vec![
                    small(if n == 0 { "keine".into() } else { n.to_string() }),
                    Node::button("sel_silhouette", "Silhouette des Modells"),
                    Node::Button {
                        id: "sel_cutouts_none".into(),
                        label: "entfernen".into(),
                        primary: false,
                        enabled: n > 0,
                    },
                ]),
            ));
        }
        Some(Node::Group {
            title: Some(title),
            collapsible: false,
            children: vec![Node::Form { id: "selected".into(), rows, submit: None }],
        })
    }

    /// The placement an item names (by its front-left cell), as an index into the project's
    /// placements after `adopt_plan`.
    fn index_of(&self, item: &str) -> Option<usize> {
        self.with_plan(|p| p.placements.iter().position(|q| item_id(&q.placement) == item)).flatten()
    }

    /// Cuts the outline of the panel's library model into placement `i`, centred, with 1 mm of air.
    fn silhouette(&self, i: usize) -> Vec<Effect> {
        let (id, down) = {
            let s = self.state.borrow();
            (s.model_id.trim().to_string(), s.model_down)
        };
        if id.is_empty() {
            return vec![Effect::Toast { text: "Erst eine Modell-ID eintragen".into(), level: Level::Warn }];
        }
        let mesh = match library::triangles(&id, 50_000) {
            Ok(m) => m,
            Err(e) => return vec![Effect::Toast { text: e, level: Level::Error }],
        };
        let Some((points, height)) = silhouette::outline(&mesh.positions, down) else {
            return vec![Effect::Toast { text: "Das Modell hat keine Fläche".into(), level: Level::Error }];
        };
        let size = silhouette::size(&points);
        let clearance = 1.0;
        let mut toast = None;
        self.edit(i, |p| {
            let [w, d] = p.cells().map(bingrove_core::spec::bin_outer);
            let inner = [
                w - 2.0 * bingrove_core::spec::WALL - 2.0 * clearance,
                d - 2.0 * bingrove_core::spec::WALL - 2.0 * clearance,
            ];
            if size[0] > inner[0] || size[1] > inner[1] {
                toast = Some(format!(
                    "Das Modell ({:.0} × {:.0} mm) passt nicht in den Bin ({:.0} × {:.0} mm innen)",
                    size[0], size[1], inner[0], inner[1]
                ));
                return;
            }
            let shift = [(w - size[0]) / 2.0, (d - size[1]) / 2.0];
            let points = points.iter().map(|q| [q[0] + shift[0], q[1] + shift[1]]).collect();
            let max_depth = bingrove_core::spec::body_height(p.bin.units)
                - bingrove_core::spec::DIVIDER_GAP
                - bingrove_core::spec::FLOOR_Z
                - bingrove_core::spec::FLOOR;
            let depth = (height * 0.7).min(max_depth).max(3.0);
            p.bin.cutouts.push(Cutout::Polygon { points, clearance, depth });
            p.holder = None;
            if p.label.is_none() {
                p.label = Some(id.rsplit('/').next().unwrap_or(&id).to_string());
            }
        });
        match toast {
            Some(text) => vec![Effect::Toast { text, level: Level::Warn }],
            None => vec![Effect::Toast {
                text: format!("Silhouette {:.0} × {:.0} mm eingetragen", size[0], size[1]),
                level: Level::Success,
            }],
        }
    }

    fn grid_change(&self, change: GridChange) -> Vec<Effect> {
        let ny = self.with_plan(|p| p.grid.cells[1]).unwrap_or(0);
        // Back to drawer coordinates: the editor's y runs down the screen.
        let to_y = |y: f32, h: f32| (ny as f32 - y - h).round().max(0.0) as u32;
        match change {
            GridChange::Moved { item, x, y } => {
                if let Some(i) = self.index_of(&item) {
                    let mut moved = String::new();
                    self.edit(i, |p| {
                        let d = p.cells()[1];
                        p.x = x.round().max(0.0) as u32;
                        p.y = to_y(y, d as f32);
                        moved = item_id(p);
                    });
                    self.select(vec![moved]);
                }
            }
            GridChange::Resized { item, x, y, w, h } => {
                if let Some(i) = self.index_of(&item) {
                    let mut resized = String::new();
                    self.edit(i, |p| {
                        if p.holder.is_none() {
                            p.bin.cells = [w.round().max(1.0) as u32, h.round().max(1.0) as u32];
                        }
                        p.x = x.round().max(0.0) as u32;
                        p.y = to_y(y, p.cells()[1] as f32);
                        resized = item_id(p);
                    });
                    self.select(vec![resized]);
                }
            }
            GridChange::Created { x, y, w, h } => {
                self.adopt_plan();
                let units = self.with_plan(|p| p.heights.front).unwrap_or(6);
                let cells = [w.round().max(1.0) as u32, h.round().max(1.0) as u32];
                let created = Placement::bin(
                    x.round().max(0.0) as u32,
                    to_y(y, cells[1] as f32),
                    BinSpec { cells, units, ..BinSpec::default() },
                );
                let id = item_id(&created);
                self.state.borrow_mut().project.placements.insert(0, created);
                self.changed();
                self.select(vec![id]);
            }
            GridChange::Deleted { items } => {
                let mut gone: Vec<usize> = items.iter().filter_map(|i| self.index_of(i)).collect();
                self.adopt_plan();
                gone.sort_unstable();
                gone.dedup();
                {
                    let mut s = self.state.borrow_mut();
                    for i in gone.into_iter().rev() {
                        if i < s.project.placements.len() {
                            s.project.placements.remove(i);
                        }
                    }
                }
                self.changed();
                self.select(vec![]);
            }
            GridChange::Merge { items } => {
                let mut idx: Vec<usize> = items.iter().filter_map(|i| self.index_of(i)).collect();
                idx.sort_unstable();
                idx.dedup();
                self.adopt_plan();
                let merged = {
                    let s = self.state.borrow();
                    let chosen: Vec<&Placement> = idx.iter().filter_map(|&i| s.project.placements.get(i)).collect();
                    (chosen.len() >= 2 && chosen.iter().all(|p| p.holder.is_none())).then(|| {
                        let x0 = chosen.iter().map(|p| p.x).min().unwrap();
                        let y0 = chosen.iter().map(|p| p.y).min().unwrap();
                        let x1 = chosen.iter().map(|p| p.x + p.cells()[0]).max().unwrap();
                        let y1 = chosen.iter().map(|p| p.y + p.cells()[1]).max().unwrap();
                        Placement::bin(x0, y0, BinSpec { cells: [x1 - x0, y1 - y0], ..chosen[0].bin.clone() })
                    })
                };
                if let Some(merged) = merged {
                    let id = item_id(&merged);
                    {
                        let mut s = self.state.borrow_mut();
                        for i in idx.into_iter().rev() {
                            s.project.placements.remove(i);
                        }
                        s.project.placements.insert(0, merged);
                    }
                    self.changed();
                    self.select(vec![id]);
                }
            }
            GridChange::Split { item } => {
                if let Some(i) = self.index_of(&item) {
                    self.adopt_plan();
                    let split = {
                        let mut s = self.state.borrow_mut();
                        match s.project.placements.get(i).cloned() {
                            Some(p) if p.holder.is_none() => {
                                let [w, d] = p.bin.cells;
                                s.project.placements.remove(i);
                                for cx in 0..w {
                                    for cy in 0..d {
                                        let bin = BinSpec { cells: [1, 1], ..p.bin.clone() };
                                        s.project.placements.push(Placement::bin(p.x + cx, p.y + cy, bin));
                                    }
                                }
                                true
                            }
                            _ => false,
                        }
                    };
                    if split {
                        self.changed();
                        self.select(vec![]);
                    }
                }
            }
            GridChange::Selected { items } => self.select(items),
            GridChange::Action { item, action } if action == "silhouette" => {
                if let Some(i) = self.index_of(&item) {
                    return self.silhouette(i);
                }
            }
            GridChange::Action { item, action } => {
                if let Some(i) = self.index_of(&item) {
                    let max = self.with_plan(|p| p.heights.max).unwrap_or(u32::MAX);
                    self.edit(i, |p| match action.as_str() {
                        "taller" => p.bin.units = (p.bin.units + 1).min(max.max(1)),
                        "lower" => p.bin.units = p.bin.units.saturating_sub(1).max(1),
                        "div_x" => p.bin.dividers[0] += 1,
                        "div_y" => p.bin.dividers[1] += 1,
                        "div_none" => p.bin.dividers = [1, 1],
                        "label_tab" => p.bin.label_tab = !p.bin.label_tab,
                        "scoop" => p.bin.scoop = if p.bin.scoop > 0 { 0 } else { 60 },
                        "open_front" => p.bin.open_front = !p.bin.open_front,
                        "fingers" => p.bin.finger_cutouts = !p.bin.finger_cutouts,
                        "window" => p.bin.window = p.bin.window.is_none().then(bingrove_core::model::Window::default),
                        "edges" => {
                            let all_off = p.edges.off.iter().all(|&o| o);
                            p.edges.off = [!all_off; 4];
                        }
                        "cutouts_none" => p.bin.cutouts.clear(),
                        id if id.starts_with("holder_") => set_holder(p, &id[7..]),
                        _ => {}
                    });
                }
            }
        }
        Vec::new()
    }
}

impl GuestPanel for DrawerPanel {
    fn new(_id: String, state: Option<String>) -> DrawerPanel {
        let state = state.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        DrawerPanel { state: RefCell::new(state), plan: RefCell::new(None) }
    }

    fn render(&self) -> String {
        if self.plan.borrow().is_none() {
            self.replan();
        }
        let s = self.state.borrow();
        let plan = self.plan.borrow();
        let current = match &*plan {
            Some(Ok(p)) => Some(p),
            _ => None,
        };
        let mut left = vec![Node::heading("Bingrove"), self.form(&s, current)];
        let mut middle = Vec::new();
        let mut right = Vec::new();
        match &*plan {
            Some(Ok(p)) => {
                let g = &p.grid;
                let zone = |side: Side| match g.zone(side) {
                    Zone::Edge => format!("{:.1} mm Randzone", g.margin(side)),
                    Zone::Solid => format!("{:.1} mm fest", g.margin(side)),
                };
                let text = if p.kind == Kind::Surface {
                    format!(
                        "Fläche {} × {} Zellen ({:.0} × {:.0} mm) · {} Plattenteile · {:.0} % belegt",
                        g.cells[0],
                        g.cells[1],
                        g.plate[0],
                        g.plate[1],
                        p.pieces.len(),
                        p.usable_area()
                    )
                } else {
                    format!(
                        "Raster {} × {} ({:.0} × {:.0} mm) · L/R {} / {} · V/H {} / {} · {} Plattenteile · {:.0} % nutzbar",
                        g.cells[0],
                        g.cells[1],
                        g.cells[0] as f64 * GRID,
                        g.cells[1] as f64 * GRID,
                        zone(Side::Left),
                        zone(Side::Right),
                        zone(Side::Front),
                        zone(Side::Back),
                        p.pieces.len(),
                        p.usable_area()
                    )
                };
                left.push(Node::Label { text, style: TextStyle::Small });
                let language = guest::info().language;
                for w in &p.warnings {
                    left.push(Node::Notice { level: Level::Warn, text: w.text(&language) });
                }
                left.push(Node::row(vec![
                    Node::Button {
                        id: "auto".into(),
                        label: "Automatisch belegen".into(),
                        primary: false,
                        enabled: !s.busy,
                    },
                    Node::Button {
                        id: "generate".into(),
                        label: "Generieren + Druckliste".into(),
                        primary: true,
                        enabled: !s.busy,
                    },
                    Node::Button {
                        id: "calibrate".into(),
                        label: "Kalibrierteile".into(),
                        primary: false,
                        enabled: !s.busy,
                    },
                ]));
                middle.push(self.grid(&s, p));
                match self.selected_form(&s, p) {
                    Some(form) => middle.push(form),
                    None => middle.push(Node::Label {
                        text: "Ein Fach anklicken, um es hier einzustellen; Rechtsklick für das Menü.".into(),
                        style: TextStyle::Small,
                    }),
                }
                let key = s.selected_part.map_or("drawer".to_string(), |i| format!("part:{i}"));
                right.push(Node::MeshView {
                    id: "preview".into(),
                    key,
                    revision: s.revision,
                    height: Some(320.0),
                    bed: s.selected_part.map(|_| [s.project.bed.x as f32, s.project.bed.y as f32]),
                });
                if s.preview_job.is_some() {
                    let (fraction, what) = s.preview_progress.clone().unwrap_or_default();
                    right.push(Node::Progress {
                        fraction: Some(fraction),
                        text: Some(format!("Vorschau wird berechnet … {what}")),
                    });
                } else if s.preview.is_none() {
                    // Without an event at opening nothing could start the job yet.
                    right.push(Node::Button {
                        id: "preview".into(),
                        label: "Vorschau berechnen".into(),
                        primary: false,
                        enabled: true,
                    });
                }
                right.push(Node::Label {
                    text: format!("Teile ({} Stück zu drucken)", p.part_count()),
                    style: meshgrove_plugin_api::ui::TextStyle::Strong,
                });
                right.push(self.parts(&s, p));
            }
            Some(Err(e)) => left.push(Node::Notice { level: Level::Error, text: e.clone() }),
            None => {}
        }
        if let Some((level, text)) = &s.message {
            left.push(Node::Notice { level: *level, text: text.clone() });
        }
        if s.busy {
            left.push(Node::Progress { fraction: None, text: Some("Teile werden berechnet …".into()) });
        }
        // The form needs about 380 px; a wider first share keeps its labels readable.
        let root = Node::Split {
            vertical: false,
            ratio: 0.36,
            first: Box::new(Node::Scroll { max_height: None, children: left }),
            second: Box::new(Node::Split {
                vertical: false,
                ratio: 0.55,
                first: Box::new(Node::Scroll { max_height: None, children: middle }),
                second: Box::new(Node::Scroll { max_height: None, children: right }),
            }),
        };
        guest::json(&View::new(root))
    }

    fn event(&self, event: String) -> String {
        let Ok(event) = serde_json::from_str::<Event>(&event) else { return guest::effects(&[]) };
        let mut effects = match event {
            Event::Changed { id, value } if id.starts_with("mixn:") => {
                self.mix_changed(&id[5..], value.as_f64().unwrap_or(0.0).max(0.0) as u32)
            }
            Event::Click { id } if id.starts_with("mix:") => self.mix_clicked(&id[4..]),
            Event::Changed { id, value } if id == "cells_x" || id == "cells_y" => {
                // A surface is sized in cells; the mm follow.
                let mut s = self.state.borrow_mut();
                let d = &s.project.drawer;
                let mut cells = s
                    .project
                    .options
                    .cells
                    .unwrap_or([(d.width / GRID).floor().max(1.0) as u32, (d.depth / GRID).floor().max(1.0) as u32]);
                cells[usize::from(id == "cells_y")] = (value.as_f64().unwrap_or(1.0).round().max(1.0) as u32).min(35);
                s.project.options.cells = Some(cells);
                s.project.drawer.width = f64::from(cells[0]) * GRID;
                s.project.drawer.depth = f64::from(cells[1]) * GRID;
                drop(s);
                self.changed();
                Vec::new()
            }
            Event::Changed { id, value } if id == "left_tabs" => {
                self.state.borrow_mut().tab = value.as_u64().unwrap_or(0) as usize;
                Vec::new()
            }
            Event::Changed { id, value } if id.starts_with("sel_") => self.selected_changed(&id[4..], value),
            Event::Click { id } if id == "sel_silhouette" => {
                let first = self.state.borrow().selected_items.first().cloned();
                match first.and_then(|id| self.index_of(&id)) {
                    Some(i) => self.silhouette(i),
                    None => Vec::new(),
                }
            }
            Event::Click { id } if id == "sel_cutouts_none" => {
                self.edit_selected(|p| p.bin.cutouts.clear());
                Vec::new()
            }
            Event::Click { id } if id == "sel_row_add" => self.row_count(true),
            Event::Click { id } if id == "sel_row_del" => self.row_count(false),
            Event::Changed { id, value } => {
                let num = value.as_f64();
                let text = value.as_str().map(str::to_owned);
                {
                    let mut s = self.state.borrow_mut();
                    let o = &mut s.project.options;
                    match (id.as_str(), num) {
                        ("width", Some(v)) => s.project.drawer.width = v,
                        ("depth", Some(v)) => s.project.drawer.depth = v,
                        ("height", Some(v)) => s.project.drawer.height = v,
                        ("edge_threshold", Some(v)) => o.edge_threshold = v,
                        ("bins", Some(v)) => s.bins_wanted = v.max(0.0) as u32,
                        ("kind", _) => {
                            o.kind = if text.as_deref() == Some("surface") { Kind::Surface } else { Kind::Drawer };
                            if o.kind == Kind::Drawer {
                                o.cells = None;
                            }
                        }
                        ("cover", _) => {
                            o.cover = if value.as_bool().unwrap_or(false) { Cover::Partial } else { Cover::Exact }
                        }
                        ("magnet_d", Some(v)) => s.project.profile.magnet.d = v,
                        ("magnet_h", Some(v)) => s.project.profile.magnet.h = v,
                        ("model_id", _) => s.model_id = text.unwrap_or_default(),
                        ("model_down", _) => {
                            s.model_down = match text.as_deref() {
                                Some("x") => Down::X,
                                Some("y") => Down::Y,
                                _ => Down::Z,
                            }
                        }
                        ("name", _) => s.project.drawer.name = text.unwrap_or_default(),
                        ("edge_mode", _) => {
                            o.edge_mode = match text.as_deref() {
                                Some("resting") => EdgeMode::Resting,
                                Some("none") => EdgeMode::None,
                                _ => EdgeMode::ToFloor,
                            }
                        }
                        ("align_x" | "align_y", _) => {
                            let a = match text.as_deref() {
                                Some("start") => Align::Start,
                                Some("end") => Align::End,
                                _ => Align::Center,
                            };
                            o.align[usize::from(id == "align_y")] = a;
                        }
                        ("floor", _) => {
                            o.floor = match text.as_deref() {
                                Some("solid") => PlateFloor::Solid,
                                Some("magnets") => PlateFloor::Magnets,
                                _ => PlateFloor::Open,
                            }
                        }
                        ("screws", _) => o.mount.screws = value.as_bool().unwrap_or(false),
                        ("connectors", _) => o.mount.connectors = value.as_bool().unwrap_or(false),
                        ("printer", _) => s.project.bed = crate::bed_of(text.as_deref()),
                        ("lists", _) => {
                            s.lists = match text.as_deref() {
                                Some("parts") => ListMode::Parts,
                                Some("beds") => ListMode::Beds,
                                _ => ListMode::Both,
                            }
                        }
                        _ => {}
                    }
                }
                self.changed();
                Vec::new()
            }
            Event::Click { id } if id == "auto" => {
                // Back to the solver: it fills the drawer afresh from the items (or with bins).
                let mut s = self.state.borrow_mut();
                s.project.placements.clear();
                drop(s);
                self.changed();
                Vec::new()
            }
            Event::Click { id } if id == "calibrate" => {
                let mut s = self.state.borrow_mut();
                s.busy = true;
                let params = json!({"profile": s.project.profile, "name": s.project.drawer.name});
                vec![Effect::StartJobs {
                    jobs: vec![JobSpec {
                        id: "calibration".into(),
                        kind: "calibration".into(),
                        params,
                        label: Some("Kalibrierung".into()),
                    }],
                }]
            }
            Event::Click { id } if id == "generate" => {
                self.adopt_plan();
                let mut s = self.state.borrow_mut();
                s.busy = true;
                let params = json!({"project": s.project, "lists": s.lists});
                vec![Effect::StartJobs {
                    jobs: vec![JobSpec {
                        id: "generate".into(),
                        kind: "generate".into(),
                        params,
                        label: Some("Gridfinity".into()),
                    }],
                }]
            }
            Event::TableSelect { id, row } if id == "parts" => {
                let mut s = self.state.borrow_mut();
                s.selected_part = row.and_then(|r| r.parse().ok());
                s.revision += 1;
                Vec::new()
            }
            Event::Grid { change, .. } => self.grid_change(change),
            Event::JobProgress { job, fraction, message } => {
                if let Some(key) = job.strip_prefix("preview:") {
                    let mut s = self.state.borrow_mut();
                    if s.preview_job.as_deref() == Some(key) {
                        s.preview_progress = Some((fraction, message));
                    }
                }
                Vec::new()
            }
            Event::JobFinished { job, result, error } => match job.strip_prefix("preview:") {
                Some(key) => self.preview_finished(key, error),
                None => self.job_finished(result, error),
            },
            Event::HostChanged { .. } => {
                self.changed();
                Vec::new()
            }
            // Among the rest: the "preview" button, which only needs an event to happen.
            _ => Vec::new(),
        };
        effects.extend(self.preview_effects());
        guest::effects(&effects)
    }

    fn mesh(&self, key: String) -> Option<MeshData> {
        let s = self.state.borrow();
        let preview = s.preview.clone()?;
        match key.strip_prefix("part:") {
            // A part by index only from the preview of the current plan; the whole drawer may
            // stay the earlier one while the job runs.
            Some(i) => {
                let i: usize = i.parse().ok()?;
                let wanted = self.with_plan(|p| crate::preview::key(p, &s.project.profile))?;
                if wanted != preview {
                    return None;
                }
                crate::preview::read(&preview, Some(i))
            }
            None => crate::preview::read(&preview, None),
        }
        .map(Into::into)
    }

    fn snapshot(&self) -> String {
        guest::json(&*self.state.borrow())
    }
}

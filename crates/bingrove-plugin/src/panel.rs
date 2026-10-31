//! The drawer panel: form on the left, the plan as a grid in the middle, parts and a 3D view
//! on the right. Every change re-plans (cheap); geometry only runs for the 3D view and the
//! export job.

use std::cell::RefCell;

use bingrove_core::grid::Zone;
use bingrove_core::model::{Align, BinSpec, DrawerProject, EdgeMode, Placement, PlateFloor, Side};
use bingrove_core::plan::{Plan, plan};
use bingrove_core::spec::GRID;
use meshgrove_plugin_api::guest::{self, GuestPanel, MeshData};
use meshgrove_plugin_api::ui::{
    Action, Choice, Effect, Event, FormRow, GridChange, GridEditor, GridItem, GridZone, JobSpec, Level, Node,
    TableColumn, TableRow, View,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{ListMode, default_project};

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
        }
    }
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
        drop(s);
        self.replan();
    }

    fn form(&self, s: &PanelState) -> Node {
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
        Node::Form {
            id: "drawer".into(),
            rows: vec![
                row("Name", Node::text_input("name", d.name.clone())),
                row("Breite", Node::mm("width", d.width, 42.0, 1500.0)),
                row("Tiefe", Node::mm("depth", d.depth, 42.0, 1500.0)),
                row("Höhe innen", Node::mm("height", d.height, 0.0, 1000.0)),
                row("Randzone ab", Node::mm("edge_threshold", o.edge_threshold, 0.0, 42.0)),
                row(
                    "Randbins",
                    select(
                        "edge_mode",
                        &[("to_floor", "bis zum Boden (ohne Stützen)"), ("resting", "aufliegend (Stützen nötig)")],
                        if o.edge_mode == EdgeMode::Resting { "resting" } else { "to_floor" },
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
                row(
                    "Magnete",
                    Node::Checkbox {
                        id: "magnets".into(),
                        label: "Boden mit Magnetlöchern".into(),
                        checked: o.floor == PlateFloor::Magnets,
                        enabled: true,
                    },
                ),
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
            ],
            submit: None,
        }
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
            zone(Side::Front, 0.0, -m(Side::Front), nx as f32, m(Side::Front)),
            zone(Side::Back, 0.0, ny as f32, nx as f32, m(Side::Back)),
        ];
        let actions = vec![
            Action { id: "taller".into(), label: "Höher (+1 Einheit)".into() },
            Action { id: "lower".into(), label: "Niedriger (−1 Einheit)".into() },
            Action { id: "div_x".into(), label: "Fach teilen (quer)".into() },
            Action { id: "div_y".into(), label: "Fach teilen (längs)".into() },
            Action { id: "div_none".into(), label: "Fächer entfernen".into() },
            Action { id: "label_tab".into(), label: "Label-Lasche an/aus".into() },
            Action { id: "scoop".into(), label: "Griffmulde an/aus".into() },
            Action { id: "edges".into(), label: "Rand nutzen an/aus".into() },
        ];
        let items = p
            .placements
            .iter()
            .enumerate()
            .map(|(i, q)| {
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
                    id: i.to_string(),
                    // The grid editor's y runs down the screen; the drawer's front is at the bottom.
                    x: pl.x as f32,
                    y: (ny - pl.y - d) as f32,
                    w: w as f32,
                    h: d as f32,
                    color: Some(colour(pl.bin.units, p.heights.max.min(24), pl.holder.is_some())),
                    label: pl.label.clone().or_else(|| Some(p.parts[q.part].name.trim_end_matches(".stl").into())),
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

    fn grid_change(&self, change: GridChange) -> Vec<Effect> {
        let ny = self.with_plan(|p| p.grid.cells[1]).unwrap_or(0);
        let index = |item: &str| item.parse::<usize>().ok();
        // Back to drawer coordinates: the editor's y runs down the screen.
        let to_y = |y: f32, h: f32| (ny as f32 - y - h).round().max(0.0) as u32;
        match change {
            GridChange::Moved { item, x, y } => {
                if let Some(i) = index(&item) {
                    self.edit(i, |p| {
                        let d = p.cells()[1];
                        p.x = x.round().max(0.0) as u32;
                        p.y = to_y(y, d as f32);
                    });
                }
            }
            GridChange::Resized { item, x, y, w, h } => {
                if let Some(i) = index(&item) {
                    self.edit(i, |p| {
                        if p.holder.is_none() {
                            p.bin.cells = [w.round().max(1.0) as u32, h.round().max(1.0) as u32];
                        }
                        p.x = x.round().max(0.0) as u32;
                        p.y = to_y(y, p.cells()[1] as f32);
                    });
                }
            }
            GridChange::Created { x, y, w, h } => {
                self.adopt_plan();
                let units = self.with_plan(|p| p.heights.front).unwrap_or(6);
                let cells = [w.round().max(1.0) as u32, h.round().max(1.0) as u32];
                let mut s = self.state.borrow_mut();
                let bin = BinSpec { cells, units, ..BinSpec::default() };
                s.project
                    .placements
                    .insert(0, Placement::bin(x.round().max(0.0) as u32, to_y(y, cells[1] as f32), bin));
                drop(s);
                self.changed();
            }
            GridChange::Deleted { items } => {
                self.adopt_plan();
                let mut gone: Vec<usize> = items.iter().filter_map(|i| index(i)).collect();
                gone.sort_unstable();
                let mut s = self.state.borrow_mut();
                for i in gone.into_iter().rev() {
                    if i < s.project.placements.len() {
                        s.project.placements.remove(i);
                    }
                }
                drop(s);
                self.changed();
            }
            GridChange::Merge { items } => {
                self.adopt_plan();
                let idx: Vec<usize> = items.iter().filter_map(|i| index(i)).collect();
                let mut s = self.state.borrow_mut();
                let chosen: Vec<&Placement> = idx.iter().filter_map(|&i| s.project.placements.get(i)).collect();
                if chosen.len() >= 2 && chosen.iter().all(|p| p.holder.is_none()) {
                    let x0 = chosen.iter().map(|p| p.x).min().unwrap();
                    let y0 = chosen.iter().map(|p| p.y).min().unwrap();
                    let x1 = chosen.iter().map(|p| p.x + p.cells()[0]).max().unwrap();
                    let y1 = chosen.iter().map(|p| p.y + p.cells()[1]).max().unwrap();
                    let merged = Placement::bin(x0, y0, BinSpec { cells: [x1 - x0, y1 - y0], ..chosen[0].bin.clone() });
                    let mut sorted = idx.clone();
                    sorted.sort_unstable();
                    for i in sorted.into_iter().rev() {
                        s.project.placements.remove(i);
                    }
                    s.project.placements.insert(0, merged);
                    drop(s);
                    self.changed();
                }
            }
            GridChange::Split { item } => {
                if let Some(i) = index(&item) {
                    self.adopt_plan();
                    let mut s = self.state.borrow_mut();
                    if let Some(p) = s.project.placements.get(i).cloned()
                        && p.holder.is_none()
                    {
                        let [w, d] = p.bin.cells;
                        s.project.placements.remove(i);
                        for cx in 0..w {
                            for cy in 0..d {
                                let bin = BinSpec { cells: [1, 1], ..p.bin.clone() };
                                s.project.placements.push(Placement::bin(p.x + cx, p.y + cy, bin));
                            }
                        }
                        drop(s);
                        self.changed();
                    }
                }
            }
            GridChange::Selected { items } => {
                let mut s = self.state.borrow_mut();
                s.selected_items = items;
                s.selected_part = None;
            }
            GridChange::Action { item, action } => {
                if let Some(i) = index(&item) {
                    let max = self.with_plan(|p| p.heights.max).unwrap_or(u32::MAX);
                    self.edit(i, |p| match action.as_str() {
                        "taller" => p.bin.units = (p.bin.units + 1).min(max.max(1)),
                        "lower" => p.bin.units = p.bin.units.saturating_sub(1).max(1),
                        "div_x" => p.bin.dividers[0] += 1,
                        "div_y" => p.bin.dividers[1] += 1,
                        "div_none" => p.bin.dividers = [1, 1],
                        "label_tab" => p.bin.label_tab = !p.bin.label_tab,
                        "scoop" => p.bin.scoop = if p.bin.scoop > 0 { 0 } else { 60 },
                        "edges" => {
                            let all_off = p.edges.off.iter().all(|&o| o);
                            p.edges.off = [!all_off; 4];
                        }
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
        let mut left = vec![Node::heading("Bingrove"), self.form(&s)];
        let mut middle = Vec::new();
        let mut right = Vec::new();
        match &*plan {
            Some(Ok(p)) => {
                let g = &p.grid;
                let zone = |side: Side| match g.zone(side) {
                    Zone::Edge => format!("{:.1} mm Randzone", g.margin(side)),
                    Zone::Solid => format!("{:.1} mm fest", g.margin(side)),
                };
                left.push(Node::Label {
                    text: format!(
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
                    ),
                    style: meshgrove_plugin_api::ui::TextStyle::Small,
                });
                for w in &p.warnings {
                    left.push(Node::Notice { level: Level::Warn, text: w.to_string() });
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
                ]));
                middle.push(self.grid(&s, p));
                let key = s.selected_part.map_or("drawer".to_string(), |i| format!("part:{i}"));
                right.push(Node::MeshView {
                    id: "preview".into(),
                    key,
                    revision: s.revision,
                    height: Some(320.0),
                    bed: s.selected_part.map(|_| [s.project.bed.x as f32, s.project.bed.y as f32]),
                });
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
                first: Box::new(Node::column(middle)),
                second: Box::new(Node::Scroll { max_height: None, children: right }),
            }),
        };
        guest::json(&View::new(root))
    }

    fn event(&self, event: String) -> String {
        let Ok(event) = serde_json::from_str::<Event>(&event) else { return guest::effects(&[]) };
        let effects = match event {
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
                        ("name", _) => s.project.drawer.name = text.unwrap_or_default(),
                        ("edge_mode", _) => {
                            o.edge_mode =
                                if text.as_deref() == Some("resting") { EdgeMode::Resting } else { EdgeMode::ToFloor }
                        }
                        ("align_x" | "align_y", _) => {
                            let a = match text.as_deref() {
                                Some("start") => Align::Start,
                                Some("end") => Align::End,
                                _ => Align::Center,
                            };
                            o.align[usize::from(id == "align_y")] = a;
                        }
                        ("magnets", _) => {
                            o.floor =
                                if value.as_bool().unwrap_or(false) { PlateFloor::Magnets } else { PlateFloor::Open }
                        }
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
            Event::JobFinished { result: Some(result), .. } => {
                let mut s = self.state.borrow_mut();
                s.busy = false;
                match serde_json::from_value::<meshgrove_plugin_api::Bundle>(result["bundle"].clone()) {
                    Ok(bundle) => {
                        s.message = Some((Level::Success, bundle.summary.clone().unwrap_or_default()));
                        vec![Effect::ApplyBundle { bundle }]
                    }
                    Err(e) => vec![Effect::Toast { text: e.to_string(), level: Level::Error }],
                }
            }
            Event::JobFinished { error: Some(error), .. } => {
                let mut s = self.state.borrow_mut();
                s.busy = false;
                s.message = Some((Level::Error, error.clone()));
                vec![Effect::Toast { text: error, level: Level::Error }]
            }
            Event::HostChanged { .. } => {
                self.changed();
                Vec::new()
            }
            _ => Vec::new(),
        };
        guest::effects(&effects)
    }

    fn mesh(&self, key: String) -> Option<MeshData> {
        let s = self.state.borrow();
        let profile = s.project.profile.clone();
        drop(s);
        self.with_plan(|p| match key.strip_prefix("part:") {
            Some(i) => i.parse().ok().and_then(|i| crate::preview::part(p, i, &profile)),
            None => crate::preview::drawer(p, &profile),
        })
        .flatten()
        .map(Into::into)
    }

    fn snapshot(&self) -> String {
        guest::json(&*self.state.borrow())
    }
}

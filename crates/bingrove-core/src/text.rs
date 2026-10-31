//! A plan as text, for AI answers and terminals: the drawer from above, front at the bottom.

use std::fmt::Write;

use crate::grid::Zone;
use crate::model::Side;
use crate::plan::Plan;

fn letter(i: usize) -> String {
    let abc = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    if i < 26 { (abc[i] as char).to_string() } else { format!("{}{}", abc[i / 26 - 1] as char, abc[i % 26] as char) }
}

pub fn ascii_plan(plan: &Plan) -> String {
    let g = &plan.grid;
    let [nx, ny] = g.cells;
    let mut s = String::new();
    let zone = |side: Side| match g.zone(side) {
        Zone::Edge => format!("{:.1} mm edge zone", g.margin(side)),
        Zone::Solid => format!("{:.1} mm solid", g.margin(side)),
    };
    let _ = writeln!(
        s,
        "{}: grid {nx} x {ny} ({:.0} x {:.0} mm) on a {:.1} x {:.1} mm plate, {:.0}% of the plate usable",
        plan.name,
        nx as f64 * crate::spec::GRID,
        ny as f64 * crate::spec::GRID,
        g.plate[0],
        g.plate[1],
        plan.usable_area()
    );
    let _ = writeln!(
        s,
        "margins: left {}, right {}, front {}, back {}",
        zone(Side::Left),
        zone(Side::Right),
        zone(Side::Front),
        zone(Side::Back)
    );
    let mut cell = vec![vec![String::new(); nx as usize]; ny as usize];
    for (i, p) in plan.placements.iter().enumerate() {
        let [w, d] = p.placement.cells();
        for y in p.placement.y..p.placement.y + d {
            for x in p.placement.x..p.placement.x + w {
                cell[y as usize][x as usize] = letter(i);
            }
        }
    }
    let edge = |side: Side| if g.zone(side) == Zone::Edge { '~' } else { '#' };
    let width = 3 * nx as usize;
    let _ = writeln!(s, "back");
    let _ = writeln!(s, " {}", edge(Side::Back).to_string().repeat(width + 2));
    for row in cell.iter().rev() {
        let line: String = row.iter().map(|c| format!("{c:^3}")).collect();
        let _ = writeln!(s, " {}{line}{}", edge(Side::Left), edge(Side::Right));
    }
    let _ = writeln!(s, " {}", edge(Side::Front).to_string().repeat(width + 2));
    let _ = writeln!(s, "front   (~ edge zone that bins reach into, # solid plate margin)");
    for (i, p) in plan.placements.iter().enumerate() {
        let part = &plan.parts[p.part];
        let label = p.placement.label.as_deref().map(|l| format!(" – {l}")).unwrap_or_default();
        let _ = writeln!(s, "{:>2}: {}{}{label}", letter(i), part.name, if p.turned { " (turned)" } else { "" });
    }
    let _ = writeln!(s, "parts ({} pieces to print):", plan.part_count());
    for b in &plan.parts {
        let _ =
            writeln!(s, "  {:2} x {} ({:.1} x {:.1} x {:.1} mm)", b.quantity, b.name, b.size[0], b.size[1], b.size[2]);
    }
    for w in &plan.warnings {
        let _ = writeln!(s, "warning: {w}");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{DrawerProject, DrawerSpec};

    #[test]
    fn plan_text_shows_zones_and_parts() {
        let project = DrawerProject {
            drawer: DrawerSpec { name: "Kleinkram".into(), width: 250.0, depth: 305.0, height: 100.0 },
            placements: crate::presets::Preset::get("250x305").unwrap().placements,
            ..DrawerProject::default()
        };
        let text = ascii_plan(&crate::plan::plan(&project).unwrap());
        assert!(text.contains("left 19.5 mm edge zone"), "{text}");
        assert!(text.contains("randbin_1x5_6u_L19.25.stl"), "{text}");
        // Back row first: the two tall 2x2 bins and the 1x2 on the right.
        let back_row = text.lines().find(|l| l.starts_with(" ~") && l.contains('A')).unwrap();
        assert_eq!(back_row.trim(), "~ A  A  B  B  C ~");
    }
}

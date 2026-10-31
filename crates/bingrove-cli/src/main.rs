//! `bingrove`: drawer presets, single bins and tape holders to STL, like the original Python script.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use bingrove_core::export::{ExportOptions, save_stl};
use bingrove_core::geom::{Part, baseplate, bin, tape};
use bingrove_core::layout::{bill_of_bins, validate};
use bingrove_core::model::{
    Align, BedSpec, BinSpec, DrawerProject, DrawerSpec, EdgeMode, ItemKind, ItemRequest, MagnetMode, PlateFloor,
    PlateOptions, TapeSpec,
};
use bingrove_core::presets::Preset;
use bingrove_core::profile::Profile;
use bingrove_core::spec::{LIP_H, UNIT};
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "bingrove", version, about = "Gridfinity-Einsätze für Schubladen als STL")]
struct Cli {
    /// Schublade (Preset), deren Maße und Bin-Höhe gelten
    #[arg(long, default_value = "435x330", global = true)]
    schublade: String,
    /// Magnetlöcher: Baseplate immer unter jedem Feld, Bins in den äußeren Ecken oder unter jedem Feld
    #[arg(long, value_enum, global = true)]
    magnete: Option<Magnete>,
    /// Durchmesser und Tiefe der Magnetlöcher (Standard 4.2 1.4; Magnete 6x2 mm: 6.5 2.4)
    #[arg(long, num_args = 2, value_names = ["D", "T"], global = true)]
    magnetloch: Option<Vec<f64>>,
    /// Ausgabeordner (Standard: out/<schublade>)
    #[arg(long, global = true)]
    out: Option<PathBuf>,
    /// Druckbett in mm (Teile, die nicht passen, werden gemeldet)
    #[arg(long, default_value_t = 250.0, global = true)]
    bett: f64,
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Clone, Copy, ValueEnum)]
enum Magnete {
    Ecken,
    Alle,
}

#[derive(Subcommand)]
enum Cmd {
    /// Alle Schubladen-Presets auflisten
    Presets,
    /// Eine beliebige Schublade planen und komplett ausgeben (Platte, Bins, Randbins, Druckbetten)
    Drawer {
        /// Innenmaß Breite x Tiefe [x Höhe] in mm, z. B. 250x305x100
        masse: String,
        /// Name der Schublade (Ordner- und Dateinamen)
        #[arg(long, default_value = "Schublade")]
        name: String,
        /// Inhalt als "NAME:BxTxH" (Gegenstand), "NAME:bin AxB[xU]" (Bin) oder "NAME:tape OD[xN]"; mehrfach
        #[arg(long = "inhalt", value_name = "ITEM")]
        items: Vec<String>,
        /// Ab dieser Randbreite reichen Bins in den Rand (mm)
        #[arg(long, default_value_t = 10.0)]
        schwelle: f64,
        /// Randbins liegen auf dem Plattenrand statt bis zum Boden zu reichen
        #[arg(long)]
        aufliegend: bool,
        /// Raster an eine Wand schieben statt mittig: l, r, v, h (links/rechts/vorne/hinten)
        #[arg(long, value_delimiter = ',')]
        buendig: Vec<String>,
        /// Nur planen und den Plan ausgeben
        #[arg(long)]
        plan: bool,
        /// Zusätzlich den Zusammenbau als 3MF schreiben
        #[arg(long)]
        zusammenbau: bool,
    },
    /// Nur die Baseplate-Teile der Schublade
    Baseplate,
    /// Ein einzelner Bin
    Bin {
        gx: u32,
        gy: u32,
        /// Höheneinheiten à 7 mm (Standard je Schublade)
        #[arg(short, long)]
        units: Option<u32>,
        /// Fächer in X und Y
        #[arg(short, long, num_args = 2, value_names = ["DX", "DY"], default_values_t = [1, 1])]
        div: Vec<u32>,
        /// Ohne Stapelrand
        #[arg(long)]
        ohne_rand: bool,
    },
    /// Tape-Halter: Stapel von Rollen auf Ringdornen
    Tape {
        /// Außendurchmesser der größten Rolle (mm)
        od: f64,
        /// Stapelplätze in X und Y
        #[arg(short, num_args = 2, value_names = ["NX", "NY"], default_values_t = [1, 1])]
        n: Vec<u32>,
        /// Rollen übereinander
        #[arg(short, long, default_value_t = 2)]
        stack: u32,
        /// Kern-Innendurchmesser
        #[arg(long, default_value_t = bingrove_core::spec::TAPE_CORE)]
        core: f64,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Fehler: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<(), String> {
    if let Some(Cmd::Presets) = cli.cmd {
        for p in Preset::all() {
            println!("{:10} {}", p.name, p.description);
        }
        return Ok(());
    }
    if let Some(Cmd::Drawer { .. }) = &cli.cmd {
        return drawer(cli);
    }
    let preset = Preset::get(&cli.schublade)
        .ok_or_else(|| format!("unbekannte Schublade {}, siehe `bingrove presets`", cli.schublade))?;
    let mut profile = Profile::default();
    if let Some(v) = &cli.magnetloch {
        profile.magnet.d = v[0];
        profile.magnet.h = v[1];
    }
    let magnets = match cli.magnete {
        None => MagnetMode::None,
        Some(Magnete::Ecken) => MagnetMode::Corners,
        Some(Magnete::Alle) => MagnetMode::All,
    };
    let out = cli.out.clone().unwrap_or_else(|| Path::new("out").join(&preset.name));
    let w = Writer { out, bed: cli.bett, started: Instant::now() };

    match cli.cmd {
        None | Some(Cmd::Baseplate) => {
            let mut plate = preset.plate.clone();
            plate.magnets = magnets != MagnetMode::None;
            let [nx, ny] = plate.cells();
            let [pw, pd] = plate.size();
            let [mx, my] = plate.margins();
            println!(
                "Baseplate {nx} x {ny} Felder, Platte {pw:.1} x {pd:.1} x {:.2} mm, Rand {mx:.2} / {my:.2} mm",
                plate.height(&profile.magnet)
            );
            for part in baseplate::pieces(&plate, &profile) {
                w.save(&part)?;
            }
            if cli.cmd.is_some() {
                return Ok(());
            }
            if !preset.placements.is_empty() {
                validate(nx, ny, &preset.placements).map_err(|e| e.to_string())?;
                println!("Bins nach Belegung:");
                let bom = bill_of_bins(&preset.placements);
                let mut names = Vec::new();
                for (b, n) in bom {
                    let part = bin::bin(&BinSpec { magnets, ..b }, &profile);
                    w.save(&part)?;
                    names.push((n, part.name));
                }
                println!("Stückliste:");
                for (n, name) in names {
                    println!("  {n:2} x {name}");
                }
            } else {
                println!("Bins ({} Einheiten, {:.1} mm):", preset.units, preset.units as f64 * UNIT + LIP_H);
                for b in &preset.bins {
                    w.save(&bin::bin(&BinSpec { magnets, ..b.clone() }, &profile))?;
                }
                println!("Tape-Halter:");
                for t in &preset.tape {
                    w.save(&tape::tape(t, &profile))?;
                    println!("    Schublade innen mind. {:.0} mm hoch", t.drawer_height());
                }
            }
        }
        Some(Cmd::Bin { gx, gy, units, div, ohne_rand }) => {
            let b = BinSpec {
                cells: [gx, gy],
                units: units.unwrap_or(preset.units),
                dividers: [div[0], div[1]],
                lip: !ohne_rand,
                magnets,
                ..BinSpec::default()
            };
            w.save(&bin::bin(&b, &profile))?;
        }
        Some(Cmd::Tape { od, n, stack, core }) => {
            let t = TapeSpec { od, stacks: [n[0], n[1]], height: stack, core };
            w.save(&tape::tape(&t, &profile))?;
            println!("    Schublade innen mind. {:.0} mm hoch", t.drawer_height());
        }
        Some(Cmd::Presets | Cmd::Drawer { .. }) => unreachable!(),
    }
    println!("Fertig in {:.1} s → {}", w.started.elapsed().as_secs_f64(), w.out.display());
    Ok(())
}

struct Writer {
    out: PathBuf,
    bed: f64,
    started: Instant,
}

impl Writer {
    fn save(&self, part: &Part) -> Result<(), String> {
        let mesh = part.mesh().map_err(|e| e.to_string())?;
        let (lo, hi) = mesh.bbox();
        let size = [0, 1, 2].map(|k| hi[k] - lo[k]);
        save_stl(&self.out, &part.name, &mesh).map_err(|e| format!("{}: {e}", part.name))?;
        println!(
            "  {:38} {:.1} x {:.1} x {:.1} mm  {:6.1} cm³",
            part.name,
            size[0],
            size[1],
            size[2],
            mesh.volume() / 1000.0
        );
        if size[0].max(size[1]) as f64 > self.bed {
            println!("  WARNUNG: {} ist größer als das Druckbett ({:.0} mm)", part.name, self.bed);
        }
        Ok(())
    }
}

fn parse_mm(s: &str) -> Result<Vec<f64>, String> {
    s.split(['x', 'X', '×'])
        .map(|p| p.trim().replace(',', ".").parse::<f64>().map_err(|_| format!("keine Zahl: {p}")))
        .collect()
}

/// "Stifte:20x180x20", "Kabel:bin 2x2x12", "Tape:tape 103x2", optionally "3*NAME:..." for a quantity.
fn parse_item(s: &str) -> Result<ItemRequest, String> {
    let (quantity, s) = match s.split_once('*') {
        Some((n, rest)) if n.trim().parse::<u32>().is_ok() => (n.trim().parse().unwrap(), rest),
        _ => (1, s),
    };
    let (name, spec) = s.split_once(':').ok_or_else(|| format!("{s}: NAME:MASSE erwartet"))?;
    let spec = spec.trim();
    let what = if let Some(rest) = spec.strip_prefix("bin ") {
        let v = parse_mm(rest)?;
        if v.len() < 2 {
            return Err(format!("{s}: bin AxB[xU]"));
        }
        ItemKind::Bin { cells: [v[0] as u32, v[1] as u32], units: v.get(2).map(|u| *u as u32), dividers: [1, 1] }
    } else if let Some(rest) = spec.strip_prefix("tape ") {
        let v = parse_mm(rest)?;
        let n = v.get(1).map_or(1, |n| *n as u32);
        ItemKind::Tape(TapeSpec { od: v[0], stacks: [n, 1], ..TapeSpec::default() })
    } else {
        let v = parse_mm(spec)?;
        if v.len() != 3 {
            return Err(format!("{s}: BxTxH erwartet"));
        }
        ItemKind::Object { size: [v[0], v[1], v[2]], clearance: 2.0 }
    };
    Ok(ItemRequest { name: name.trim().to_string(), quantity, what })
}

fn drawer(cli: Cli) -> Result<(), String> {
    let Some(Cmd::Drawer { masse, name, items, schwelle, aufliegend, buendig, plan: only_plan, zusammenbau }) = cli.cmd
    else {
        unreachable!()
    };
    let v = parse_mm(&masse)?;
    if v.len() < 2 {
        return Err("Maße als BREITExTIEFE[xHÖHE] angeben".into());
    }
    let mut align = [Align::Center; 2];
    for b in &buendig {
        match b.as_str() {
            "l" => align[0] = Align::Start,
            "r" => align[0] = Align::End,
            "v" => align[1] = Align::Start,
            "h" => align[1] = Align::End,
            other => return Err(format!("--buendig: {other} (erlaubt: l, r, v, h)")),
        }
    }
    let mut profile = Profile::default();
    if let Some(m) = &cli.magnetloch {
        profile.magnet.d = m[0];
        profile.magnet.h = m[1];
    }
    let project = DrawerProject {
        drawer: DrawerSpec { name: name.clone(), width: v[0], depth: v[1], height: v.get(2).copied().unwrap_or(0.0) },
        options: PlateOptions {
            align,
            edge_threshold: schwelle,
            edge_mode: if aufliegend { EdgeMode::Resting } else { EdgeMode::ToFloor },
            floor: if cli.magnete.is_some() { PlateFloor::Magnets } else { PlateFloor::Open },
            ..PlateOptions::default()
        },
        items: items.iter().map(|s| parse_item(s)).collect::<Result<_, _>>()?,
        profile,
        bed: BedSpec { x: cli.bett, y: cli.bett, ..BedSpec::default() },
        ..DrawerProject::default()
    };
    let plan = bingrove_core::plan::plan(&project).map_err(|e| e.to_string())?;
    print!("{}", bingrove_core::text::ascii_plan(&plan));
    if only_plan {
        return Ok(());
    }
    let out = cli.out.unwrap_or_else(|| Path::new("out").join(bingrove_core::export::slug(&name)));
    let started = Instant::now();
    let opts = ExportOptions { assembly: zusammenbau, ..ExportOptions::default() };
    let report = bingrove_core::export::export(&plan, &project, &out, opts, &|done, total, what| {
        eprintln!("  [{done:2}/{total}] {what}");
    })
    .map_err(|e| e.to_string())?;
    println!(
        "{} Dateien, {} Druckbetten, fertig in {:.1} s → {}",
        report.files.len(),
        report.beds,
        started.elapsed().as_secs_f64(),
        out.display()
    );
    Ok(())
}

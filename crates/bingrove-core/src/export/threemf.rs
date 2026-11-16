//! 3MF: one object per distinct mesh, one build item per copy, each with its own transform. A
//! part with an inlay (the body filling its engraving) becomes one object of two parts: a
//! `<components>` object over both meshes, named and the inlay put on the second filament in
//! `Metadata/model_settings.config`, which Orca, Bambu Studio and Anycubic Slicer Next read.

use std::fmt::Write as _;
use std::io::Write as _;

use crate::mesh::TriMesh;

/// A copy of an object: turned by `quarter_turns` × 90° about z, then moved by `at`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Item {
    pub object: usize,
    pub quarter_turns: u8,
    pub at: [f64; 3],
}

impl Item {
    /// The 3MF transform (row vector times a 3×4 matrix, written row by row).
    fn transform(&self) -> String {
        let (c, s) = match self.quarter_turns % 4 {
            0 => (1.0, 0.0),
            1 => (0.0, 1.0),
            2 => (-1.0, 0.0),
            _ => (0.0, -1.0),
        };
        let [x, y, z] = self.at;
        format!("{c} {s} 0 {} {c} 0 0 0 1 {x:.4} {y:.4} {z:.4}", -s)
    }
}

pub struct Object<'a> {
    pub name: &'a str,
    pub mesh: &'a TriMesh,
    /// The body that fills the part's engraving, to print in a second colour.
    pub inlay: Option<&'a TriMesh>,
}

/// What a part and its inlay are called in the slicer.
pub const BODY_PART: &str = "Körper";
pub const INLAY_PART: &str = "Schrift";

fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn model_xml(title: &str, objects: &[Object], items: &[Item]) -> String {
    let mut x =
        String::with_capacity(objects.iter().map(|o| o.mesh.positions.len() * 60 + o.mesh.indices.len() * 15).sum());
    x.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    x.push_str("<model unit=\"millimeter\" xml:lang=\"en-US\" xmlns=\"http://schemas.microsoft.com/3dmanufacturing/core/2015/02\">\n");
    let _ = writeln!(x, " <metadata name=\"Title\">{}</metadata>", escape(title));
    x.push_str(" <metadata name=\"Application\">Bingrove</metadata>\n <resources>\n");
    // Object i is id i + 1 (what build items name); the two meshes of an object with an inlay
    // follow after all of them.
    // Only what lies on this bed; ids stay those of the whole list.
    let used = |i: usize| items.iter().any(|it| it.object == i);
    let mut next = objects.len() + 1;
    for (i, o) in objects.iter().enumerate().filter(|(i, _)| used(*i)) {
        match o.inlay {
            None => mesh_object(&mut x, i + 1, o.name, o.mesh),
            Some(inlay) => {
                let (body, fill) = (next, next + 1);
                next += 2;
                mesh_object(&mut x, body, &format!("{} – {BODY_PART}", o.name), o.mesh);
                mesh_object(&mut x, fill, &format!("{} – {INLAY_PART}", o.name), inlay);
                let _ = writeln!(
                    x,
                    "  <object id=\"{}\" type=\"model\" name=\"{}\">\n   <components>\n    \
                     <component objectid=\"{body}\"/>\n    <component objectid=\"{fill}\"/>\n   </components>\n  </object>",
                    i + 1,
                    escape(o.name)
                );
            }
        }
    }
    x.push_str(" </resources>\n <build>\n");
    for it in items {
        let _ = writeln!(x, "  <item objectid=\"{}\" transform=\"{}\"/>", it.object + 1, it.transform());
    }
    x.push_str(" </build>\n</model>\n");
    x
}

const CONTENT_TYPES: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
 <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
 <Default Extension="model" ContentType="application/vnd.ms-package.3dmanufacturing-3dmodel+xml"/>
</Types>
"#;

const RELS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
 <Relationship Target="/3D/3dmodel.model" Id="rel0" Type="http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel"/>
</Relationships>
"#;

fn mesh_object(x: &mut String, id: usize, name: &str, mesh: &TriMesh) {
    let _ = writeln!(x, "  <object id=\"{id}\" type=\"model\" name=\"{}\">\n   <mesh>\n    <vertices>", escape(name));
    for p in &mesh.positions {
        let _ = writeln!(x, "     <vertex x=\"{}\" y=\"{}\" z=\"{}\"/>", p[0], p[1], p[2]);
    }
    x.push_str("    </vertices>\n    <triangles>\n");
    for t in mesh.indices.as_chunks::<3>().0.iter() {
        let _ = writeln!(x, "     <triangle v1=\"{}\" v2=\"{}\" v3=\"{}\"/>", t[0], t[1], t[2]);
    }
    x.push_str("    </triangles>\n   </mesh>\n  </object>\n");
}

/// The parts of the objects with an inlay, for Orca-like slicers: object ids are the build
/// items' objects, part ids the component objects, extruders count from 1. `None` without any.
fn model_settings(objects: &[Object], items: &[Item]) -> Option<String> {
    let used = |i: usize| items.iter().any(|it| it.object == i);
    let mut x = String::new();
    let mut next = objects.len() + 1;
    for (i, o) in objects.iter().enumerate().filter(|(i, o)| used(*i) && o.inlay.is_some()) {
        let (body, fill) = (next, next + 1);
        next += 2;
        let _ = writeln!(x, " <object id=\"{}\">\n  <metadata key=\"name\" value=\"{}\"/>", i + 1, escape(o.name));
        x.push_str("  <metadata key=\"extruder\" value=\"1\"/>\n");
        for (id, name, extruder) in [(body, BODY_PART, 1), (fill, INLAY_PART, 2)] {
            let _ = writeln!(
                x,
                "  <part id=\"{id}\" subtype=\"normal_part\">\n   <metadata key=\"name\" value=\"{name}\"/>\n   \
                 <metadata key=\"extruder\" value=\"{extruder}\"/>\n  </part>"
            );
        }
        x.push_str(" </object>\n");
    }
    (!x.is_empty()).then(|| format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<config>\n{x}</config>\n"))
}

/// The bytes of a 3MF package.
pub fn write(title: &str, objects: &[Object], items: &[Item]) -> std::io::Result<Vec<u8>> {
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let mut entries = vec![
        ("[Content_Types].xml", CONTENT_TYPES.to_string()),
        ("_rels/.rels", RELS.to_string()),
        ("3D/3dmodel.model", model_xml(title, objects, items)),
    ];
    if let Some(settings) = model_settings(objects, items) {
        entries.push(("Metadata/model_settings.config", settings));
    }
    for (name, body) in entries {
        zip.start_file(name, opts).map_err(std::io::Error::other)?;
        zip.write_all(body.as_bytes())?;
    }
    Ok(zip.finish().map_err(std::io::Error::other)?.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transforms_turn_about_z() {
        let it = |q| Item { object: 0, quarter_turns: q, at: [1.0, 2.0, 0.0] };
        assert_eq!(it(0).transform(), "1 0 0 -0 1 0 0 0 1 1.0000 2.0000 0.0000");
        // A quarter turn maps (1, 0) to (0, 1).
        assert_eq!(it(1).transform(), "0 1 0 -1 0 0 0 0 1 1.0000 2.0000 0.0000");
        assert_eq!(it(2).transform(), "-1 0 0 -0 -1 0 0 0 1 1.0000 2.0000 0.0000");
    }

    #[test]
    fn package_is_a_zip_with_the_model() {
        let mesh = TriMesh {
            positions: vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            indices: vec![0, 2, 1, 0, 1, 3, 0, 3, 2, 1, 2, 3],
        };
        let bytes = write(
            "Test & Co",
            &[Object { name: "tetra", mesh: &mesh, inlay: None }],
            &[Item { object: 0, quarter_turns: 0, at: [0.0; 3] }],
        )
        .unwrap();
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        let mut model = String::new();
        std::io::Read::read_to_string(&mut zip.by_name("3D/3dmodel.model").unwrap(), &mut model).unwrap();
        assert!(model.contains("Test &amp; Co") && model.matches("<triangle ").count() == 4);
        assert!(zip.by_name("Metadata/model_settings.config").is_err(), "no settings without an inlay");
    }

    #[test]
    fn a_part_with_an_inlay_is_one_object_of_two_parts() {
        let tetra = |dx: f32| TriMesh {
            positions: vec![[dx, 0.0, 0.0], [dx + 1.0, 0.0, 0.0], [dx, 1.0, 0.0], [dx, 0.0, 1.0]],
            indices: vec![0, 2, 1, 0, 1, 3, 0, 3, 2, 1, 2, 3],
        };
        let (plain, lid, text) = (tetra(0.0), tetra(5.0), tetra(6.0));
        let objects =
            [Object { name: "bin", mesh: &plain, inlay: None }, Object { name: "lid", mesh: &lid, inlay: Some(&text) }];
        let items =
            [Item { object: 0, quarter_turns: 0, at: [0.0; 3] }, Item { object: 1, quarter_turns: 0, at: [0.0; 3] }];
        let bytes = write("Bed", &objects, &items).unwrap();
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        let read = |zip: &mut zip::ZipArchive<_>, name: &str| {
            let mut s = String::new();
            std::io::Read::read_to_string(&mut zip.by_name(name).unwrap(), &mut s).unwrap();
            s
        };
        let model = read(&mut zip, "3D/3dmodel.model");
        // The lid (object 2) is a components object over meshes 3 and 4; items name 1 and 2.
        assert!(model.contains("<object id=\"2\" type=\"model\" name=\"lid\">\n   <components>"), "{model}");
        assert!(model.contains("<component objectid=\"3\"/>") && model.contains("<component objectid=\"4\"/>"));
        assert!(model.contains("<item objectid=\"1\"") && model.contains("<item objectid=\"2\""));
        assert_eq!(model.matches("<mesh>").count(), 3);
        let settings = read(&mut zip, "Metadata/model_settings.config");
        assert!(settings.contains("<object id=\"2\">") && !settings.contains("<object id=\"1\">"), "{settings}");
        assert!(settings.contains("<part id=\"4\" subtype=\"normal_part\">\n   <metadata key=\"name\" value=\"Schrift\"/>\n   <metadata key=\"extruder\" value=\"2\"/>"), "{settings}");
    }
}

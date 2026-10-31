//! 3MF: one object per distinct mesh, one build item per copy, each with its own transform.

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
}

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
    for (i, o) in objects.iter().enumerate() {
        let _ = writeln!(
            x,
            "  <object id=\"{}\" type=\"model\" name=\"{}\">\n   <mesh>\n    <vertices>",
            i + 1,
            escape(o.name)
        );
        for p in &o.mesh.positions {
            let _ = writeln!(x, "     <vertex x=\"{}\" y=\"{}\" z=\"{}\"/>", p[0], p[1], p[2]);
        }
        x.push_str("    </vertices>\n    <triangles>\n");
        for t in o.mesh.indices.as_chunks::<3>().0.iter() {
            let _ = writeln!(x, "     <triangle v1=\"{}\" v2=\"{}\" v3=\"{}\"/>", t[0], t[1], t[2]);
        }
        x.push_str("    </triangles>\n   </mesh>\n  </object>\n");
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

/// The bytes of a 3MF package.
pub fn write(title: &str, objects: &[Object], items: &[Item]) -> std::io::Result<Vec<u8>> {
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for (name, body) in [
        ("[Content_Types].xml", CONTENT_TYPES.to_string()),
        ("_rels/.rels", RELS.to_string()),
        ("3D/3dmodel.model", model_xml(title, objects, items)),
    ] {
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
            &[Object { name: "tetra", mesh: &mesh }],
            &[Item { object: 0, quarter_turns: 0, at: [0.0; 3] }],
        )
        .unwrap();
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        let mut model = String::new();
        std::io::Read::read_to_string(&mut zip.by_name("3D/3dmodel.model").unwrap(), &mut model).unwrap();
        assert!(model.contains("Test &amp; Co") && model.matches("<triangle ").count() == 4);
    }
}

//! Indexed triangle meshes, as handed to exporters, previews and the plugin host.

use crate::geom::kernel::Solid;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TriMesh {
    pub positions: Vec<[f32; 3]>,
    /// Three indices into `positions` per triangle, counter-clockwise seen from outside.
    pub indices: Vec<u32>,
}

impl TriMesh {
    pub fn from_solid(s: &Solid) -> Self {
        let gl = s.get_mesh_gl(-1);
        let stride = (gl.num_prop as usize).max(3);
        let positions: Vec<[f32; 3]> = gl.vert_properties.chunks_exact(stride).map(|v| [v[0], v[1], v[2]]).collect();
        // Vertices split for properties are merged back, so shared edges share indices.
        let mut to = (0..positions.len() as u32).collect::<Vec<_>>();
        for (&from, &into) in gl.merge_from_vert.iter().zip(&gl.merge_to_vert) {
            to[from as usize] = into;
        }
        let indices = gl.tri_verts.iter().map(|&i| to[i as usize]).collect();
        TriMesh { positions, indices }
    }

    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    pub fn triangles(&self) -> impl Iterator<Item = [[f32; 3]; 3]> + '_ {
        self.indices.as_chunks::<3>().0.iter().map(|t| [0, 1, 2].map(|k| self.positions[t[k] as usize]))
    }

    /// Minimum and maximum corner.
    pub fn bbox(&self) -> ([f32; 3], [f32; 3]) {
        let mut lo = [f32::INFINITY; 3];
        let mut hi = [f32::NEG_INFINITY; 3];
        for p in &self.positions {
            for k in 0..3 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
        (lo, hi)
    }

    /// Enclosed volume (signed tetrahedra against the origin).
    pub fn volume(&self) -> f64 {
        self.triangles()
            .map(|[a, b, c]| {
                let [a, b, c] = [a, b, c].map(|v| v.map(f64::from));
                (a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
                    + a[2] * (b[0] * c[1] - b[1] * c[0]))
                    / 6.0
            })
            .sum()
    }

    /// Watertight: every directed edge is matched by exactly one opposite edge.
    pub fn is_closed(&self) -> bool {
        use std::collections::HashMap;
        let mut edges: HashMap<(u32, u32), i32> = HashMap::with_capacity(self.indices.len());
        for t in self.indices.as_chunks::<3>().0 {
            for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                *edges.entry((a, b)).or_default() += 1;
            }
        }
        !edges.is_empty() && edges.iter().all(|(&(a, b), &n)| n == 1 && edges.get(&(b, a)) == Some(&1))
    }

    /// Turned by 180° about z within its own footprint `size`, so the minimum corner stays put.
    pub fn turned_180(mut self, size: [f32; 2]) -> Self {
        for p in &mut self.positions {
            p[0] = size[0] - p[0];
            p[1] = size[1] - p[1];
        }
        self
    }

    /// Adds another mesh's triangles.
    pub fn append(&mut self, other: &TriMesh) {
        let base = self.positions.len() as u32;
        self.positions.extend_from_slice(&other.positions);
        self.indices.extend(other.indices.iter().map(|i| i + base));
    }

    /// Stood up on a wall: y (up the wall) becomes z, z (out of the wall) becomes −y.
    pub fn stood_up(mut self) -> Self {
        for p in &mut self.positions {
            *p = [p[0], -p[2], p[1]];
        }
        self
    }

    /// The same mesh moved by `v`.
    pub fn translated(mut self, v: [f32; 3]) -> Self {
        for p in &mut self.positions {
            for k in 0..3 {
                p[k] += v[k];
            }
        }
        self
    }
}

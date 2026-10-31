//! Binary STL.

use crate::mesh::TriMesh;

const HEADER_LEN: usize = 80;
const RECORD_LEN: usize = 50;

pub fn write_binary(mesh: &TriMesh, header: &str) -> Vec<u8> {
    let n = mesh.triangle_count();
    let mut out = Vec::with_capacity(HEADER_LEN + 4 + n * RECORD_LEN);
    let mut head = [b' '; HEADER_LEN];
    let h = header.as_bytes();
    head[..h.len().min(HEADER_LEN)].copy_from_slice(&h[..h.len().min(HEADER_LEN)]);
    out.extend_from_slice(&head);
    out.extend_from_slice(&(n as u32).to_le_bytes());
    for [a, b, c] in mesh.triangles() {
        let (u, v) = ([b[0] - a[0], b[1] - a[1], b[2] - a[2]], [c[0] - a[0], c[1] - a[1], c[2] - a[2]]);
        let mut normal = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
        let len = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
        if len > 1e-12 {
            normal = normal.map(|x| x / len);
        }
        for p in [normal, a, b, c] {
            for f in p {
                out.extend_from_slice(&f.to_le_bytes());
            }
        }
        out.extend_from_slice(&[0, 0]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_layout() {
        let mesh =
            TriMesh { positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]], indices: vec![0, 1, 2] };
        let bytes = write_binary(&mesh, "bingrove");
        assert_eq!(bytes.len(), 84 + 50);
        assert_eq!(&bytes[..8], b"bingrove");
        assert_eq!(u32::from_le_bytes(bytes[80..84].try_into().unwrap()), 1);
        // Normal points up.
        assert_eq!(f32::from_le_bytes(bytes[92..96].try_into().unwrap()), 1.0);
    }
}

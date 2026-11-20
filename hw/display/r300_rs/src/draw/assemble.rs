//! Primitive assembly (`r300_assemble_prov`).

/// Per output primitive: the source primitive's FIRST, SECOND, THIRD and LAST
/// vertex (flat-shading candidates) and its triangle edge flags (bit 0: edge
/// 0-1, bit 1: 1-2, bit 2: 2-0; the diagonals the card adds inside quads and
/// polygons are not edges).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Prov {
    pub v: [u32; 4],
    pub edges: u32,
}

/// What a primitive list expands to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    Triangles = 0,
    Lines = 1,
    Points = 2,
}

/// A primitive list (`VAP_VF_CNTL.PRIM_TYPE`) expanded to triangle, line or
/// point list indices, with the flat-shading candidates (per
/// `GA_COLOR_CONTROL.PROVOKING_VERTEX`) of each output primitive.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Assembly {
    pub class: Class,
    pub indices: Vec<u32>,
    pub prov: Vec<Prov>,
}

/// Assembles `n` vertices of primitive type `prim`; `None` for a type that is
/// not supported.
pub fn assemble(prim: u32, n: u32) -> Option<Assembly> {
    let mut a = Assembly { class: Class::Triangles, indices: Vec::new(), prov: Vec::new() };
    let put = |a: &mut Assembly, ix: &[u32], p: Prov| {
        a.indices.extend_from_slice(ix);
        a.prov.push(p);
    };
    let pv = |f, s, t, l, e| Prov { v: [f, s, t, l], edges: e };
    match prim {
        // points
        1 => {
            a.class = Class::Points;
            for i in 0..n {
                put(&mut a, &[i], pv(i, i, i, i, 0));
            }
        }
        // lines
        2 => {
            a.class = Class::Lines;
            for i in (0..n.saturating_sub(1)).step_by(2) {
                put(&mut a, &[i, i + 1], pv(i, i + 1, i + 1, i + 1, 1));
            }
        }
        // line strip, line loop
        3 | 12 => {
            a.class = Class::Lines;
            for i in 0..n.saturating_sub(1) {
                put(&mut a, &[i, i + 1], pv(i, i + 1, i + 1, i + 1, 1));
            }
            if prim == 12 && n > 2 {
                put(&mut a, &[n - 1, 0], pv(n - 1, 0, 0, 0, 1));
            }
        }
        // triangles, triangles with wFlags
        4 | 7 => {
            for i in (0..n.saturating_sub(2)).step_by(3) {
                put(&mut a, &[i, i + 1, i + 2], pv(i, i + 1, i + 2, i + 2, 7));
            }
        }
        // fan
        5 => {
            for i in 1..n.saturating_sub(1) {
                put(&mut a, &[0, i, i + 1], pv(0, i, i + 1, i + 1, 7));
            }
        }
        // polygon
        15 => {
            for i in 1..n.saturating_sub(1) {
                // PROVOKING_VERTEX_LAST on a polygon reduces to the first
                // vertex (Mesa r300_provoking_vertex_fixes)
                let edges = u32::from(i == 1) | 2 | if i + 2 == n { 4 } else { 0 };
                put(&mut a, &[0, i, i + 1], pv(0, 1, 2, 0, edges));
            }
        }
        // strip
        6 => {
            for i in 0..n.saturating_sub(2) {
                let ix = if i & 1 != 0 { [i + 1, i, i + 2] } else { [i, i + 1, i + 2] };
                put(&mut a, &ix, pv(i, i + 1, i + 2, i + 2, 7));
            }
        }
        // quads
        13 => {
            for i in (0..n.saturating_sub(3)).step_by(4) {
                put(&mut a, &[i, i + 1, i + 2], pv(i, i + 1, i + 2, i + 3, 3));
                put(&mut a, &[i, i + 2, i + 3], pv(i, i + 1, i + 2, i + 3, 6));
            }
        }
        // quad strip
        14 => {
            for i in (0..n.saturating_sub(3)).step_by(2) {
                put(&mut a, &[i, i + 1, i + 3], pv(i, i + 1, i + 2, i + 3, 3));
                put(&mut a, &[i, i + 3, i + 2], pv(i, i + 1, i + 2, i + 3, 6));
            }
        }
        _ => return None,
    }
    Some(a)
}

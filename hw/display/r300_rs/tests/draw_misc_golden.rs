//! The self-contained helpers of the draw module against the C original (see
//! `tools/gen_draw_misc.c`).

use r300::draw::assemble::{assemble, Class};
use r300::draw::layout::{border_color, tex_layout, tex_level_dims};
use r300::draw::misc::*;
use r300::state::R300State;
use r300::types::R300TexDesc;

const GOLDEN: &str = include_str!("golden/draw_misc.txt");

fn rows(tag: &str) -> Vec<Vec<&'static str>> {
    GOLDEN
        .lines()
        .filter(|l| l.split(' ').next() == Some(tag))
        .map(|l| l.split(' ').skip(1).collect())
        .collect()
}

fn nums(f: &[&str]) -> Vec<u32> {
    f.iter().map(|x| x.parse().unwrap()).collect()
}

fn hex(f: &[&str]) -> Vec<u32> {
    f.iter().map(|x| u32::from_str_radix(x, 16).unwrap()).collect()
}

fn fnv(h: u64, bytes: &[u8]) -> u64 {
    bytes.iter().fold(h, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

#[test]
fn texture_layout_matches_c() {
    let r = rows("Y");
    assert_eq!(r.len(), 300);
    for (case, f) in r.iter().enumerate() {
        let (head, tail) = f.split_at(10);
        let h = nums(head);
        let want = nums(&tail[1..]);
        let mut td = R300TexDesc {
            dim: h[0],
            width: h[1],
            height: h[2],
            depth: h[3],
            levels: h[4],
            tile: h[5],
            pitch_bytes: h[6],
            ..Default::default()
        };
        tex_layout(&mut td, h[7], h[8] != 0, h[9] != 0);
        let mut got = vec![td.size_bytes];
        got.extend(td.lvl_off);
        got.extend(td.lvl_pitch);
        got.extend(td.lvl_rows);
        assert_eq!(got, want, "case {case}: {head:?}");
    }
}

#[test]
fn level_dimensions_match_c() {
    let r = rows("D");
    assert_eq!(r.len(), 3600);
    for f in r {
        let n = nums(&f[..5]);
        let td = R300TexDesc { dim: n[0], width: n[1], height: n[2], depth: n[3], ..Default::default() };
        let (w, h, d) = tex_level_dims(&td, n[4]);
        assert_eq!(vec![w, h, d], nums(&f[6..9]), "{f:?}");
    }
}

#[test]
fn border_colours_match_c_bit_for_bit() {
    let r = rows("C");
    assert_eq!(r.len(), 384);
    for f in r {
        let (fmt, v) = (f[0].parse().unwrap(), f[1].parse().unwrap());
        let got = border_color(fmt, v).map(f32::to_bits);
        assert_eq!(got.to_vec(), hex(&f[3..7]), "format {fmt} value {v:#x}");
    }
}

#[test]
fn primitive_assembly_matches_c() {
    let r = rows("A");
    assert_eq!(r.len(), 252);
    for f in r {
        let (prim, n): (u32, u32) = (f[0].parse().unwrap(), f[1].parse().unwrap());
        let (cls, m, m2, hash) = (&f[2], &f[3], &f[4], f[5]);
        let a = assemble(prim, n);
        let count = a.as_ref().map_or(0, |a| a.indices.len());
        // C returns 0 both for an unsupported type and for too few vertices.
        assert_eq!(count.to_string(), *m, "prim {prim} n {n}");
        assert_eq!(m, m2);
        let Some(a) = a.filter(|a| !a.indices.is_empty()) else {
            assert_eq!(*cls, "99");
            continue;
        };
        assert_eq!((a.class as u32).to_string(), *cls, "prim {prim} n {n}");
        let mut h = fnv(0xcbf2_9ce4_8422_2325, &a.indices.iter().flat_map(|v| v.to_le_bytes()).collect::<Vec<_>>());
        for p in &a.prov {
            let words = [p.v[0], p.v[1], p.v[2], p.v[3], p.edges];
            h = fnv(h, &words.iter().flat_map(|v| v.to_le_bytes()).collect::<Vec<_>>());
        }
        assert_eq!(format!("{h:016x}"), hash, "prim {prim} n {n}");
        assert!(matches!(a.class, Class::Triangles | Class::Lines | Class::Points));
    }
}

#[test]
fn multisample_positions_and_offsets_match_c() {
    for f in rows("Q") {
        let h = hex(&f[..3]);
        let mut st = R300State::default();
        st.write(0x4020, h[0]);
        st.write(0x4010, h[1]);
        st.write(0x4014, h[2]);
        assert_eq!(aa_samples(&st).to_string(), f[4]);
        let got: Vec<u32> = aa_positions(&st).iter().flatten().map(|v| v.to_bits()).collect();
        assert_eq!(got, hex(&f[5..17]));
    }
    let r = rows("M");
    assert_eq!(r.len(), 320);
    for f in r {
        let n = nums(&f);
        let got = msaa_offset(n[0], n[1], n[2], n[3], n[4], n[5]).unwrap_or(u32::MAX);
        assert_eq!(got, n[6], "{f:?}");
    }
}

#[test]
fn index_streams_match_c() {
    let r = rows("X");
    let f = &r[0];
    let sep = f.iter().position(|w| *w == "|").unwrap();
    let dw = hex(&f[..sep]);
    let want = hex(&f[sep + 1..]);
    let mut got = vec![];
    for i32_indices in [false, true] {
        got.extend((0..20).map(|i| index_at(&dw, i32_indices, i)));
    }
    assert_eq!(got, want);
}

#[test]
fn content_hashes_match_c() {
    let r = rows("H");
    assert_eq!(r.len(), 13);
    for (k, f) in r.iter().enumerate() {
        let n: usize = f[0].parse().unwrap();
        let mut s = k as u32 + 1;
        let buf: Vec<u8> = (0..n)
            .map(|_| {
                s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                (s >> 24) as u8
            })
            .collect();
        assert_eq!(format!("{:016x}", hash_bytes(&buf)), f[2], "bytes, n = {n}");
        assert_eq!(format!("{:016x}", hash_tex(&buf)), f[3], "tex, n = {n}");
    }
}

#[test]
fn vertex_array_pointers_match_c() {
    let r = rows("L");
    assert_eq!(r.len(), 60);
    for f in r {
        let ndw: usize = f[0].parse().unwrap();
        let d = hex(&f[1..1 + ndw]);
        let want = &f[1 + ndw + 1..];
        let arr = load_vbpntr(&d);
        assert_eq!(arr.count.to_string(), want[0], "{f:?}");
        for (k, a) in arr.a.iter().enumerate() {
            let w = &want[1 + 3 * k..4 + 3 * k];
            assert_eq!(
                (format!("{:08x}", a.addr), a.size_dw.to_string(), a.stride_dw.to_string()),
                (w[0].to_string(), w[1].to_string(), w[2].to_string()),
                "array {k} of {f:?}"
            );
        }
    }
}

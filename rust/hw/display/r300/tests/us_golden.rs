//! The fragment-shader translator against the C original (see
//! `tools/gen_us.c`): the lookup tables, and the GLSL text of random programs.

use r300::state::R300State;
use r300::us::*;

const GOLDEN: &str = include_str!("golden/us.txt");

fn fnv(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

fn words(tag: &str) -> Vec<Vec<&'static str>> {
    GOLDEN
        .lines()
        .filter(|l| l.split(' ').next() == Some(tag))
        .map(|l| l.split(' ').skip(1).collect())
        .collect()
}

#[test]
fn raw_texel_sizes_match_c() {
    let rows = words("B");
    assert_eq!(rows.len(), 32);
    for r in rows {
        assert_eq!(tex_raw_bpp(r[0].parse().unwrap()).to_string(), r[1], "format {}", r[0]);
    }
}

#[test]
fn colour_buffer_views_match_c() {
    let rows = words("V");
    assert_eq!(rows.len(), 512);
    for r in rows {
        let (view, bpp) = cb_view(r[0].parse().unwrap(), r[1].parse().unwrap());
        assert_eq!((view as u32).to_string(), r[2], "cf {} of {}", r[0], r[1]);
        assert_eq!(bpp.to_string(), r[3], "cf {} of {}", r[0], r[1]);
    }
}

#[test]
fn float24_matches_c_bit_for_bit() {
    let rows = words("F");
    assert_eq!(rows.len(), 400);
    for r in rows {
        let got = float24(u32::from_str_radix(r[0], 16).unwrap()).to_bits();
        assert_eq!(format!("{got:08x}"), r[1], "value {}", r[0]);
    }
}

struct Case {
    state: R300State,
    route: [i8; NUM_TEMPS],
    vs: bool,
}

fn cases() -> Vec<Case> {
    let writes = words("W");
    let descs = words("D");
    assert_eq!(writes.len(), 160);
    writes
        .iter()
        .zip(&descs)
        .map(|(w, d)| {
            let mut state = R300State::default();
            let n: usize = w[1].parse().unwrap();
            for i in 0..n {
                let addr = u32::from_str_radix(w[2 + 2 * i], 16).unwrap();
                let val = u32::from_str_radix(w[3 + 2 * i], 16).unwrap();
                state.write(addr, val);
            }
            let mut route = [0i8; NUM_TEMPS];
            for (k, r) in route.iter_mut().enumerate() {
                *r = d[2 + k].parse().unwrap();
            }
            Case { state, route, vs: d[1] == "1" }
        })
        .collect()
}

const VS_GPU: &str = "void main() { gl_Position = vec4(0.0); }\n";

#[test]
fn targets_formats_and_disassembly_match_c() {
    let cases = cases();
    for r in words("N") {
        let c = &cases[r[0].parse::<usize>().unwrap()];
        assert_eq!(num_targets(&c.state).to_string(), r[1], "case {}", r[0]);
        for k in 0..4 {
            assert_eq!(format!("{:08x}", out_fmt(&c.state, k)), r[2 + k], "case {} target {k}", r[0]);
        }
        let dis = disasm(&c.state);
        assert_eq!(dis.len().to_string(), r[6], "case {}", r[0]);
        assert_eq!(format!("{:016x}", fnv(&dis)), r[7], "case {}", r[0]);
    }
}

#[test]
fn glsl_matches_c_for_every_program_and_flag() {
    let cases = cases();
    let (mut ok, mut refused) = (0, 0);
    let mut lines = GOLDEN.lines().filter(|l| l.starts_with("U ") || l.starts_with("E "));
    for (idx, c) in cases.iter().enumerate() {
        for flags in 0..4u32 {
            let desc = FsDesc { route: c.route, vs_gpu: c.vs.then_some(VS_GPU) };
            let got = to_glsl(&c.state, &desc, flags);
            let want: Vec<&str> = lines.next().unwrap().splitn(4, ' ').collect();
            assert_eq!((want[1], want[2]), (&*idx.to_string(), &*flags.to_string()));
            match (want[0], got) {
                ("U", Ok(text)) => {
                    let (len, hash) = want[3].split_once(' ').unwrap();
                    assert_eq!(text.len().to_string(), len, "case {idx} flags {flags}: length");
                    assert_eq!(format!("{:016x}", fnv(&text)), hash, "case {idx} flags {flags}");
                    ok += 1;
                }
                ("E", Err(e)) => {
                    assert_eq!(e, want[3], "case {idx} flags {flags}");
                    refused += 1;
                }
                (tag, got) => panic!("case {idx} flags {flags}: C says {tag}, Rust says {got:?}"),
            }
        }
    }
    assert_eq!((ok, refused), (524, 116));
}

#[test]
fn sample_texts_match_c_exactly() {
    let cases = cases();
    let mut rest = GOLDEN;
    let mut samples = 0;
    while let Some(at) = rest.find("\n@@ ") {
        rest = &rest[at + 4..];
        let (head, body) = rest.split_once('\n').unwrap();
        let h: Vec<&str> = head.split(' ').collect();
        let (idx, flags, len): (usize, u32, usize) = (h[0].parse().unwrap(), h[1].parse().unwrap(), h[2].parse().unwrap());
        let c = &cases[idx];
        let desc = FsDesc { route: c.route, vs_gpu: c.vs.then_some(VS_GPU) };
        assert_eq!(to_glsl(&c.state, &desc, flags).unwrap(), body[..len], "case {idx} flags {flags}");
        samples += 1;
    }
    assert_eq!(samples, 5);
}

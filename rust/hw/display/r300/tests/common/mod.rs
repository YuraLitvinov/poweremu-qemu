//! Parser for `tests/golden/pvs.txt` (see `tools/gen_pvs.c` for the format).
#![allow(dead_code)]

use r300::pvs::{FlowControl, Inputs, Outputs, ProgramDesc, Vec4};

pub const GOLDEN: &str = include_str!("../golden/pvs.txt");

pub struct Case {
    pub idx: usize,
    pub first: usize,
    pub last: usize,
    pub max_const: i32,
    pub flow: FlowControl,
    pub code: Vec<u32>,
}

impl Case {
    pub fn desc<'a>(&'a self, consts: &'a [Vec4]) -> ProgramDesc<'a> {
        ProgramDesc {
            code: &self.code,
            first_inst: self.first,
            last_inst: self.last,
            consts,
            max_const: self.max_const,
            flow: self.flow,
        }
    }
}

pub struct Golden {
    pub consts: Vec<Vec4>,
    pub inputs: Vec<Inputs>,
    pub cases: Vec<Case>,
}

pub fn fnv(h: u64, bytes: &[u8]) -> u64 {
    bytes.iter().fold(h, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

pub const FNV_BASIS: u64 = 0xcbf2_9ce4_8422_2325;

pub fn hash_outputs(h: u64, outs: &[Outputs]) -> u64 {
    // -0.0 hashes as +0.0: the sign of a zero that fmax/fmin return for a +-0 tie
    // is unspecified in C (see tools/gen_pvs.c).
    outs.iter().flatten().flatten().fold(h, |h, v| {
        let bits = if v.to_bits() == 0x8000_0000 { 0 } else { v.to_bits() };
        fnv(h, &bits.to_le_bytes())
    })
}

/// The output memory as the generator primes it: distinct bit patterns, so
/// that "keeps its previous contents" is visible.
pub fn primed_outputs() -> Outputs {
    let mut out = [[0.0; 4]; 32];
    for (i, o) in out.iter_mut().enumerate() {
        for (k, v) in o.iter_mut().enumerate() {
            *v = f32::from_bits(0x3f80_0000 + (i * 16 + k) as u32);
        }
    }
    out
}

pub fn load() -> Golden {
    let hex = |w: &str| f32::from_bits(u32::from_str_radix(w, 16).unwrap());
    let mut g = Golden { consts: vec![], inputs: vec![], cases: vec![] };
    for line in GOLDEN.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        match f.first().copied() {
            Some("K") => g.consts.push([hex(f[1]), hex(f[2]), hex(f[3]), hex(f[4])]),
            Some("V") => {
                let mut v = [[0.0; 4]; 32];
                for (i, w) in f[2..].iter().enumerate() {
                    v[i / 4][i % 4] = hex(w);
                }
                g.inputs.push(v);
            }
            Some("C") => {
                let n: Vec<u32> = f[1..].iter().map(|x| x.parse().unwrap()).collect();
                let mut flow = FlowControl { opc: n[4], ..Default::default() };
                flow.addrs.copy_from_slice(&n[5..21]);
                flow.loops.copy_from_slice(&n[21..37]);
                g.cases.push(Case {
                    idx: n[0] as usize,
                    first: n[1] as usize,
                    last: n[2] as usize,
                    max_const: n[3] as i32,
                    flow,
                    code: n[37..].to_vec(),
                });
            }
            _ => {}
        }
    }
    g
}

/// Lines starting with `tag`, as (words after the tag).
pub fn lines(tag: &str) -> Vec<Vec<&'static str>> {
    GOLDEN
        .lines()
        .filter(|l| l.split_whitespace().next() == Some(tag))
        .map(|l| l.split_whitespace().skip(1).collect())
        .collect()
}

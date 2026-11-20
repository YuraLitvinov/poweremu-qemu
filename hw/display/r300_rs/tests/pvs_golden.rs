//! The PVS interpreter and disassembler against the C original, over 400
//! random programs (see `tools/gen_pvs.c`).

mod common;

use common::*;
use r300::pvs::{disasm_inst, Program};

#[test]
fn interpreter_matches_c() {
    let g = load();
    let runs = lines("R");
    assert_eq!(g.cases.len(), 400);
    for (case, want) in g.cases.iter().zip(&runs) {
        assert_eq!(case.idx.to_string(), want[0]);
        let prog = Program::new(case.desc(&g.consts));
        let (mut h, mut unsup) = (FNV_BASIS, 0);
        for input in &g.inputs {
            let mut out = primed_outputs();
            unsup |= prog.run(input, &mut out).0;
            h = hash_outputs(h, &[out]);
        }
        assert_eq!(unsup.to_string(), want[1], "program {}: unsupported flags", case.idx);
        assert_eq!(format!("{h:016x}"), want[2], "program {}: outputs", case.idx);
    }
}

#[test]
fn batches_match_c_and_serial_runs() {
    let g = load();
    for want in lines("B") {
        let idx: usize = want[0].parse().unwrap();
        let n: usize = want[3].parse().unwrap();
        let case = &g.cases[idx];
        let prog = Program::new(case.desc(&g.consts));
        let ins: Vec<_> = (0..n).map(|i| g.inputs[i % g.inputs.len()]).collect();
        for oidx in [(0..n as u32).collect::<Vec<_>>(), (0..n as u32).rev().collect()] {
            let mut batch = vec![primed_outputs(); n];
            let u = prog.run_batch(&ins, &mut batch, &oidx);
            let mut serial = vec![primed_outputs(); n];
            let mut us = 0;
            for i in 0..n {
                us |= prog.run(&ins[i], &mut serial[oidx[i] as usize]).0;
            }
            assert_eq!(batch, serial, "program {idx}");
            assert_eq!(u.0, us);
        }
        // The shuffled order the generator used, against the C result.
        let oidx: Vec<u32> = lines("O").iter().find(|l| l[0] == want[0]).unwrap()[1..]
            .iter()
            .map(|x| x.parse().unwrap())
            .collect();
        let mut outs = vec![primed_outputs(); n];
        let u = prog.run_batch(&ins, &mut outs, &oidx);
        assert_eq!(u.0.to_string(), want[1], "program {idx}: batch flags");
        assert_eq!(format!("{:016x}", hash_outputs(FNV_BASIS, &outs)), want[2], "program {idx}: batch outputs");
    }
}

#[test]
fn disassembly_matches_c() {
    let g = load();
    let mut checked = 0;
    for l in GOLDEN.lines().filter(|l| l.starts_with("M ")) {
        let mut parts = l.splitn(4, ' ');
        parts.next();
        let idx: usize = parts.next().unwrap().parse().unwrap();
        let i: usize = parts.next().unwrap().parse().unwrap();
        let text = parts.next().unwrap();
        let w = &g.cases[idx].code[i * 4..i * 4 + 4];
        assert_eq!(disasm_inst(&[w[0], w[1], w[2], w[3]]), text, "program {idx} inst {i}");
        checked += 1;
    }
    assert!(checked > 1500, "{checked}");
}

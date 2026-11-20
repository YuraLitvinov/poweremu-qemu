//! The GLSL translator against the C original: every program's text by hash
//! and length, and a few in full (see `tools/gen_pvs.c`).

mod common;

use common::*;
use r300::pvs::Program;
use r300::pvs_glsl::to_glsl;

#[test]
fn translation_matches_c_for_every_program() {
    let g = load();
    let mut translated = 0;
    for (case, want) in g.cases.iter().zip(lines("G")) {
        let prog = Program::new(case.desc(&g.consts));
        match to_glsl(&prog) {
            Ok(text) => {
                assert_eq!(want[1], "0", "program {}: C refused it", case.idx);
                assert_eq!(text.len().to_string(), want[3], "program {}: length", case.idx);
                assert_eq!(format!("{:016x}", fnv(FNV_BASIS, text.as_bytes())), want[2], "program {}", case.idx);
                translated += 1;
            }
            Err(e) => assert_eq!(e.0.to_string(), want[1], "program {}: refusal flags", case.idx),
        }
    }
    assert_eq!(translated, 229);
}

#[test]
fn sample_texts_match_c_exactly() {
    let g = load();
    let mut samples = 0;
    let mut rest = GOLDEN;
    while let Some(at) = rest.find("\n@@ ") {
        rest = &rest[at + 4..];
        let (head, body) = rest.split_once('\n').unwrap();
        let (idx, len) = head.split_once(' ').unwrap();
        let (idx, len): (usize, usize) = (idx.parse().unwrap(), len.parse().unwrap());
        let want = &body[..len];
        let prog = Program::new(g.cases[idx].desc(&g.consts));
        assert_eq!(to_glsl(&prog).unwrap(), want, "program {idx}");
        samples += 1;
    }
    assert_eq!(samples, 6);
}

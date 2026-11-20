//! GLSL to SPIR-V against the C original (needs libshaderc).

use r300::spirv::{glsl_to_spirv, Stage};

const GOLDEN: &str = include_str!("golden/spirv.txt");
const STAGE: &str = include_str!("golden/shaders/stage.glsl");
const BROKEN: &str = include_str!("golden/shaders/broken.glsl");
const STAGES: [Stage; 3] = [Stage::Vs, Stage::Fs, Stage::FsZ];

fn fnv(words: &[u32]) -> u64 {
    words
        .iter()
        .flat_map(|w| w.to_ne_bytes())
        .fold(0xcbf2_9ce4_8422_2325, |h, b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

#[test]
fn compiled_stages_match_c() {
    let mut checked = 0;
    for line in GOLDEN.lines().filter(|l| l.starts_with("S ")) {
        let f: Vec<&str> = line.split(' ').collect();
        let stage = STAGES[f[2].parse::<usize>().unwrap()];
        let words = glsl_to_spirv(STAGE, stage).unwrap();
        assert_eq!(words.len().to_string(), f[3], "{stage:?}");
        assert_eq!(format!("{:016x}", fnv(&words)), f[4], "{stage:?}");
        checked += 1;
    }
    assert_eq!(checked, 3);
}

#[test]
fn errors_match_c() {
    let mut checked = 0;
    for line in GOLDEN.lines().filter(|l| l.starts_with("E ")) {
        // E file stage message
        let mut f = line.splitn(4, ' ').skip(2);
        let (head, msg) = (f.next().unwrap(), f.next().unwrap());
        let stage = STAGES[head.parse::<usize>().unwrap()];
        let err = glsl_to_spirv(BROKEN, stage).unwrap_err();
        assert_eq!(err.replace('\n', "|"), msg);
        checked += 1;
    }
    assert_eq!(checked, 3);
}

#[test]
fn entry_names_match_c() {
    for line in GOLDEN.lines().filter(|l| l.starts_with("N ")) {
        let f: Vec<&str> = line.split(' ').collect();
        assert_eq!(STAGES[f[1].parse::<usize>().unwrap()].entry(), f[2]);
    }
}

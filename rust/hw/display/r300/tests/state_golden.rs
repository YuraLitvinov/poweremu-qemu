//! The state module against the C original (`tests/golden/state*.txt`).

use r300::cfmt::write_g;
use r300::regnames::REG_NAMES;
use r300::state::*;

const GOLDEN: &str = include_str!("golden/state.txt");

fn fnv_add(mut h: u64, s: &str) -> u64 {
    for &b in s.as_bytes() {
        h = (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3);
    }
    h
}

#[test]
fn percent_g_matches_c() {
    let mut checked = 0;
    for line in GOLDEN.lines().filter(|l| l.starts_with("G ")) {
        let mut f = line.split_whitespace().skip(1);
        let bits = u32::from_str_radix(f.next().unwrap(), 16).unwrap();
        let want = f.next().unwrap();
        let mut got = String::new();
        write_g(&mut got, f32::from_bits(bits)).unwrap();
        assert_eq!(got, want, "bits {bits:08x}");
        checked += 1;
    }
    assert_eq!(checked, 529);
}

#[test]
fn register_names_match_c_over_the_whole_window() {
    let want = GOLDEN.lines().find_map(|l| l.strip_prefix("N ")).unwrap();
    let mut h = 0xcbf2_9ce4_8422_2325;
    for addr in (REG_BASE - 4..=REG_END).step_by(4) {
        let name = reg_name(addr).map_or("(null)", |n| n.to_str().unwrap());
        h = fnv_add(h, &format!("{addr:04x}={name}\n"));
    }
    assert_eq!(format!("{h:016x}"), want);
}

#[test]
fn name_table_is_the_c_header() {
    let hdr = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../hw/display/r300/r300_regnames.h")).unwrap();
    let rows: Vec<(u16, &str)> = hdr
        .lines()
        .filter_map(|l| {
            let l = l.trim().strip_prefix("{ 0x")?;
            let (addr, rest) = l.split_once(", \"")?;
            Some((u16::from_str_radix(addr, 16).ok()?, rest.split_once('"')?.0))
        })
        .collect();
    let table: Vec<(u16, &str)> = REG_NAMES.iter().map(|&(a, n)| (a, n.to_str().unwrap())).collect();
    assert_eq!(table, rows, "run tools/gen_regnames.py");
}

fn sample_state() -> R300State {
    // The same writes tools/gen_state.c makes.
    let mut st = R300State::default();
    let mut s: u32 = 7;
    let mut lcg = || {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        s
    };
    st.write(0x1000, 0x1234);
    st.write(0x1D98, 0x3f80_0000);
    st.write(0x2080, 0x5);
    st.write(0x2138, 0xdead_beef);
    st.write(0x2FFC, 0x77);
    st.write(0x4F60, 0x10);
    st.write(0x22D0, 2 | (4 << 10) | (5 << 20));
    st.write(VAP_PVS_VECTOR_INDX_REG, 0);
    for _ in 0..24 {
        st.write(VAP_PVS_VECTOR_DATA_REG, lcg());
    }
    st.write(VAP_PVS_VECTOR_INDX_REG, PVS_CONST_START as u32);
    let fl = [0x3f80_0000, 0xbf00_0000, 0x4120_0000, 0x3dcc_cccd, 0x7f80_0000, 0, 0x3586_37bd, 0x4974_2400];
    for i in 0..32 {
        st.write(VAP_PVS_UPLOAD_DATA, fl[i % 8]);
    }
    st.write(VAP_PVS_VECTOR_INDX_REG, PVS_MEM_VECS as u32 - 1);
    for i in 0..9 {
        st.write(VAP_PVS_VECTOR_DATA_REG, 0x1111_1111 * (i + 1));
    }
    st.draws = 12_345_678_901;
    st
}

#[test]
fn dump_matches_c_text_for_text() {
    let st = sample_state();
    let mut got = format!("gen={} dw={}\n", st.pvs_gen, st.pvs_upload_dw);
    st.dump(&mut got).unwrap();
    assert_eq!(got, include_str!("golden/state_dump.txt"));
}

#[test]
fn writes_outside_the_window_are_ignored_and_pvs_overrun_is_dropped() {
    let mut st = R300State::default();
    st.write(REG_BASE - 4, 1);
    st.write(REG_END, 1);
    assert!(st.regs.iter().all(|&r| r == 0));
    st.write(VAP_PVS_VECTOR_INDX_REG, PVS_MEM_VECS as u32 * 4 / 4 - 1);
    for _ in 0..8 {
        st.write(VAP_PVS_VECTOR_DATA_REG, 9);
    }
    assert_eq!(st.pvs_upload_dw as usize, (PVS_MEM_VECS - 1) * 4 + 8);
    assert_eq!(st.reg(0x10), 0, "outside the window reads 0");
}

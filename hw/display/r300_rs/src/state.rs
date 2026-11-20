//! R300-family 3D engine state: the register file the command processor
//! writes, plus the memories behind the indexed upload ports
//! (`r300_state.c`).

use core::ffi::CStr;
use core::fmt::{self, Write};

use crate::cfmt::write_g;
use crate::regnames::REG_NAMES;

/// 3D register window mirrored here (VAP 0x2000.. through ZB 0x4Fxx).
pub const REG_BASE: u32 = 0x1C00;
pub const REG_END: u32 = 0x5000;
pub const REG_COUNT: usize = ((REG_END - REG_BASE) / 4) as usize;

/// PVS vector memory: code at 0, constants at 512, clip planes at 1024.
pub const PVS_MEM_VECS: usize = 1536;
pub const PVS_CODE_START: usize = 0;
pub const PVS_CONST_START: usize = 512;
pub const PVS_UCP_START: usize = 1024;

pub const VAP_PVS_VECTOR_INDX_REG: u32 = 0x2200;
pub const VAP_PVS_VECTOR_DATA_REG: u32 = 0x2204;
pub const VAP_PVS_UPLOAD_DATA: u32 = 0x2208;
pub const VAP_PVS_CODE_CNTL_0: u32 = 0x22D0;

/// `R300State`.  Embedded in the C device, so the layout is the C struct's.
#[repr(C)]
#[derive(Clone, Debug)]
pub struct R300State {
    pub regs: [u32; REG_COUNT],
    /// 4 dwords per vector.
    pub pvs_mem: [u32; PVS_MEM_VECS * 4],
    /// Next dword to write.
    pub pvs_upload_dw: u32,
    /// Bumps when PVS code/constants change.
    pub pvs_gen: u64,
    pub draws: u64,
    /// A `3D_DRAW_IMMD` (0x29) packet was seen: the guest runs the Windows
    /// driver.
    pub win_immd: bool,
    /// Little-endian (Linux) guest: the screen height, else 0.  Linux clips
    /// with the SC_CLIPRECT registers and leaves the scissor open, and the
    /// colour buffer is screen-sized.
    pub le_screen_h: u32,
    /// `r300_us_to_glsl()` flags the renderer wants (not a register; the
    /// device sets it before each draw).
    pub glsl_flags: u32,
}

impl Default for R300State {
    fn default() -> Self {
        Self {
            regs: [0; REG_COUNT],
            pvs_mem: [0; PVS_MEM_VECS * 4],
            pvs_upload_dw: 0,
            pvs_gen: 0,
            draws: 0,
            win_immd: false,
            le_screen_h: 0,
            glsl_flags: 0,
        }
    }
}

/// Whether `addr` is a register in the mirrored window.
pub fn owns(addr: u32) -> bool {
    (REG_BASE..REG_END).contains(&addr)
}

impl R300State {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// The register at byte offset `addr`; 0 outside the window.
    pub fn reg(&self, addr: u32) -> u32 {
        addr.checked_sub(REG_BASE)
            .and_then(|off| self.regs.get((off / 4) as usize))
            .copied()
            .unwrap_or(0)
    }

    /// A register write from MMIO or a PM4 type-0/1 packet.  `addr` is a byte
    /// offset in the MMIO BAR; writes outside the 3D window are ignored.
    pub fn write(&mut self, addr: u32, val: u32) {
        if !owns(addr) {
            return;
        }
        self.regs[((addr - REG_BASE) / 4) as usize] = val;

        match addr {
            // Vector index; data writes then fill 4 dwords per vector.
            VAP_PVS_VECTOR_INDX_REG => self.pvs_upload_dw = (val & 0x7FF) * 4,
            // The 32-bit data port (the Radeon DDX) and the 128-bit one (Mesa).
            VAP_PVS_VECTOR_DATA_REG | VAP_PVS_UPLOAD_DATA => {
                if let Some(dw) = self.pvs_mem.get_mut(self.pvs_upload_dw as usize) {
                    *dw = val;
                }
                self.pvs_upload_dw = self.pvs_upload_dw.wrapping_add(1);
                self.pvs_gen = self.pvs_gen.wrapping_add(1);
            }
            _ => {}
        }
    }

    /// Writes every non-zero register (with names where known) and the live
    /// part of PVS memory, for reverse-engineering what a driver programs.
    pub fn dump(&self, out: &mut impl Write) -> fmt::Result {
        writeln!(out, "=== R300 state at draw {} ===", self.draws)?;
        for (i, &v) in self.regs.iter().enumerate() {
            let addr = REG_BASE + i as u32 * 4;
            if v == 0 || addr == VAP_PVS_UPLOAD_DATA {
                continue;
            }
            let name = reg_name(addr).and_then(|n| n.to_str().ok()).unwrap_or("");
            writeln!(out, "  {addr:04X} {v:08X}  {name}")?;
        }

        let cntl = self.reg(VAP_PVS_CODE_CNTL_0);
        let (first, last) = ((cntl & 0x3FF) as usize, ((cntl >> 20) & 0x3FF) as usize);
        writeln!(out, "  PVS code {first}..{last} (xyzw valid after {}):", (cntl >> 10) & 0x3FF)?;
        for i in (first..=last).take_while(|&i| i < 1024) {
            let d = self.pvs_vec(PVS_CODE_START + i);
            writeln!(out, "    {i:3}: {:08X} {:08X} {:08X} {:08X}", d[0], d[1], d[2], d[3])?;
        }
        writeln!(out, "  PVS constants (non-zero):")?;
        for i in 0..256 + 6 {
            let d = self.pvs_vec(PVS_CONST_START + i);
            if d.iter().any(|&w| w != 0) {
                write!(out, "    {:<4}", format!("c{i}"))?;
                for &w in d {
                    out.write_char(' ')?;
                    write_g(out, f32::from_bits(w))?;
                }
                out.write_char('\n')?;
            }
        }
        Ok(())
    }

    /// One 4-dword PVS memory vector.
    fn pvs_vec(&self, index: usize) -> &[u32; 4] {
        let (vecs, _) = self.pvs_mem.as_chunks::<4>();
        &vecs[index]
    }
}

/// Register name for traces.
pub fn reg_name(addr: u32) -> Option<&'static CStr> {
    let addr = u16::try_from(addr).ok()?;
    REG_NAMES.binary_search_by_key(&addr, |&(a, _)| a).ok().map(|i| REG_NAMES[i].1)
}

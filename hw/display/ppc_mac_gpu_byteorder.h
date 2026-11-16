/*
 * Guest byte order for the ppc_mac_gpu device, fixed per build: the PowerPC
 * build defines PPC_MAC_GPU_BE=1, the x86 build PPC_MAC_GPU_BE=0 (hw/display/
 * meson.build).  Branches on it are removed by the preprocessor.
 *
 * This work is licensed under the terms of the GNU GPL, version 2 or later.
 */

#ifndef PPC_MAC_GPU_BYTEORDER_H
#define PPC_MAC_GPU_BYTEORDER_H

#ifndef PPC_MAC_GPU_BE
#define PPC_MAC_GPU_BE 0
#endif

/*
 * Byte-order primitives: on the little-endian build the value passes through
 * unchanged, on the big-endian build it is swapped to the guest's order.  The
 * *_BE / *_LE selectors name the function variant for the build.  Call sites
 * use these instead of testing PPC_MAC_GPU_BE themselves.
 */
/* 1 when the guest is little-endian (the x86 build), 0 for the PowerPC build. */
#if PPC_MAC_GPU_BE
#define GUEST_LE           0
#define GUEST_TO_CPU32(x)  be32_to_cpu(x)
#define GUEST_STW_P(p, v)  stw_be_p((p), (v))
#define GUEST_LDL_P(p)     ldl_be_p(p)
#define GUEST_LDUW_P(p)    lduw_be_p(p)
#define GUEST_STL_P(p, v)  stl_be_p((p), (v))
#define GUEST_FMT32()      ppc_mac_gpu_fmt32_be()
#define GUEST_FMT24()      ppc_mac_gpu_fmt24_be()
#define GUEST_LINE32       ppc_mac_gpu_line32_be
#define GUEST_SCAN16       ppc_mac_gpu_scan16_be
#else
#define GUEST_LE           1
#define GUEST_TO_CPU32(x)  (x)
#define GUEST_STW_P(p, v)  stw_le_p((p), (v))
#define GUEST_LDL_P(p)     ldl_le_p(p)
#define GUEST_LDUW_P(p)    lduw_le_p(p)
#define GUEST_STL_P(p, v)  stl_le_p((p), (v))
#define GUEST_FMT32()      ppc_mac_gpu_fmt32_le()
#define GUEST_FMT24()      ppc_mac_gpu_fmt24_le()
#define GUEST_LINE32       ppc_mac_gpu_line32_le
#define GUEST_SCAN16       ppc_mac_gpu_scan16_le
#endif

#endif

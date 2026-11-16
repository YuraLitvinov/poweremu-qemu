/*
 * The PowerMac UniNorth AGP bridge's GART, as the Radeon model sees it, for
 * machines that have no such bridge (an x86 guest).
 *
 * ppc_mac_gpu.c asks the bridge where the guest's AGP GART table is and when
 * it last changed.  hw/pci-host/uninorth.c defines both for real; these weak
 * definitions only exist so that a target without it links.  "No bridge GART"
 * (base 0) makes the model fall back to the card's own PCI GART (AIC_PT_BASE),
 * which is what a Linux guest programs.
 *
 * The generation never changes here: for a little-endian guest the model does
 * not cache GART translations at all (see gpu_agp_page).
 *
 * This work is licensed under the terms of the GNU GPL, version 2 or later.
 */

#include "qemu/osdep.h"
#include "exec/hwaddr.h"
#include "hw/pci-host/uninorth.h"

hwaddr __attribute__((weak)) uninorth_get_agp_gart_base(void)
{
    return 0;
}

uint32_t __attribute__((weak)) uninorth_get_agp_gart_gen(void)
{
    return 0;
}

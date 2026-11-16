/*
 * R300 texel conversion for renderers: internal interface between
 * r300_texel.c (shared dispatch), r300_texel_le.c (little-endian guests)
 * and r300_texel_be.c (big-endian guests).
 *
 * This work is licensed under the terms of the GNU GPL, version 2 or later.
 */

#ifndef R300_TEXEL_H
#define R300_TEXEL_H

#include "r300_draw.h"

/* A 16bpp texel as the card sees it, decoded to RGBA8 bytes. */
void r300_decode16(uint32_t fmt, uint16_t v, uint8_t out[4]);

/* Little-endian guest: texel words are the card's own. */
uint8_t *r300_level_le(const R300TexDesc *td, const uint8_t *src,
                       uint32_t l, uint32_t w, uint32_t h, uint32_t *bpr);

/* Big-endian guest: TX_OFFSET dword swap applied to the texel words. */
uint8_t *r300_level_be(const R300TexDesc *td, const uint8_t *src,
                       uint32_t l, uint32_t w, uint32_t h, uint32_t *bpr);

#endif

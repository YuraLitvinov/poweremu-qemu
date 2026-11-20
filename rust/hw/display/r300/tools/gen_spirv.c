/* Golden vectors for GLSL -> SPIR-V, from the C original (r300_spirv.c).
 * Usage: gen_spirv tests/golden/shaders
 *   S file stage nwords fnv      a stage compiled
 *   E file stage message         a stage that failed to compile
 * Only the shaderc half is covered: the SPIRV-Cross/MSL half serves the
 * Metal backend, which is not ported. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "r300_spirv.h"

static char *slurp(const char *path)
{
    FILE *f = fopen(path, "rb");
    if (!f) { perror(path); exit(1); }
    fseek(f, 0, SEEK_END); long n = ftell(f); rewind(f);
    char *s = malloc(n + 1);
    if (fread(s, 1, n, f) != (size_t)n) exit(1);
    s[n] = 0; fclose(f);
    return s;
}
static uint64_t fnv(const uint8_t *p, size_t n)
{
    uint64_t h = 0xcbf29ce484222325ull;
    for (size_t i = 0; i < n; i++) { h ^= p[i]; h *= 0x100000001b3ull; }
    return h;
}
int main(int argc, char **argv)
{
    static const char *files[] = { "stage.glsl", "broken.glsl" };
    for (int f = 0; f < 2; f++) {
        char path[512];
        snprintf(path, sizeof path, "%s/%s", argv[1], files[f]);
        char *glsl = slurp(path);
        for (int st = 0; st < 3; st++) {
            size_t n = 0; char *err = NULL;
            uint32_t *w = r300_glsl_to_spirv(glsl, (R300Stage)st, &n, &err);
            if (w) printf("S %s %d %zu %016llx\n", files[f], st, n, (unsigned long long)fnv((uint8_t *)w, n * 4));
            else {
                for (char *c = err; *c; c++) if (*c == '\n') *c = '|';
                printf("E %s %d %s\n", files[f], st, err);
            }
            free(w); free(err);
        }
        free(glsl);
    }
    for (int st = 0; st < 3; st++) printf("N %d %s\n", st, r300_stage_entry((R300Stage)st));
    return 0;
}

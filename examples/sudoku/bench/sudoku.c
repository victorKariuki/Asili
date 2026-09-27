/* Same iterative MRV backtracking algorithm as ../src/kuu.as, for benchmarking. */
#include <stdio.h>
int main(void) {
    int b[81] = {8,0,0,0,0,0,0,0,0, 0,0,3,6,0,0,0,0,0, 0,7,0,0,9,0,2,0,0,
                 0,5,0,0,0,7,0,0,0, 0,0,0,0,4,5,7,0,0, 0,0,0,1,0,0,0,3,0,
                 0,0,1,0,0,0,0,6,8, 0,0,8,5,0,0,0,1,0, 0,9,0,0,0,0,4,0,0};
    int rows[9] = {0}, cols[9] = {0}, boxes[9] = {0};
    for (int i = 0; i < 81; i++) if (b[i]) {
        int r = i / 9, c = i % 9, q = r / 3 * 3 + c / 3, x = 1 << (b[i] - 1);
        rows[r] |= x; cols[c] |= x; boxes[q] |= x;
    }
    int cells[81], next[81], depth = 0, len = 0, done = 0;
    long tries = 0, backs = 0;
    while (depth >= 0) {
        if (depth == 81) { done = 1; break; }
        if (len == depth) {
            int best = -1, count = 10;
            for (int p = 0; p < 81; p++) if (b[p] == 0) {
                int r = p / 9, c = p % 9, q = r / 3 * 3 + c / 3;
                int used = rows[r] | cols[c] | boxes[q], n = 0;
                for (int v = 1; v < 10; v++) if ((used & (1 << (v - 1))) == 0) n++;
                if (n < count) { best = p; count = n; }
            }
            if (best == -1) { done = 1; break; }
            cells[len] = best; next[len] = 1; len++;
        }
        int slot = depth, p = cells[slot];
        int r = p / 9, c = p % 9, q = r / 3 * 3 + c / 3;
        int used = rows[r] | cols[c] | boxes[q], v = next[slot], found = 0;
        while (v <= 9) {
            tries++;
            int x = 1 << (v - 1);
            if ((used & x) == 0) {
                b[p] = v; rows[r] |= x; cols[c] |= x; boxes[q] |= x;
                next[slot] = v + 1; depth++; found = 1; break;
            }
            v++;
        }
        if (!found) {
            len--; depth--; backs++;
            if (depth >= 0) {
                int old = cells[depth], orr = old / 9, oc = old % 9, oq = orr / 3 * 3 + oc / 3;
                int x = 1 << (b[old] - 1);
                b[old] = 0; rows[orr] ^= x; cols[oc] ^= x; boxes[oq] ^= x;
            }
        }
    }
    if (!done) { puts("Hakuna suluhisho."); return 0; }
    for (int i = 0; i < 81; i++) printf("%d%c", b[i], i % 9 == 8 ? '\n' : ' ');
    printf("Majaribio: %ld\nMarudio: %ld\n", tries, backs);
    return 0;
}

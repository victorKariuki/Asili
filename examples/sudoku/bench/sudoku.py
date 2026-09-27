# Same iterative MRV backtracking algorithm as ../src/kuu.as, for benchmarking.
def main():
    b = [8,0,0,0,0,0,0,0,0, 0,0,3,6,0,0,0,0,0, 0,7,0,0,9,0,2,0,0,
         0,5,0,0,0,7,0,0,0, 0,0,0,0,4,5,7,0,0, 0,0,0,1,0,0,0,3,0,
         0,0,1,0,0,0,0,6,8, 0,0,8,5,0,0,0,1,0, 0,9,0,0,0,0,4,0,0]
    rows, cols, boxes = [0] * 9, [0] * 9, [0] * 9
    for i in range(81):
        if b[i]:
            r, c = divmod(i, 9)
            q = r // 3 * 3 + c // 3
            x = 1 << (b[i] - 1)
            rows[r] |= x; cols[c] |= x; boxes[q] |= x
    cells, nxt, depth, tries, backs, done = [], [], 0, 0, 0, False
    while depth >= 0:
        if depth == 81:
            done = True; break
        if len(cells) == depth:
            best, count = -1, 10
            for p in range(81):
                if b[p] == 0:
                    r, c = divmod(p, 9)
                    used = rows[r] | cols[c] | boxes[r // 3 * 3 + c // 3]
                    n = 0
                    for v in range(1, 10):
                        if used & (1 << (v - 1)) == 0:
                            n += 1
                    if n < count:
                        best, count = p, n
            if best == -1:
                done = True; break
            cells.append(best); nxt.append(1)
        slot = depth
        p = cells[slot]
        r, c = divmod(p, 9)
        q = r // 3 * 3 + c // 3
        used = rows[r] | cols[c] | boxes[q]
        v = nxt[slot]
        found = False
        while v <= 9:
            tries += 1
            x = 1 << (v - 1)
            if used & x == 0:
                b[p] = v; rows[r] |= x; cols[c] |= x; boxes[q] |= x
                nxt[slot] = v + 1; depth += 1; found = True
                break
            v += 1
        if not found:
            cells.pop(slot); nxt.pop(slot)
            depth -= 1; backs += 1
            if depth >= 0:
                old = cells[depth]
                orr, oc = divmod(old, 9)
                x = 1 << (b[old] - 1)
                b[old] = 0
                rows[orr] ^= x; cols[oc] ^= x; boxes[orr // 3 * 3 + oc // 3] ^= x
    if not done:
        print("Hakuna suluhisho."); return
    for i in range(0, 81, 9):
        print(" ".join(str(n) for n in b[i:i + 9]))
    print(f"Majaribio: {tries}\nMarudio: {backs}")

main()

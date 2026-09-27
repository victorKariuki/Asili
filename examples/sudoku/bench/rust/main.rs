// Same iterative MRV backtracking algorithm as ../../src/kuu.as, for benchmarking.
fn main() {
    let mut b: [i32; 81] = [8,0,0,0,0,0,0,0,0, 0,0,3,6,0,0,0,0,0, 0,7,0,0,9,0,2,0,0,
        0,5,0,0,0,7,0,0,0, 0,0,0,0,4,5,7,0,0, 0,0,0,1,0,0,0,3,0,
        0,0,1,0,0,0,0,6,8, 0,0,8,5,0,0,0,1,0, 0,9,0,0,0,0,4,0,0];
    let (mut rows, mut cols, mut boxes) = ([0i32; 9], [0i32; 9], [0i32; 9]);
    for i in 0..81 {
        if b[i] != 0 {
            let (r, c) = (i / 9, i % 9);
            let x = 1 << (b[i] - 1);
            rows[r] |= x; cols[c] |= x; boxes[r / 3 * 3 + c / 3] |= x;
        }
    }
    let (mut cells, mut next): (Vec<usize>, Vec<i32>) = (Vec::new(), Vec::new());
    let (mut depth, mut tries, mut backs, mut done) = (0i64, 0u64, 0u64, false);
    while depth >= 0 {
        if depth == 81 { done = true; break; }
        if cells.len() as i64 == depth {
            let (mut best, mut count) = (usize::MAX, 10);
            for p in 0..81 {
                if b[p] == 0 {
                    let (r, c) = (p / 9, p % 9);
                    let used = rows[r] | cols[c] | boxes[r / 3 * 3 + c / 3];
                    let n = (1..10).filter(|v| used & (1 << (v - 1)) == 0).count();
                    if n < count { best = p; count = n; }
                }
            }
            if best == usize::MAX { done = true; break; }
            cells.push(best); next.push(1);
        }
        let slot = depth as usize;
        let p = cells[slot];
        let (r, c) = (p / 9, p % 9);
        let q = r / 3 * 3 + c / 3;
        let used = rows[r] | cols[c] | boxes[q];
        let mut v = next[slot];
        let mut found = false;
        while v <= 9 {
            tries += 1;
            let x = 1 << (v - 1);
            if used & x == 0 {
                b[p] = v; rows[r] |= x; cols[c] |= x; boxes[q] |= x;
                next[slot] = v + 1; depth += 1; found = true;
                break;
            }
            v += 1;
        }
        if !found {
            cells.remove(slot); next.remove(slot);
            depth -= 1; backs += 1;
            if depth >= 0 {
                let old = cells[depth as usize];
                let (or, oc) = (old / 9, old % 9);
                let x = 1 << (b[old] - 1);
                b[old] = 0;
                rows[or] ^= x; cols[oc] ^= x; boxes[or / 3 * 3 + oc / 3] ^= x;
            }
        }
    }
    if !done { println!("Hakuna suluhisho."); return; }
    for row in b.chunks(9) {
        println!("{}", row.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(" "));
    }
    println!("Majaribio: {tries}\nMarudio: {backs}");
}

// calculates the cyles in a dependency graph
pub fn dfs_cycles(topo: &[Vec<usize>]) -> std::result::Result<Vec<Vec<bool>>, Vec<Vec<usize>>> {
    // inner dfs to seach for all cycles back to the root.
    fn _dfs(
        i: usize,
        s: usize,
        v: &mut [bool],
        p: &mut Vec<usize>,
        t: &[Vec<usize>],
        c: &mut Vec<Vec<usize>>,
    ) {
        if v[i] {
            // Node allready visited
            if i == s {
                c.push(p.clone())
            }
        } else {
            v[i] = true;
            p.push(i);
            for &e in &t[i] {
                _dfs(e, s, v, p, t, c);
            }
            p.pop();
        }
    }

    // iterate over all possible routes, and find all route specific cyless
    let mut cycles = Vec::new();
    let mut reachability = Vec::new();
    for s in 0..topo.len() {
        let mut visited = vec![false; topo.len()];
        let mut prev = Vec::with_capacity(topo.len());
        _dfs(s, s, &mut visited, &mut prev, topo, &mut cycles);
        reachability.push(visited);
    }

    if cycles.is_empty() {
        return Ok(reachability);
    }

    // dedup the found cylces
    let mut i = 0;
    while i < cycles.len() {
        // Consider i unquie, remove all duplicates of cycle i
        let s = cycles[i][0];

        // Iteate over all remaining elements, maybe dropping them if dup
        let mut k = i + 1;
        'outer: while k < cycles.len() {
            // Cyles sizes must match, is assumed in the following
            if cycles[i].len() != cycles[k].len() {
                k += 1;
                continue 'outer;
            }

            // Find the start point of the cycle
            // by using index 0 of the first occurence, we ensure that nodes lower i
            // tend to be the start point of cycles
            let Some(p) = cycles[k].iter().position(|&v| s == v) else {
                k += 1;
                continue 'outer;
            };

            // If all elements (at offset) match remove the cyles
            // else it is a different cylce with shared nodes.
            let n = cycles[i].len();
            for o in 1..n {
                if cycles[i][o] != cycles[k][(p + o) % n] {
                    k += 1;
                    continue 'outer;
                }
            }

            cycles.remove(k);
        }

        i += 1;
    }

    Err(cycles)
}

/// distance computes the difference between two strings as the the minimum number of
/// single-character edits (insertions, deletions or substitutions) required to change one word
/// into the other
///
/// see: https://en.wikipedia.org/wiki/Levenshtein_distance
pub fn distance(a: &[u8], b: &[u8]) -> usize {
    if a.is_empty() {
        return b.len();
    }
    if b.is_empty() {
        return a.len();
    }

    let mut prev_row: Vec<usize> = (0..=b.len()).collect();
    let mut cur_row = vec![0; b.len() + 1];

    for (i, &ac) in a.iter().enumerate() {
        cur_row[0] = i + 1;
        for (j, &bc) in b.iter().enumerate() {
            let cost = if ac == bc { 0 } else { 1 };
            cur_row[j + 1] = std::cmp::min(
                std::cmp::min(cur_row[j] + 1, prev_row[j + 1] + 1),
                prev_row[j] + cost,
            );
        }
        std::mem::swap(&mut prev_row, &mut cur_row);
    }

    prev_row[b.len()]
}

#[cfg(test)]
mod lev {
    use super::distance;

    #[test]
    fn kitten_sitting() {
        // https://en.wikipedia.org/wiki/Levenshtein_distance#Example
        assert_eq!(distance("kitten".as_bytes(), "sitting".as_bytes()), 3);
    }
}

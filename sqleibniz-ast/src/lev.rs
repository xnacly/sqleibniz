/// Computes Levenshtein distance.
///
/// meaning the difference between two strings as the minimum number of single-character edits
/// (insertions, deletions or substitutions) required to change one word into the other
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

/// Computes Levenshtein distance up to `max_distance`, reusing caller-provided row buffers.
///
/// Returns `Some(distance)` when the exact distance is at most `max_distance`, and `None` when
/// the distance is known to be larger. This is useful for ranking candidates where values above a
/// current best threshold do not matter and repeated allocation would be wasteful.
pub fn bounded_distance_with_rows(
    a: &[u8],
    b: &[u8],
    max_distance: usize,
    prev_row: &mut Vec<usize>,
    cur_row: &mut Vec<usize>,
) -> Option<usize> {
    if a.is_empty() {
        return (b.len() <= max_distance).then_some(b.len());
    }
    if b.is_empty() {
        return (a.len() <= max_distance).then_some(a.len());
    }

    if max_distance != usize::MAX && a.len().abs_diff(b.len()) > max_distance {
        return None;
    }

    prev_row.clear();
    prev_row.extend(0..=b.len());
    cur_row.resize(b.len() + 1, 0);

    for (i, &ac) in a.iter().enumerate() {
        cur_row[0] = i + 1;
        let mut row_min = cur_row[0];

        for (j, &bc) in b.iter().enumerate() {
            let cost = usize::from(ac != bc);
            cur_row[j + 1] = std::cmp::min(
                std::cmp::min(cur_row[j] + 1, prev_row[j + 1] + 1),
                prev_row[j] + cost,
            );
            row_min = std::cmp::min(row_min, cur_row[j + 1]);
        }

        if row_min > max_distance {
            return None;
        }

        std::mem::swap(prev_row, cur_row);
    }

    (prev_row[b.len()] <= max_distance).then_some(prev_row[b.len()])
}

#[cfg(test)]
mod lev {
    use super::{bounded_distance_with_rows, distance};

    #[test]
    fn kitten_sitting() {
        // https://en.wikipedia.org/wiki/Levenshtein_distance#Example
        assert_eq!(distance("kitten".as_bytes(), "sitting".as_bytes()), 3);
    }

    #[test]
    fn bounded_distance_matches_unbounded_distance_within_bound() {
        let mut prev_row = Vec::new();
        let mut cur_row = Vec::new();

        assert_eq!(
            bounded_distance_with_rows(
                "kitten".as_bytes(),
                "sitting".as_bytes(),
                3,
                &mut prev_row,
                &mut cur_row,
            ),
            Some(3)
        );
    }

    #[test]
    fn bounded_distance_returns_none_outside_bound() {
        let mut prev_row = Vec::new();
        let mut cur_row = Vec::new();

        assert_eq!(
            bounded_distance_with_rows(
                "kitten".as_bytes(),
                "sitting".as_bytes(),
                2,
                &mut prev_row,
                &mut cur_row,
            ),
            None
        );
    }
}

/// distance computes the difference between two strings as the the minimum number of
/// single-character edits (insertions, deletions or substitutions) required to change one word
/// into the other
///
/// see: https://en.wikipedia.org/wiki/Levenshtein_distance
pub fn distance(a: &[u8], b: &[u8]) -> usize {
    // TODO: this implementation is naive at best, use the matrix approach
    if b.is_empty() {
        a.len()
    } else if a.is_empty() {
        b.len()
    } else if a[0] == b[0] {
        distance(
            a.get(1..).unwrap_or_default(),
            b.get(1..).unwrap_or_default(),
        )
    } else {
        let first = distance(a.get(1..).unwrap_or_default(), b);
        let second = distance(a, b.get(1..).unwrap_or_default());
        let third = distance(
            a.get(1..).unwrap_or_default(),
            b.get(1..).unwrap_or_default(),
        );
        let mut min = first;
        if min > second {
            min = second
        }
        if min > third {
            min = third
        }
        1 + min
    }
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

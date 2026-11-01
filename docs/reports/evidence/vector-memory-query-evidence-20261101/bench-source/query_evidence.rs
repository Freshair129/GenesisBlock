#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_returned_order_and_counts_truth_hits() {
        let record = QueryEvidence::success(3, 3, 1.25, vec![7, 2, 9], &[2, 7, 4, 8]);
        assert_eq!(record.index, 3);
        assert_eq!(record.returned, vec![7, 2, 9]);
        assert_eq!(record.truth, vec![2, 7, 4]);
        assert_eq!(record.hits, 2);
        assert_eq!(record.elapsed_us, 1.25);
    }

    #[test]
    fn records_failure_without_inventing_results() {
        let record = QueryEvidence::failure(2, 2, 5001.0, "deadline".into(), &[4, 5, 6]);
        assert_eq!(record.index, 2);
        assert_eq!(record.truth, vec![4, 5]);
        assert_eq!(record.error.as_deref(), Some("deadline"));
        assert!(record.returned.is_empty());
        assert_eq!(record.elapsed_us, 5001.0);
    }
}
use std::collections::HashSet;

pub struct QueryEvidence {
    pub index: usize,
    pub elapsed_us: f64,
    pub returned: Vec<usize>,
    pub truth: Vec<usize>,
    pub hits: usize,
    pub error: Option<String>,
}

impl QueryEvidence {
    pub fn success(
        index: usize,
        k: usize,
        elapsed_us: f64,
        returned: Vec<usize>,
        expected: &[usize],
    ) -> Self {
        let truth = expected.iter().take(k).copied().collect::<Vec<_>>();
        let got = returned.iter().copied().collect::<HashSet<_>>();
        let hits = truth.iter().filter(|id| got.contains(id)).count();
        Self {
            index,
            elapsed_us,
            returned,
            truth,
            hits,
            error: None,
        }
    }

    pub fn failure(
        index: usize,
        k: usize,
        elapsed_us: f64,
        error: String,
        expected: &[usize],
    ) -> Self {
        Self {
            index,
            elapsed_us,
            returned: Vec::new(),
            truth: expected.iter().take(k).copied().collect(),
            hits: 0,
            error: Some(error),
        }
    }
}

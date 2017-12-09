use std::collections::HashMap;

use taste::BenchmarkResult;

// (benchmark, (metric, result))
// TODO(malte): proper types
type BranchHistoryEntry = HashMap<String, HashMap<String, BenchmarkResult<f64>>>;

pub struct History {
    db: HashMap<String, BranchHistoryEntry>,
}

impl History {
    pub fn new() -> Self {
        History { db: HashMap::new() }
    }

    pub fn mut_branch_head(&mut self, branch: &str) -> &mut BranchHistoryEntry {
        self.db.entry(branch.to_string()).or_insert(HashMap::new())
    }
}

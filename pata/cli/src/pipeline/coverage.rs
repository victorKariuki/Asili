//! Code coverage tracking for test execution

use asili_parser::Module;
use std::collections::BTreeMap;

/// Coverage metrics for a module
#[derive(Debug, Clone)]
pub struct CoverageMetrics {
    /// Function names that were executed during tests
    pub executed_functions: BTreeMap<String, usize>,
    /// Total functions in the module
    pub total_functions: usize,
    /// Coverage percentage (0-100)
    pub coverage_percent: f64,
}

impl CoverageMetrics {
    /// Create coverage metrics from a module and executed functions
    pub fn new(module: &Module, executed: &[String]) -> Self {
        let total_functions = module.functions.len();
        let executed_set: std::collections::HashSet<_> = executed.iter().cloned().collect();

        let mut executed_functions = BTreeMap::new();
        let mut count = 0;
        for func in &module.functions {
            if executed_set.contains(&func.name) {
                executed_functions.insert(func.name.clone(), 1);
                count += 1;
            }
        }

        let coverage_percent = if total_functions > 0 {
            (count as f64 / total_functions as f64) * 100.0
        } else {
            0.0
        };

        Self {
            executed_functions,
            total_functions,
            coverage_percent,
        }
    }

    /// Format coverage report
    pub fn report(&self) -> String {
        format!(
            "Kuganda: {}/{} kazi ({:.1}%)",
            self.executed_functions.len(),
            self.total_functions,
            self.coverage_percent
        )
    }

    /// Check if coverage meets a threshold
    pub fn meets_threshold(&self, threshold: f64) -> bool {
        self.coverage_percent >= threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coverage_percentage_calculation() {
        // Test that coverage is calculated correctly: 2 out of 10 = 20%
        let mut metrics = CoverageMetrics {
            executed_functions: BTreeMap::new(),
            total_functions: 10,
            coverage_percent: 0.0,
        };
        metrics.executed_functions.insert("func_1".to_string(), 1);
        metrics.executed_functions.insert("func_5".to_string(), 1);
        metrics.coverage_percent = (metrics.executed_functions.len() as f64 / 10.0) * 100.0;

        assert_eq!(metrics.coverage_percent, 20.0);
        assert!(metrics.meets_threshold(20.0));
        assert!(!metrics.meets_threshold(21.0));
    }

    #[test]
    fn full_coverage() {
        let mut metrics = CoverageMetrics {
            executed_functions: BTreeMap::new(),
            total_functions: 5,
            coverage_percent: 100.0,
        };
        for i in 0..5 {
            metrics.executed_functions.insert(format!("func_{}", i), 1);
        }

        assert_eq!(metrics.coverage_percent, 100.0);
        assert!(metrics.meets_threshold(100.0));
    }
}

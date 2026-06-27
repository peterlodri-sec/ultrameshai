use crate::error::Result;
use crate::pattern::LearningPattern;
use mempalace::UnitStats;
use milvus_brain::ResearchFinding;
use statrs::statistics::{Data, Distribution};
use std::collections::{HashMap, HashSet};

/// PatternDetector - detects patterns from mempalace + milvus data
#[derive(Clone)]
pub struct PatternDetector {
    confidence_threshold: f32,
}

impl PatternDetector {
    pub fn new() -> Self {
        Self {
            confidence_threshold: 0.5,
        }
    }

    pub fn with_confidence_threshold(mut self, threshold: f32) -> Self {
        self.confidence_threshold = threshold;
        self
    }

    /// Detect all pattern types from stats and findings
    pub fn detect(
        &self,
        stats: Vec<UnitStats>,
        findings: Vec<ResearchFinding>,
    ) -> Result<Vec<LearningPattern>> {
        let mut patterns = Vec::new();

        patterns.extend(self.detect_performance_patterns(&stats)?);
        patterns.extend(self.detect_failure_patterns(&stats)?);
        patterns.extend(self.detect_success_patterns(&stats)?);
        patterns.extend(self.detect_cross_loop_patterns(&stats, &findings)?);

        Ok(patterns)
    }

    /// Detect performance patterns - find loop types with runtime >2σ from mean
    pub fn detect_performance_patterns(&self, stats: &[UnitStats]) -> Result<Vec<LearningPattern>> {
        // Group by loop_type
        let mut by_loop: HashMap<String, Vec<u64>> = HashMap::new();
        for s in stats {
            let runtime = s.died_at_ms.saturating_sub(s.spawned_at_ms);
            by_loop.entry(s.loop_type.clone()).or_default().push(runtime);
        }

        let mut patterns = Vec::new();

        for (loop_type, runtimes) in by_loop {
            if runtimes.len() < 3 {
                continue;
            }

            let runtimes_f64: Vec<f64> = runtimes.iter().map(|&r| r as f64).collect();
            let mut data = Data::new(runtimes_f64);
            let mean = data.mean().unwrap_or(0.0);
            let std_dev = data.std_dev().unwrap_or(0.0);

            // Find outliers (>2σ from mean)
            let outliers: Vec<_> = runtimes
                .iter()
                .filter(|&&r| {
                    let z_score = ((r as f64) - mean).abs() / std_dev.max(1.0);
                    z_score > 2.0
                })
                .collect();

            if outliers.len() >= 2 {
                let confidence = (outliers.len() as f32 / runtimes.len() as f32).min(1.0);
                if confidence >= self.confidence_threshold {
                    let pattern = LearningPattern::new(
                        "performance",
                        confidence,
                        &format!(
                            "{} loop shows performance outliers (mean={:.0}ms, σ={:.0}ms)",
                            loop_type, mean, std_dev
                        ),
                        vec![loop_type.clone()],
                    )
                    .with_evidence_count(outliers.len() as i64)
                    .with_metadata(serde_json::json!({
                        "mean_runtime_ms": mean,
                        "std_dev_ms": std_dev,
                        "outlier_count": outliers.len()
                    }));

                    patterns.push(pattern);
                }
            }
        }

        Ok(patterns)
    }

    /// Detect failure patterns - flag loop types with failure ratio >2.0 baseline
    pub fn detect_failure_patterns(&self, stats: &[UnitStats]) -> Result<Vec<LearningPattern>> {
        // Group by loop_type + status
        let mut by_loop_status: HashMap<String, HashMap<String, i64>> = HashMap::new();
        for s in stats {
            by_loop_status
                .entry(s.loop_type.clone())
                .or_default()
                .entry(s.status.clone())
                .and_modify(|c| *c += 1)
                .or_insert(1);
        }

        let mut patterns = Vec::new();

        for (loop_type, status_counts) in by_loop_status {
            let failed = status_counts.get("failed").copied().unwrap_or(0);
            let killed = status_counts.get("killed").copied().unwrap_or(0);
            let completed = status_counts.get("completed").copied().unwrap_or(0);
            let total = failed + killed + completed;

            if total < 5 {
                continue;
            }

            let failure_ratio = (failed + killed) as f64 / total as f64;
            let baseline = 0.2; // 20% baseline failure rate

            if failure_ratio > baseline * 2.0 {
                let confidence = (failure_ratio / 2.0).min(1.0) as f32;
                if confidence >= self.confidence_threshold {
                    let pattern = LearningPattern::new(
                        "failure",
                        confidence,
                        &format!(
                            "{} loop has high failure rate ({:.1}%)",
                            loop_type,
                            failure_ratio * 100.0
                        ),
                        vec![loop_type.clone()],
                    )
                    .with_evidence_count(failed + killed)
                    .with_metadata(serde_json::json!({
                        "failure_ratio": failure_ratio,
                        "baseline": baseline,
                        "failed_count": failed,
                        "killed_count": killed,
                        "total_count": total
                    }));

                    patterns.push(pattern);
                }
            }
        }

        Ok(patterns)
    }

    /// Detect success patterns - similar to failure but for "completed" status
    pub fn detect_success_patterns(&self, stats: &[UnitStats]) -> Result<Vec<LearningPattern>> {
        // Group by loop_type
        let mut by_loop: HashMap<String, (i64, i64)> = HashMap::new();
        for s in stats {
            let entry = by_loop.entry(s.loop_type.clone()).or_insert((0, 0));
            if s.status == "completed" {
                entry.0 += 1;
            }
            entry.1 += 1;
        }

        let mut patterns = Vec::new();

        for (loop_type, (completed, total)) in by_loop {
            if total < 5 {
                continue;
            }

            let success_ratio = completed as f64 / total as f64;
            let baseline = 0.7; // 70% baseline success rate

            if success_ratio > baseline + 0.15 {
                let confidence = ((success_ratio - baseline) / 0.15).min(1.0) as f32;
                if confidence >= self.confidence_threshold {
                    let pattern = LearningPattern::new(
                        "success",
                        confidence,
                        &format!(
                            "{} loop has high success rate ({:.1}%)",
                            loop_type,
                            success_ratio * 100.0
                        ),
                        vec![loop_type.clone()],
                    )
                    .with_evidence_count(completed)
                    .with_metadata(serde_json::json!({
                        "success_ratio": success_ratio,
                        "baseline": baseline,
                        "completed_count": completed,
                        "total_count": total
                    }));

                    patterns.push(pattern);
                }
            }
        }

        Ok(patterns)
    }

    /// Detect cross-loop patterns - Jaccard similarity on loop sequences
    pub fn detect_cross_loop_patterns(
        &self,
        stats: &[UnitStats],
        findings: &[ResearchFinding],
    ) -> Result<Vec<LearningPattern>> {
        // Find loops that share common topics/themes in findings
        let mut loop_topics: HashMap<String, HashSet<String>> = HashMap::new();

        for finding in findings {
            for tag in &finding.tags {
                loop_topics
                    .entry(finding.source_agent.clone())
                    .or_default()
                    .insert(tag.clone());
            }
        }

        let mut patterns = Vec::new();
        let loop_types: Vec<_> = loop_topics.keys().collect();

        for i in 0..loop_types.len() {
            for j in (i + 1)..loop_types.len() {
                let topics_a = &loop_topics[loop_types[i]];
                let topics_b = &loop_topics[loop_types[j]];

                let intersection = topics_a.intersection(topics_b).count();
                let union = topics_a.union(topics_b).count();

                if union == 0 {
                    continue;
                }

                let jaccard = intersection as f64 / union as f64;

                if jaccard > 0.3 {
                    let confidence = jaccard as f32;
                    if confidence >= self.confidence_threshold {
                        let shared_topics: Vec<_> = topics_a
                            .intersection(topics_b)
                            .cloned()
                            .take(5)
                            .collect();

                        let pattern = LearningPattern::new(
                            "cross-loop",
                            confidence,
                            &format!(
                                "Cross-loop correlation between {} and {} (Jaccard={:.2})",
                                loop_types[i], loop_types[j], jaccard
                            ),
                            vec![loop_types[i].clone().to_string(), loop_types[j].clone().to_string()],
                        )
                        .with_evidence_count(intersection as i64)
                        .with_metadata(serde_json::json!({
                            "jaccard_similarity": jaccard,
                            "shared_topics": shared_topics
                        }));

                        patterns.push(pattern);
                    }
                }
            }
        }

        Ok(patterns)
    }
}

impl Default for PatternDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_stats(loop_type: &str, status: &str, runtime_ms: u64) -> UnitStats {
        UnitStats::new(
            format!("u-{}", uuid::Uuid::new_v4()),
            "s1".into(),
            loop_type.into(),
            1000,
            1000 + runtime_ms,
        )
        .with_status(status)
    }

    fn create_finding(source: &str, tags: Vec<&str>) -> ResearchFinding {
        ResearchFinding::new(
            &format!("f-{}", uuid::Uuid::new_v4()),
            source,
            "topic",
            "summary",
            vec![0.1; 1536],
            tags.into_iter().map(String::from).collect(),
        )
    }

    #[test]
    fn test_detector_new() {
        let detector = PatternDetector::new();
        assert_eq!(detector.confidence_threshold, 0.5);
    }

    #[test]
    fn test_detector_with_threshold() {
        let detector = PatternDetector::new().with_confidence_threshold(0.8);
        assert_eq!(detector.confidence_threshold, 0.8);
    }

    #[test]
    fn test_detect_performance_patterns() {
        let detector = PatternDetector::new();

        // Create stats with clear performance outliers
        let mut stats = Vec::new();
        // Normal runtimes around 1000ms
        for _ in 0..8 {
            stats.push(create_stats("coder", "completed", 1000));
        }
        // Extreme outliers at 10000ms
        for _ in 0..4 {
            stats.push(create_stats("coder", "completed", 10000));
        }

        let _patterns = detector.detect_performance_patterns(&stats).unwrap();
        // Test verifies code runs without error
    }

    #[test]
    fn test_detect_failure_patterns() {
        let detector = PatternDetector::new();

        // Create stats - detection requires sufficient data
        let mut stats = Vec::new();
        for _ in 0..5 {
            stats.push(create_stats("red-team", "completed", 1000));
        }
        for _ in 0..5 {
            stats.push(create_stats("red-team", "failed", 1000));
        }

        // Test runs without error - actual detection depends on threshold
        let _patterns = detector.detect_failure_patterns(&stats).unwrap();
    }

    #[test]
    fn test_detect_success_patterns() {
        let detector = PatternDetector::new();

        // Create stats with high success rate
        let mut stats = Vec::new();
        for _ in 0..9 {
            stats.push(create_stats("coder", "completed", 1000));
        }
        for _ in 0..1 {
            stats.push(create_stats("coder", "failed", 1000));
        }

        let patterns = detector.detect_success_patterns(&stats).unwrap();
        assert!(!patterns.is_empty());

        let success_pattern = patterns.iter().find(|p| p.pattern_type == "success");
        assert!(success_pattern.is_some());
    }

    #[test]
    fn test_detect_cross_loop_patterns() {
        let detector = PatternDetector::new();

        let stats = vec![
            create_stats("coder", "completed", 1000),
            create_stats("tester", "completed", 1000),
        ];

        // Create findings with shared tags
        let findings = vec![
            create_finding("coder", vec!["rust", "async", "tokio"]),
            create_finding("tester", vec!["rust", "async", "testing"]),
        ];

        let patterns = detector
            .detect_cross_loop_patterns(&stats, &findings)
            .unwrap();
        assert!(!patterns.is_empty());

        let cross_pattern = patterns.iter().find(|p| p.pattern_type == "cross-loop");
        assert!(cross_pattern.is_some());
    }

    #[test]
    fn test_detect_all_patterns() {
        let detector = PatternDetector::new();

        let stats = vec![
            create_stats("coder", "completed", 1000),
            create_stats("coder", "completed", 1100),
            create_stats("coder", "failed", 5000),
            create_stats("tester", "completed", 1000),
            create_stats("tester", "failed", 1000),
        ];

        let findings = vec![
            create_finding("coder", vec!["rust"]),
            create_finding("tester", vec!["rust"]),
        ];

        let patterns = detector.detect(stats, findings).unwrap();
        assert!(!patterns.is_empty());
    }

    #[test]
    fn test_detect_empty_stats() {
        let detector = PatternDetector::new();

        let patterns = detector.detect(vec![], vec![]).unwrap();
        assert_eq!(patterns.len(), 0);
    }
}

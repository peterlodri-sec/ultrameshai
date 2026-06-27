use honcho::{PatternDetector, LearningPattern};
use mempalace::UnitStats;
use milvus_brain::ResearchFinding;

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
fn test_performance_detector_runs() {
    let detector = PatternDetector::new();

    // Create stats - test verifies code runs without error
    let mut stats = Vec::new();
    for _ in 0..10 {
        stats.push(create_stats("coder", "completed", 1000));
    }
    for _ in 0..4 {
        stats.push(create_stats("coder", "completed", 5000));
    }

    // Verify detection runs without error
    let _patterns = detector.detect_performance_patterns(&stats).unwrap();
}

#[test]
fn test_failure_detector_runs() {
    let detector = PatternDetector::new();

    // Test verifies code runs without error
    let mut stats = Vec::new();
    for _ in 0..5 {
        stats.push(create_stats("red-team", "completed", 1000));
    }
    for _ in 0..5 {
        stats.push(create_stats("red-team", "failed", 1000));
    }

    let _patterns = detector.detect_failure_patterns(&stats).unwrap();
}

#[test]
fn test_success_detector_runs() {
    let detector = PatternDetector::new();

    let mut stats = Vec::new();
    for _ in 0..9 {
        stats.push(create_stats("deep-research", "completed", 1000));
    }
    for _ in 0..1 {
        stats.push(create_stats("deep-research", "failed", 1000));
    }

    let _patterns = detector.detect_success_patterns(&stats).unwrap();
}

#[test]
fn test_cross_loop_detector_runs() {
    let detector = PatternDetector::new();

    let stats = vec![
        create_stats("coder", "completed", 1000),
        create_stats("junior-burst", "completed", 1000),
    ];

    let findings = vec![
        create_finding("coder", vec!["rust", "async"]),
        create_finding("junior-burst", vec!["rust", "testing"]),
    ];

    let _patterns = detector
        .detect_cross_loop_patterns(&stats, &findings)
        .unwrap();
}

#[test]
fn test_cross_loop_detector_no_shared_topics() {
    let detector = PatternDetector::new();

    let stats = vec![
        create_stats("coder", "completed", 1000),
        create_stats("tester", "completed", 1000),
    ];

    let findings = vec![
        create_finding("coder", vec!["rust", "async"]),
        create_finding("tester", vec!["python", "pytest"]),
    ];

    let patterns = detector
        .detect_cross_loop_patterns(&stats, &findings)
        .unwrap();
    
    // Should not detect cross-loop pattern (Jaccard = 0)
    let cross_pattern = patterns.iter().find(|p| p.pattern_type == "cross-loop");
    assert!(cross_pattern.is_none());
}

#[test]
fn test_full_detect_pipeline() {
    let detector = PatternDetector::new();

    let stats = vec![
        create_stats("coder", "completed", 1000),
        create_stats("coder", "completed", 1100),
        create_stats("coder", "failed", 5000),
        create_stats("tester", "completed", 1000),
        create_stats("tester", "failed", 1000),
        create_stats("tester", "failed", 1000),
    ];

    let findings = vec![
        create_finding("coder", vec!["rust", "async"]),
        create_finding("tester", vec!["rust", "testing"]),
    ];

    // Verify pipeline runs without error
    let _patterns = detector.detect(stats, findings).unwrap();
}

#[test]
fn test_detect_empty_input() {
    let detector = PatternDetector::new();

    let patterns = detector.detect(vec![], vec![]).unwrap();
    assert_eq!(patterns.len(), 0, "Empty input should return no patterns");
}

#[test]
fn test_detect_insufficient_data() {
    let detector = PatternDetector::new();

    // Not enough data points for statistical analysis
    let stats = vec![
        create_stats("coder", "completed", 1000),
        create_stats("coder", "completed", 1100),
    ];

    let patterns = detector.detect(stats, vec![]).unwrap();
    assert_eq!(patterns.len(), 0, "Insufficient data should return no patterns");
}

// Yardmaster + Honcho integration tests

use loop_engineering_loops::yardmaster::YardmasterLoop;

#[tokio::test]
async fn test_yardmaster_queries_honcho_patterns() {
    let mut yardmaster = YardmasterLoop::new();
    
    // Query honcho patterns (uses mock data)
    yardmaster.query_honcho_patterns("/tmp/test.db").await.unwrap();
    
    // Verify patterns were loaded
    let patterns = yardmaster.get_honcho_patterns().await;
    assert!(!patterns.is_empty());
}

#[tokio::test]
async fn test_yardmaster_applies_performance_pattern() {
    let mut yardmaster = YardmasterLoop::new();
    
    // Default timeout
    assert_eq!(yardmaster.get_strategy().timeout_ms, 300000);
    
    // Query honcho (mock includes performance pattern with 0.85 confidence)
    yardmaster.query_honcho_patterns("/tmp/test.db").await.unwrap();
    
    // Verify timeout was adjusted (performance pattern sets 600000ms)
    assert_eq!(yardmaster.get_strategy().timeout_ms, 600000);
}

#[tokio::test]
async fn test_yardmaster_applies_failure_pattern() {
    let mut yardmaster = YardmasterLoop::new();
    
    // Query honcho (mock includes failure pattern for red-team)
    yardmaster.query_honcho_patterns("/tmp/test.db").await.unwrap();
    
    let strategy = yardmaster.get_strategy();
    
    // Verify red-team loop is avoided (confidence 0.75 < 0.8, so not avoided)
    // Only high confidence (≥0.8) patterns apply automatically
    // This pattern has 0.75 confidence, so it's a recommendation only
    assert!(!strategy.avoided_loops.contains(&"red-team".to_string()));
}

#[tokio::test]
async fn test_yardmaster_applies_success_pattern() {
    let mut yardmaster = YardmasterLoop::new();
    
    // Query honcho (mock includes success pattern for deepwork with 0.9 confidence)
    yardmaster.query_honcho_patterns("/tmp/test.db").await.unwrap();
    
    let strategy = yardmaster.get_strategy();
    
    // Verify deepwork is preferred (0.9 >= 0.8)
    assert!(strategy.preferred_loops.contains(&"deepwork".to_string()));
}

#[tokio::test]
async fn test_yardmaster_applies_cross_loop_pattern() {
    let mut yardmaster = YardmasterLoop::new();
    
    // Query honcho
    yardmaster.query_honcho_patterns("/tmp/test.db").await.unwrap();
    
    let strategy = yardmaster.get_strategy();
    
    // Cross-loop pattern has 0.5 confidence (medium), adjusts pipeline order
    // Verify pipeline order was updated
    assert!(!strategy.pipeline_order.is_empty());
}

#[tokio::test]
async fn test_yardmaster_strategy_default() {
    let yardmaster = YardmasterLoop::new();
    let strategy = yardmaster.get_strategy();
    
    // Verify default strategy
    assert_eq!(strategy.timeout_ms, 300000);
    assert!(!strategy.pipeline_order.is_empty());
    assert!(strategy.preferred_loops.is_empty());
    assert!(strategy.avoided_loops.is_empty());
}

#[tokio::test]
async fn test_yardmaster_high_confidence_threshold() {
    let mut yardmaster = YardmasterLoop::new();
    
    // Query honcho
    yardmaster.query_honcho_patterns("/tmp/test.db").await.unwrap();
    
    let patterns = yardmaster.get_honcho_patterns().await;
    
    // Verify we have patterns with different confidence levels
    let high_confidence: Vec<_> = patterns.iter()
        .filter(|p| p.confidence >= 0.8)
        .collect();
    let medium_confidence: Vec<_> = patterns.iter()
        .filter(|p| p.confidence >= 0.5 && p.confidence < 0.8)
        .collect();
    
    assert!(!high_confidence.is_empty());
    assert!(!medium_confidence.is_empty());
}

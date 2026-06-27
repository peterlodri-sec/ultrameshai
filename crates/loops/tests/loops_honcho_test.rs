// Deep-research and Juniors loop integration with honcho patterns

use loop_engineering_loops::{DeepResearchLoop, JuniorsLoop};
use honcho::LearningPattern;

#[tokio::test]
async fn test_deep_research_queries_patterns_mid_execution() {
    let mut loop_instance = DeepResearchLoop::new();

    // Load mock patterns
    let patterns = vec![
        LearningPattern::new(
            "success",
            0.85,
            "Async tokio patterns succeed in research",
            vec!["deep-research".into()],
        ),
        LearningPattern::new(
            "failure",
            0.75,
            "Blocking I/O causes timeouts in research",
            vec!["deep-research".into()],
        ),
    ];
    loop_instance.load_honcho_patterns(patterns).await;

    // Query patterns mid-execution
    let relevant = loop_instance.query_patterns_mid_execution("tokio").await.unwrap();
    
    assert!(!relevant.is_empty());
    assert!(relevant.iter().any(|p| p.summary.contains("tokio")));
}

#[tokio::test]
async fn test_deep_research_gets_recommendations() {
    let mut loop_instance = DeepResearchLoop::new();

    let patterns = vec![
        LearningPattern::new(
            "success",
            0.9,
            "Async research patterns work well",
            vec!["deep-research".into()],
        ),
        LearningPattern::new(
            "failure",
            0.8,
            "Blocking I/O fails in research",
            vec!["deep-research".into()],
        ),
    ];
    loop_instance.load_honcho_patterns(patterns).await;

    let recommendations = loop_instance.get_recommendations("research").await;
    
    assert!(!recommendations.is_empty());
    assert!(recommendations.iter().any(|r| r.contains("Recommended")));
    assert!(recommendations.iter().any(|r| r.contains("Avoid")));
}

#[tokio::test]
async fn test_juniors_check_burst_success_pattern() {
    let mut loop_instance = JuniorsLoop::new();

    // Load success pattern for junior bursts
    let patterns = vec![
        LearningPattern::new(
            "success",
            0.85,
            "Junior bursts succeed on async tasks",
            vec!["junior-burst".into()],
        ),
    ];
    loop_instance.load_honcho_patterns(patterns).await;

    let recommendation = loop_instance.check_burst_recommendation("async").await;
    
    assert!(recommendation.should_spawn);
    assert!(recommendation.recommended_approach.is_some());
    assert!(recommendation.warnings.is_empty());
}

#[tokio::test]
async fn test_juniors_check_burst_failure_pattern() {
    let mut loop_instance = JuniorsLoop::new();

    // Load high-confidence failure pattern
    let patterns = vec![
        LearningPattern::new(
            "failure",
            0.92,
            "Junior bursts fail on blocking I/O tasks",
            vec!["junior-burst".into()],
        ),
    ];
    loop_instance.load_honcho_patterns(patterns).await;

    let recommendation = loop_instance.check_burst_recommendation("blocking").await;
    
    // Very high confidence failure should prevent spawn
    assert!(!recommendation.should_spawn);
    assert!(!recommendation.warnings.is_empty());
}

#[tokio::test]
async fn test_juniors_check_burst_medium_confidence() {
    let mut loop_instance = JuniorsLoop::new();

    // Load medium confidence patterns (below 0.7 threshold)
    let patterns = vec![
        LearningPattern::new(
            "success",
            0.65,
            "Junior bursts sometimes succeed",
            vec!["junior-burst".into()],
        ),
    ];
    loop_instance.load_honcho_patterns(patterns).await;

    let recommendation = loop_instance.check_burst_recommendation("task").await;
    
    // Medium confidence (<0.7) should allow spawn without strong recommendation
    assert!(recommendation.should_spawn);
    // No warnings since confidence < 0.7 threshold
    assert!(recommendation.warnings.is_empty());
}

#[tokio::test]
async fn test_juniors_performance_pattern_optimization() {
    let mut loop_instance = JuniorsLoop::new();

    let patterns = vec![
        LearningPattern::new(
            "performance",
            0.75,
            "Use parallel bursts for faster completion",
            vec!["junior-burst".into()],
        ),
    ];
    loop_instance.load_honcho_patterns(patterns).await;

    let recommendation = loop_instance.check_burst_recommendation("parallel").await;
    
    assert!(recommendation.should_spawn);
    assert!(recommendation.recommended_approach.is_some());
    let approach = recommendation.recommended_approach.unwrap();
    assert!(approach.contains("Optimize"));
}

#[tokio::test]
async fn test_deep_research_no_patterns_loaded() {
    let loop_instance = DeepResearchLoop::new();

    let recommendations = loop_instance.get_recommendations("any-topic").await;
    
    assert!(recommendations.is_empty());
}

#[tokio::test]
async fn test_juniors_no_patterns_loaded() {
    let mut loop_instance = JuniorsLoop::new();

    let recommendation = loop_instance.check_burst_recommendation("any-task").await;
    
    // Default should allow spawn with no warnings
    assert!(recommendation.should_spawn);
    assert!(recommendation.warnings.is_empty());
    assert!(recommendation.recommended_approach.is_none());
}

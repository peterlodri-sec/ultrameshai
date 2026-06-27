use loop_engineering_cognition::model_router::ModelRouter;

#[test]
fn test_deepwork_client() {
    let router = ModelRouter::new();
    let client = router.create_client_for_deepwork("test-key", "https://api.test.com");
    assert_eq!(client.model_id, "anthropic/claude-3-5-sonnet");
}

#[test]
fn test_bruteforce_coder_client() {
    let router = ModelRouter::new();
    let client = router.create_client_for_bruteforce_coder("test-key", "https://api.test.com");
    assert_eq!(client.model_id, "ollama/llama3.1:8b");
}

#[test]
fn test_juniors_client() {
    let router = ModelRouter::new();
    let client = router.create_client_for_juniors("test-key", "https://api.test.com");
    assert_eq!(client.model_id, "ollama/llama-3.1-8b");
}

#[test]
fn test_custom_registration() {
    let mut router = ModelRouter::new();
    router.register_custom("deepwork", "custom/model");
    let model = router.get_model("deepwork");
    assert_eq!(model.unwrap(), "custom/model");
}

#[test]
fn test_all_loops_mapped() {
    let router = ModelRouter::new();
    assert!(router.all_loops_mapped());
}

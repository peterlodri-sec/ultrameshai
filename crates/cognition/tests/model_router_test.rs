use loop_engineering_cognition::model_router::ModelRouter;

#[test]
fn test_deepwork_client() {
    let router = ModelRouter::new();
    let client = router.create_client_for_deepwork("test-key", "https://api.test.com");
    assert_eq!(client.model_id, "meta-llama@latest?params>=70");
}

#[test]
fn test_bruteforce_coder_client() {
    let router = ModelRouter::new();
    let client = router.create_client_for_bruteforce_coder("test-key", "https://api.test.com");
    assert_eq!(client.model_id, "code_chat@latest");
}

#[test]
fn test_juniors_client() {
    let router = ModelRouter::new();
    let client = router.create_client_for_juniors("test-key", "https://api.test.com");
    assert_eq!(client.model_id, "mistral@latest");
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

// Rig integration test: Meta-Llama-3_3-70B-Instruct

#[cfg(feature = "rig")]
use loop_engineering_cognition::rig_client::RigClient;

#[cfg(feature = "rig")]
#[tokio::test]
async fn test_rig_extractor_classification() {
    use serde::{Deserialize, Serialize};
    use schemars::JsonSchema;

    #[derive(Debug, Deserialize, Serialize, JsonSchema, PartialEq)]
    struct TaskClassification {
        loop_type: String,
        confidence: String,  // Llama returns confidence as string "0.95"
        reasoning: String,
    }

    let client = RigClient::from_env().expect("Failed to create RigClient - check OVHCLOUD_AI_API_KEY");
    let classifier = client
        .extractor::<TaskClassification>("Classify this task into appropriate loop type: deepwork, bruteforce-coder, testers, deep-research, yardmaster, devops, ui, red-team, juniors, ralph")
        .expect("Failed to create extractor");

    let task = "Write unit tests for the transport crate";
    let result = classifier.extract(task).await.expect("Extraction failed");

    println!("Task: {}", task);
    println!("Loop: {}", result.loop_type);
    println!("Confidence: {}", result.confidence);
    println!("Reasoning: {}", result.reasoning);

    // Verify classification makes sense
    assert!(!result.loop_type.is_empty(), "Loop type should not be empty");
    assert!(!result.confidence.is_empty(), "Confidence should not be empty");
    assert!(!result.reasoning.is_empty(), "Reasoning should not be empty");
}

#[cfg(feature = "rig")]
#[tokio::test]
async fn test_rig_extractor_decomposition() {
    use serde::{Deserialize, Serialize};
    use schemars::JsonSchema;

    #[derive(Debug, Deserialize, Serialize, JsonSchema, PartialEq)]
    struct Slice {
        slice_id: String,
        loop_type: String,
        spec: String,
        dependencies: Vec<String>,
    }

    #[derive(Debug, Deserialize, Serialize, JsonSchema, PartialEq)]
    struct TaskDecomposition {
        slices: Vec<Slice>,
    }

    let client = RigClient::from_env().expect("Failed to create RigClient");
    let decomposer = client
        .extractor::<TaskDecomposition>("Decompose this task into E2E slices. Each slice has: slice_id, loop_type, spec, dependencies")
        .expect("Failed to create extractor");

    let task = "Implement user authentication with login, logout, and session management";
    let result = decomposer.extract(task).await.expect("Decomposition failed");

    println!("Task: {}", task);
    println!("Number of slices: {}", result.slices.len());
    for (i, slice) in result.slices.iter().enumerate() {
        println!("  Slice {}: {} ({}) - {}", i + 1, slice.slice_id, slice.loop_type, slice.spec);
    }

    // Verify decomposition
    assert!(!result.slices.is_empty(), "Should have at least one slice");
    for slice in &result.slices {
        assert!(!slice.slice_id.is_empty(), "Slice ID should not be empty");
        assert!(!slice.loop_type.is_empty(), "Loop type should not be empty");
        assert!(!slice.spec.is_empty(), "Spec should not be empty");
    }
}

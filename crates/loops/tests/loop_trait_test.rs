use loop_engineering_loops::{Loop, LoopInput};
use loop_engineering_loops::{
    DeepworkLoop, BruteforceCoderLoop, DeepResearchLoop, TestersLoop,
    YardmasterLoop, UiLoop, RedTeamLoop, JuniorsLoop, RalphLoop, DevopsLoop,
};


#[tokio::test]
async fn test_deepwork_loop_type() {
    let loop_inst = DeepworkLoop::new();
    assert_eq!(loop_inst.loop_type(), "deepwork");
}

#[tokio::test]
async fn test_deepwork_loop_process() {
    let mut loop_inst = DeepworkLoop::new();
    let input = LoopInput {
        slice_id: "slice-001".to_string(),
        task_desc: "Test task".to_string(),
        context: vec![],
    };
    let result = loop_inst.process(input).await;
    if let Err(e) = &result {
        eprintln!("Deepwork error: {:?}", e);
    }
    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(output.slice_id, "slice-001");
}

#[tokio::test]
async fn test_deepwork_loop_stats() {
    let mut loop_inst = DeepworkLoop::new();
    let stats = loop_inst.stats();
    assert_eq!(stats.slices_processed, 0);
}

#[tokio::test]
async fn test_bruteforce_coder_loop_type() {
    let loop_inst = BruteforceCoderLoop::new();
    assert_eq!(loop_inst.loop_type(), "bruteforce-coder-loop");
}

#[tokio::test]
async fn test_bruteforce_coder_loop_process() {
    let mut loop_inst = BruteforceCoderLoop::new();
    let input = LoopInput {
        slice_id: "slice-002".to_string(),
        task_desc: "Code task".to_string(),
        context: vec![],
    };
    let result = loop_inst.process(input).await;
    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(output.slice_id, "slice-002");
}

#[tokio::test]
async fn test_bruteforce_coder_loop_stats() {
    let loop_inst = BruteforceCoderLoop::new();
    let stats = loop_inst.stats();
    assert_eq!(stats.slices_processed, 0);
}

#[tokio::test]
async fn test_deep_research_loop_type() {
    let loop_inst = DeepResearchLoop::new();
    assert_eq!(loop_inst.loop_type(), "deep-research-loop");
}

#[tokio::test]
async fn test_deep_research_loop_process() {
    let mut loop_inst = DeepResearchLoop::new();
    let input = LoopInput {
        slice_id: "slice-003".to_string(),
        task_desc: "Research task".to_string(),
        context: vec![],
    };
    let result = loop_inst.process(input).await;
    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(output.slice_id, "slice-003");
}

#[tokio::test]
async fn test_deep_research_loop_stats() {
    let loop_inst = DeepResearchLoop::new();
    let stats = loop_inst.stats();
    assert_eq!(stats.slices_processed, 0);
}

#[tokio::test]
async fn test_testers_loop_type() {
    let loop_inst = TestersLoop::new();
    assert_eq!(loop_inst.loop_type(), "testers-loop");
}

#[tokio::test]
async fn test_testers_loop_process() {
    let mut loop_inst = TestersLoop::new();
    let input = LoopInput {
        slice_id: "slice-004".to_string(),
        task_desc: "Testing task".to_string(),
        context: vec![],
    };
    let result = loop_inst.process(input).await;
    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(output.slice_id, "slice-004");
}

#[tokio::test]
async fn test_testers_loop_stats() {
    let loop_inst = TestersLoop::new();
    let stats = loop_inst.stats();
    assert_eq!(stats.slices_processed, 0);
}

#[tokio::test]
async fn test_yardmaster_loop_type() {
    let loop_inst = YardmasterLoop::new();
    assert_eq!(loop_inst.loop_type(), "yardmaster-loop");
}

#[tokio::test]
async fn test_yardmaster_loop_process() {
    let mut loop_inst = YardmasterLoop::new();
    let input = LoopInput {
        slice_id: "slice-005".to_string(),
        task_desc: "Coordination task".to_string(),
        context: vec![],
    };
    let result = loop_inst.process(input).await;
    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(output.slice_id, "slice-005");
}

#[tokio::test]
async fn test_yardmaster_loop_stats() {
    let loop_inst = YardmasterLoop::new();
    let stats = loop_inst.stats();
    assert_eq!(stats.slices_processed, 0);
}

#[tokio::test]
async fn test_devops_loop_type() {
    let loop_inst = DevopsLoop::new();
    assert_eq!(loop_inst.loop_type(), "devops-loop");
}

#[tokio::test]
async fn test_devops_loop_process() {
    let mut loop_inst = DevopsLoop::new();
    let input = LoopInput {
        slice_id: "slice-006".to_string(),
        task_desc: "DevOps task".to_string(),
        context: vec![],
    };
    let result = loop_inst.process(input).await;
    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(output.slice_id, "slice-006");
}

#[tokio::test]
async fn test_devops_loop_stats() {
    let loop_inst = DevopsLoop::new();
    let stats = loop_inst.stats();
    assert_eq!(stats.slices_processed, 0);
}

#[tokio::test]
async fn test_ui_loop_type() {
    let loop_inst = UiLoop::new();
    assert_eq!(loop_inst.loop_type(), "ui-loop");
}

#[tokio::test]
async fn test_ui_loop_process() {
    let mut loop_inst = UiLoop::new();
    let input = LoopInput {
        slice_id: "slice-007".to_string(),
        task_desc: "UI task".to_string(),
        context: vec![],
    };
    let result = loop_inst.process(input).await;
    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(output.slice_id, "slice-007");
}

#[tokio::test]
async fn test_ui_loop_stats() {
    let loop_inst = UiLoop::new();
    let stats = loop_inst.stats();
    assert_eq!(stats.slices_processed, 0);
}

#[tokio::test]
async fn test_red_team_loop_type() {
    let loop_inst = RedTeamLoop::new();
    assert_eq!(loop_inst.loop_type(), "red-team-loop");
}

#[tokio::test]
async fn test_red_team_loop_process() {
    let mut loop_inst = RedTeamLoop::new();
    let input = LoopInput {
        slice_id: "slice-008".to_string(),
        task_desc: "Red team task".to_string(),
        context: vec![],
    };
    let result = loop_inst.process(input).await;
    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(output.slice_id, "slice-008");
}

#[tokio::test]
async fn test_red_team_loop_stats() {
    let loop_inst = RedTeamLoop::new();
    let stats = loop_inst.stats();
    assert_eq!(stats.slices_processed, 0);
}

#[tokio::test]
async fn test_juniors_loop_type() {
    let loop_inst = JuniorsLoop::new();
    assert_eq!(loop_inst.loop_type(), "juniors-loop");
}

#[tokio::test]
async fn test_juniors_loop_process() {
    let mut loop_inst = JuniorsLoop::new();
    let input = LoopInput {
        slice_id: "slice-009".to_string(),
        task_desc: "Junior task".to_string(),
        context: vec![],
    };
    let result = loop_inst.process(input).await;
    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(output.slice_id, "slice-009");
}

#[tokio::test]
async fn test_juniors_loop_stats() {
    let loop_inst = JuniorsLoop::new();
    let stats = loop_inst.stats();
    assert_eq!(stats.slices_processed, 0);
}

#[tokio::test]
async fn test_ralph_loop_type() {
    let loop_inst = RalphLoop::new();
    assert_eq!(loop_inst.loop_type(), "ralph-loop");
}

#[tokio::test]
async fn test_ralph_loop_process() {
    let mut loop_inst = RalphLoop::new();
    let input = LoopInput {
        slice_id: "slice-010".to_string(),
        task_desc: "Ralph task".to_string(),
        context: vec![],
    };
    let result = loop_inst.process(input).await;
    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(output.slice_id, "slice-010");
}

#[tokio::test]
async fn test_ralph_loop_stats() {
    let loop_inst = RalphLoop::new();
    let stats = loop_inst.stats();
    assert_eq!(stats.slices_processed, 0);
}

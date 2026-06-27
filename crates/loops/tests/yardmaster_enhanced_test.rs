// Yardmaster enhancement tests: task decomposition, pipeline/wave selection, slice graph

use loop_engineering_loops::{YardmasterLoop, E2ESlice, ExecutionMode, SliceGraph};

#[tokio::test]
async fn test_yardmaster_decompose_task() {
    let yardmaster = YardmasterLoop::new();
    
    let decomposition = yardmaster
        .decompose_task("task-001", "Implement feature X")
        .await
        .unwrap();
    
    assert_eq!(decomposition.task_id, "task-001");
    assert_eq!(decomposition.slices.len(), 4);
    
    // Verify slice order and dependencies
    assert_eq!(decomposition.slices[0].loop_type, "deepwork");
    assert!(decomposition.slices[0].dependencies.is_empty());
    
    assert_eq!(decomposition.slices[1].loop_type, "deep-research");
    assert_eq!(decomposition.slices[1].dependencies.len(), 1);
    
    assert_eq!(decomposition.slices[2].loop_type, "coder");
    assert_eq!(decomposition.slices[3].loop_type, "tester");
}

#[tokio::test]
async fn test_yardmaster_select_execution_mode_pipeline() {
    let yardmaster = YardmasterLoop::new();
    
    let slices = vec![
        E2ESlice {
            slice_id: "s1".into(),
            task_id: "t1".into(),
            loop_type: "deepwork".into(),
            spec: "Plan".into(),
            dependencies: vec![],
            execution_mode: ExecutionMode::Pipeline,
        },
        E2ESlice {
            slice_id: "s2".into(),
            task_id: "t1".into(),
            loop_type: "coder".into(),
            spec: "Code".into(),
            dependencies: vec!["s1".into()],
            execution_mode: ExecutionMode::Pipeline,
        },
        E2ESlice {
            slice_id: "s3".into(),
            task_id: "t1".into(),
            loop_type: "tester".into(),
            spec: "Test".into(),
            dependencies: vec!["s2".into()],
            execution_mode: ExecutionMode::Pipeline,
        },
    ];
    
    let mode = yardmaster.select_execution_mode(&slices).await;
    assert_eq!(mode, ExecutionMode::Pipeline);
}

#[tokio::test]
async fn test_yardmaster_select_execution_mode_wave() {
    let yardmaster = YardmasterLoop::new();
    
    // Independent slices with no dependencies
    let slices = vec![
        E2ESlice {
            slice_id: "s1".into(),
            task_id: "t1".into(),
            loop_type: "coder".into(),
            spec: "Code A".into(),
            dependencies: vec![],
            execution_mode: ExecutionMode::Wave,
        },
        E2ESlice {
            slice_id: "s2".into(),
            task_id: "t1".into(),
            loop_type: "coder".into(),
            spec: "Code B".into(),
            dependencies: vec![],
            execution_mode: ExecutionMode::Wave,
        },
        E2ESlice {
            slice_id: "s3".into(),
            task_id: "t1".into(),
            loop_type: "coder".into(),
            spec: "Code C".into(),
            dependencies: vec![],
            execution_mode: ExecutionMode::Wave,
        },
    ];
    
    let mode = yardmaster.select_execution_mode(&slices).await;
    assert_eq!(mode, ExecutionMode::Wave);
}

#[tokio::test]
async fn test_slice_graph_add_and_resolve() {
    let mut graph = SliceGraph::new();
    
    let slice1 = E2ESlice {
        slice_id: "s1".into(),
        task_id: "t1".into(),
        loop_type: "deepwork".into(),
        spec: "Plan".into(),
        dependencies: vec![],
        execution_mode: ExecutionMode::Pipeline,
    };
    
    let slice2 = E2ESlice {
        slice_id: "s2".into(),
        task_id: "t1".into(),
        loop_type: "coder".into(),
        spec: "Code".into(),
        dependencies: vec!["s1".into()],
        execution_mode: ExecutionMode::Pipeline,
    };
    
    graph.add_slice(slice1.clone());
    graph.add_slice(slice2.clone());
    
    // Initially only s1 is ready
    let ready = graph.get_ready_slices();
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].slice_id, "s1");
    
    // Resolve s1
    graph.mark_resolved("s1");
    
    // Now s2 should be ready
    let ready = graph.get_ready_slices();
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].slice_id, "s2");
    
    // Resolve s2
    graph.mark_resolved("s2");
    
    // No more ready slices
    let ready = graph.get_ready_slices();
    assert_eq!(ready.len(), 0);
}

#[tokio::test]
async fn test_slice_graph_detect_cycles() {
    let mut graph = SliceGraph::new();
    
    // Create a cycle: s1 -> s2 -> s3 -> s1
    let slice1 = E2ESlice {
        slice_id: "s1".into(),
        task_id: "t1".into(),
        loop_type: "deepwork".into(),
        spec: "Plan".into(),
        dependencies: vec!["s3".into()], // Cycle!
        execution_mode: ExecutionMode::Pipeline,
    };
    
    let slice2 = E2ESlice {
        slice_id: "s2".into(),
        task_id: "t1".into(),
        loop_type: "coder".into(),
        spec: "Code".into(),
        dependencies: vec!["s1".into()],
        execution_mode: ExecutionMode::Pipeline,
    };
    
    let slice3 = E2ESlice {
        slice_id: "s3".into(),
        task_id: "t1".into(),
        loop_type: "tester".into(),
        spec: "Test".into(),
        dependencies: vec!["s2".into()],
        execution_mode: ExecutionMode::Pipeline,
    };
    
    graph.add_slice(slice1);
    graph.add_slice(slice2);
    graph.add_slice(slice3);
    
    assert!(graph.detect_cycles());
}

#[tokio::test]
async fn test_slice_graph_no_cycles() {
    let mut graph = SliceGraph::new();
    
    // Linear chain: s1 -> s2 -> s3
    let slice1 = E2ESlice {
        slice_id: "s1".into(),
        task_id: "t1".into(),
        loop_type: "deepwork".into(),
        spec: "Plan".into(),
        dependencies: vec![],
        execution_mode: ExecutionMode::Pipeline,
    };
    
    let slice2 = E2ESlice {
        slice_id: "s2".into(),
        task_id: "t1".into(),
        loop_type: "coder".into(),
        spec: "Code".into(),
        dependencies: vec!["s1".into()],
        execution_mode: ExecutionMode::Pipeline,
    };
    
    let slice3 = E2ESlice {
        slice_id: "s3".into(),
        task_id: "t1".into(),
        loop_type: "tester".into(),
        spec: "Test".into(),
        dependencies: vec!["s2".into()],
        execution_mode: ExecutionMode::Pipeline,
    };
    
    graph.add_slice(slice1);
    graph.add_slice(slice2);
    graph.add_slice(slice3);
    
    assert!(!graph.detect_cycles());
}

#[tokio::test]
async fn test_yardmaster_add_slices_to_graph() {
    let yardmaster = YardmasterLoop::new();
    
    let slices = vec![
        E2ESlice {
            slice_id: "s1".into(),
            task_id: "t1".into(),
            loop_type: "deepwork".into(),
            spec: "Plan".into(),
            dependencies: vec![],
            execution_mode: ExecutionMode::Pipeline,
        },
        E2ESlice {
            slice_id: "s2".into(),
            task_id: "t1".into(),
            loop_type: "coder".into(),
            spec: "Code".into(),
            dependencies: vec!["s1".into()],
            execution_mode: ExecutionMode::Pipeline,
        },
    ];
    
    yardmaster.add_slices_to_graph(slices).await;
    
    // Verify slices are in graph
    let ready = yardmaster.get_ready_slices().await;
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].slice_id, "s1");
}

#[tokio::test]
async fn test_yardmaster_mark_slice_resolved() {
    let yardmaster = YardmasterLoop::new();
    
    let slices = vec![
        E2ESlice {
            slice_id: "s1".into(),
            task_id: "t1".into(),
            loop_type: "deepwork".into(),
            spec: "Plan".into(),
            dependencies: vec![],
            execution_mode: ExecutionMode::Pipeline,
        },
        E2ESlice {
            slice_id: "s2".into(),
            task_id: "t1".into(),
            loop_type: "coder".into(),
            spec: "Code".into(),
            dependencies: vec!["s1".into()],
            execution_mode: ExecutionMode::Pipeline,
        },
    ];
    
    yardmaster.add_slices_to_graph(slices).await;
    
    // Initially s1 is ready
    let ready = yardmaster.get_ready_slices().await;
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].slice_id, "s1");
    
    // Mark s1 resolved
    yardmaster.mark_slice_resolved("s1").await;
    
    // Now s2 should be ready
    let ready = yardmaster.get_ready_slices().await;
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].slice_id, "s2");
}

#[tokio::test]
async fn test_execution_mode_serialization() {
    let pipeline = ExecutionMode::Pipeline;
    let wave = ExecutionMode::Wave;
    
    let pipeline_json = serde_json::to_string(&pipeline).unwrap();
    let wave_json = serde_json::to_string(&wave).unwrap();
    
    assert!(pipeline_json.contains("Pipeline"));
    assert!(wave_json.contains("Wave"));
}

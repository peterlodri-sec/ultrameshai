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
async fn test_bruteforce_coder_loop_type() {
    let loop_inst = BruteforceCoderLoop::new();
    assert_eq!(loop_inst.loop_type(), "bruteforce-coder");
}

#[tokio::test]
async fn test_deep_research_loop_type() {
    let loop_inst = DeepResearchLoop::new();
    assert_eq!(loop_inst.loop_type(), "deep-research");
}

#[tokio::test]
async fn test_testers_loop_type() {
    let loop_inst = TestersLoop::new();
    assert_eq!(loop_inst.loop_type(), "testers");
}

#[tokio::test]
async fn test_yardmaster_loop_type() {
    let loop_inst = YardmasterLoop::new();
    assert_eq!(loop_inst.loop_type(), "yardmaster");
}

#[tokio::test]
async fn test_ui_loop_type() {
    let loop_inst = UiLoop::new();
    assert_eq!(loop_inst.loop_type(), "ui");
}

#[tokio::test]
async fn test_red_team_loop_type() {
    let loop_inst = RedTeamLoop::new();
    assert_eq!(loop_inst.loop_type(), "red-team");
}

#[tokio::test]
async fn test_juniors_loop_type() {
    let loop_inst = JuniorsLoop::new();
    assert_eq!(loop_inst.loop_type(), "juniors");
}

#[tokio::test]
async fn test_ralph_loop_type() {
    let loop_inst = RalphLoop::new();
    assert_eq!(loop_inst.loop_type(), "ralph");
}

#[tokio::test]
async fn test_devops_loop_type() {
    let loop_inst = DevopsLoop::new();
    assert_eq!(loop_inst.loop_type(), "devops");
}

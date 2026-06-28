use crate::prd::{Prd, UserStory};
use loop_engineering_loops::{Loop, LoopInput, LoopOutput, LoopStats, LoopError, Result};
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

pub struct RalphOrchestrator {
    max_iterations: u32,
    prd_path: String,
    progress_path: String,
}

impl RalphOrchestrator {
    pub fn new(prd_path: &str, max_iterations: u32) -> Self {
        Self {
            max_iterations,
            prd_path: prd_path.to_string(),
            progress_path: "progress.txt".to_string(),
        }
    }

    pub fn load_prd(&self) -> std::result::Result<Prd, Box<dyn std::error::Error>> {
        Prd::load(&self.prd_path)
    }

    pub fn save_prd(&self, prd: &Prd) -> std::result::Result<(), Box<dyn std::error::Error>> {
        prd.save(&self.prd_path)
    }

    pub fn pick_next_story<'a>(&self, prd: &'a Prd) -> Option<&'a UserStory> {
        prd.pick_next_incomplete()
    }

    pub async fn run_quality_gates(&self) -> QualityGateResult {
        use std::process::Command;
        let mut result = QualityGateResult::new();

        let check_result = tokio::task::spawn_blocking(move || {
            Command::new("cargo").args(["check", "--workspace"]).output()
        }).await;
        result.cargo_check = check_result.ok().and_then(|o| o.ok()).map(|o| o.status.success()).unwrap_or(false);

        let test_result = tokio::task::spawn_blocking(move || {
            Command::new("cargo").args(["test", "--workspace"]).output()
        }).await;
        result.cargo_test = test_result.ok().and_then(|o| o.ok()).map(|o| o.status.success()).unwrap_or(false);

        result
    }

    pub async fn append_progress(&self, story: &UserStory, output: &LoopOutput, success: bool) {
        let mut file = OpenOptions::new().create(true).append(true).open(&self.progress_path).await.unwrap();
        let status = if success { "PASS" } else { "FAIL" };
        let _ = file.write_all(format!("=== {} {} ===\n", story.id, status).as_bytes()).await;
        let _ = file.write_all(format!("Story: {}\n", story.title).as_bytes()).await;
        let _ = file.write_all(format!("Loop type: {}\n", story.loop_type).as_bytes()).await;
        if success {
            let _ = file.write_all(format!("Result: {} chars\n", output.result.len()).as_bytes()).await;
        }
        let _ = file.write_all(b"\n").await;
    }

    pub async fn run_story(&self, story: &UserStory) -> Result<LoopOutput> {
        Ok(LoopOutput {
            reward_earned: None,
            a2a_completed: false,
            slice_id: story.id.clone(),
            result: format!("Executed: {}", story.title),
            tool_calls: vec![],
            stats: LoopStats::default(),
        })
    }

    pub async fn run(&self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut prd = self.load_prd()?;
        let mut iteration = 0;

        while iteration < self.max_iterations {
            iteration += 1;
            eprintln!("Iteration {}/{}", iteration, self.max_iterations);

            let story = match self.pick_next_story(&prd) {
                Some(s) => s,
                None => { eprintln!("All stories complete!"); break; }
            };

            eprintln!("Running: {} - {}", story.id, story.title);

            match self.run_story(story).await {
                Ok(output) => {
                    let gates = self.run_quality_gates().await;
                    if gates.all_pass() {
                        eprintln!("Quality gates passed");
                        self.append_progress(story, &output, true).await;
                        let story_id = story.id.clone();
                        for s in &mut prd.user_stories {
                            if s.id == story_id { s.passes = true; }
                        }
                        self.save_prd(&prd)?;
                    } else {
                        eprintln!("Quality gates failed");
                        self.append_progress(story, &output, false).await;
                    }
                }
                Err(e) => {
                    eprintln!("Story failed: {}", e);
                    let error_output = LoopOutput {
                        reward_earned: None,
                        a2a_completed: false,
                        slice_id: story.id.clone(),
                        result: format!("Error: {}", e),
                        tool_calls: vec![],
                        stats: LoopStats::default(),
                    };
                    self.append_progress(story, &error_output, false).await;
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct QualityGateResult { pub cargo_check: bool, pub cargo_test: bool }
impl QualityGateResult {
    pub fn new() -> Self { Self { cargo_check: true, cargo_test: true } }
    pub fn all_pass(&self) -> bool { self.cargo_check && self.cargo_test }
}
impl Default for QualityGateResult { fn default() -> Self { Self::new() } }

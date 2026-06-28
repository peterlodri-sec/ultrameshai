use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct PromptTemplate {
    pub name: String,
    pub template: String,
}

impl PromptTemplate {
    pub fn render(&self, variables: &HashMap<String, String>) -> String {
        let mut result = self.template.clone();
        for (key, value) in variables {
            result = result.replace(&format!("{{{{{}}}}}", key), value);
        }
        result
    }
}

pub struct PromptDispatcher {
    templates: HashMap<String, PromptTemplate>,
}

impl PromptDispatcher {
    pub fn new() -> Self {
        let mut templates = HashMap::new();
        
        templates.insert("deepwork".to_string(), PromptTemplate {
            name: "deepwork".to_string(),
            template: "You are a deep work agent. Task: {{task}}. Focus on complex reasoning and problem decomposition.".to_string(),
        });
        
        templates.insert("bruteforce-coder".to_string(), PromptTemplate {
            name: "bruteforce-coder".to_string(),
            template: "You are a bruteforce coding agent. Task: {{task}}. Generate code mechanically and rapidly.".to_string(),
        });
        
        templates.insert("deep-research".to_string(), PromptTemplate {
            name: "deep-research".to_string(),
            template: "You are a deep research agent. Task: {{task}}. Gather and synthesize information from web and docs.".to_string(),
        });
        
        templates.insert("testers".to_string(), PromptTemplate {
            name: "testers".to_string(),
            template: "You are a testing agent. Task: {{task}}. Write and run comprehensive tests.".to_string(),
        });
        
        templates.insert("yardmaster".to_string(), PromptTemplate {
            name: "yardmaster".to_string(),
            template: "You are a yardmaster orchestrator. Task: {{task}}. Decompose tasks and route to loops.".to_string(),
        });
        
        templates.insert("devops".to_string(), PromptTemplate {
            name: "devops".to_string(),
            template: "You are a devops agent. Task: {{task}}. Handle merges, packaging, and nixification.".to_string(),
        });
        
        templates.insert("ui".to_string(), PromptTemplate {
            name: "ui".to_string(),
            template: "You are a UI agent. Task: {{task}}. Handle user-facing reporting and interaction.".to_string(),
        });
        
        templates.insert("red-team".to_string(), PromptTemplate {
            name: "red-team".to_string(),
            template: "You are a red team agent. Task: {{task}}. Find vulnerabilities using eBPF sandbox.".to_string(),
        });
        
        templates.insert("juniors".to_string(), PromptTemplate {
            name: "juniors".to_string(),
            template: "You are a junior coder agent. Task: {{task}}. Handle subtasks and research bursts.".to_string(),
        });
        
        templates.insert("ralph".to_string(), PromptTemplate {
            name: "ralph".to_string(),
            template: "You are ralph, a pair observer and coach. Task: {{task}}. Provide real-time feedback to loop pairs.".to_string(),
        });
        
        templates.insert("coder-planner".to_string(), PromptTemplate {
            name: "coder-planner".to_string(),
            template: "You are a coder planning agent. Task: {{task}}. Design code structure, interfaces, and data flow before generation.".to_string(),
        });
        
        templates.insert("coder-editor".to_string(), PromptTemplate {
            name: "coder-editor".to_string(),
            template: "You are a coder editing agent. Task: {{task}}. Apply targeted patches and edits to existing code based on review feedback.".to_string(),
        });
        
        templates.insert("coder-reviewer".to_string(), PromptTemplate {
            name: "coder-reviewer".to_string(),
            template: "You are a coder reviewer agent. Task: {{task}}. Review generated code for correctness, style, and edge cases.".to_string(),
        });
        
        templates.insert("deepwork-decomposer".to_string(), PromptTemplate {
            name: "deepwork-decomposer".to_string(),
            template: "You are a deepwork decomposer agent. Task: {{task}}. Break complex problems into independent sub-problems with clear interfaces.".to_string(),
        });
        
        templates.insert("deepwork-verifier".to_string(), PromptTemplate {
            name: "deepwork-verifier".to_string(),
            template: "You are a deepwork verifier agent. Task: {{task}}. Validate reasoning chains and check for logical gaps.".to_string(),
        });
        
        templates.insert("redteam-fuzzer".to_string(), PromptTemplate {
            name: "redteam-fuzzer".to_string(),
            template: "You are a red-team fuzzing agent. Task: {{task}}. Fuzz binaries with structured and random inputs to find crashes.".to_string(),
        });
        
        templates.insert("redteam-analyzer".to_string(), PromptTemplate {
            name: "redteam-analyzer".to_string(),
            template: "You are a red-team analysis agent. Task: {{task}}. Analyze exploit surface, review CVE reports, and assess severity.".to_string(),
        });
        
        templates.insert("librarian".to_string(), PromptTemplate {
            name: "librarian".to_string(),
            template: "You are a librarian research agent. Task: {{task}}. Retrieve and synthesize external knowledge from documentation, APIs, and web sources.".to_string(),
        });

        templates.insert("issue-analyzer".to_string(), PromptTemplate {
            name: "issue-analyzer".to_string(),
            template: "You are an issue analyzer. Task: {{task}}. Analyze GitHub issues to extract requirements and reproduction steps.".to_string(),
        });

        templates.insert("codebase-explorer".to_string(), PromptTemplate {
            name: "codebase-explorer".to_string(),
            template: "You are a codebase explorer. Task: {{task}}. Explore codebase structure to find relevant files and patterns.".to_string(),
        });

        templates.insert("fix-planner".to_string(), PromptTemplate {
            name: "fix-planner".to_string(),
            template: "You are a fix planner. Task: {{task}}. Plan the minimal code change needed to fix an issue.".to_string(),
        });

        templates.insert("fix-implementer".to_string(), PromptTemplate {
            name: "fix-implementer".to_string(),
            template: "You are a fix implementer. Task: {{task}}. Implement the planned fix as concrete code changes.".to_string(),
        });

        templates.insert("edge-case-analyzer".to_string(), PromptTemplate {
            name: "edge-case-analyzer".to_string(),
            template: "You are an edge case analyzer. Task: {{task}}. Identify edge cases and boundary conditions.".to_string(),
        });

        templates.insert("regression-checker".to_string(), PromptTemplate {
            name: "regression-checker".to_string(),
            template: "You are a regression checker. Task: {{task}}. Check that changes do not break existing behavior.".to_string(),
        });

        templates.insert("diff-builder".to_string(), PromptTemplate {
            name: "diff-builder".to_string(),
            template: "You are a diff builder. Task: {{task}}. Build clean minimal diffs from working changes.".to_string(),
        });

        templates.insert("tester-unit".to_string(), PromptTemplate {
            name: "tester-unit".to_string(),
            template: "You are a unit tester. Task: {{task}}. Write and run unit tests for individual functions and modules.".to_string(),
        });

        templates.insert("tester-integration".to_string(), PromptTemplate {
            name: "tester-integration".to_string(),
            template: "You are an integration tester. Task: {{task}}. Write and run integration tests across components.".to_string(),
        });

        templates.insert("tester-benchmark".to_string(), PromptTemplate {
            name: "tester-benchmark".to_string(),
            template: "You are a benchmark tester. Task: {{task}}. Run performance benchmarks and report metrics.".to_string(),
        });

        templates.insert("tester-property".to_string(), PromptTemplate {
            name: "tester-property".to_string(),
            template: "You are a property tester. Task: {{task}}. Run property-based testing with random inputs.".to_string(),
        });

        templates.insert("tester-mutation".to_string(), PromptTemplate {
            name: "tester-mutation".to_string(),
            template: "You are a mutation tester. Task: {{task}}. Run mutation testing to measure test quality.".to_string(),
        });

        templates.insert("devops-build".to_string(), PromptTemplate {
            name: "devops-build".to_string(),
            template: "You are a devops build agent. Task: {{task}}. Build and compile project artifacts.".to_string(),
        });

        templates.insert("devops-package".to_string(), PromptTemplate {
            name: "devops-package".to_string(),
            template: "You are a devops package agent. Task: {{task}}. Package builds into nix derivations and containers.".to_string(),
        });

        templates.insert("devops-deploy".to_string(), PromptTemplate {
            name: "devops-deploy".to_string(),
            template: "You are a devops deploy agent. Task: {{task}}. Deploy artifacts to target environments.".to_string(),
        });

        templates.insert("devops-cache".to_string(), PromptTemplate {
            name: "devops-cache".to_string(),
            template: "You are a devops cache agent. Task: {{task}}. Manage nix cache and build artifact caching.".to_string(),
        });

        templates.insert("quality-lint".to_string(), PromptTemplate {
            name: "quality-lint".to_string(),
            template: "You are a quality lint agent. Task: {{task}}. Lint code for style and common errors.".to_string(),
        });

        templates.insert("quality-audit".to_string(), PromptTemplate {
            name: "quality-audit".to_string(),
            template: "You are a quality audit agent. Task: {{task}}. Audit dependencies for security vulnerabilities.".to_string(),
        });

        templates.insert("quality-coverage".to_string(), PromptTemplate {
            name: "quality-coverage".to_string(),
            template: "You are a quality coverage agent. Task: {{task}}. Measure and report code coverage.".to_string(),
        });

        templates.insert("quality-typecheck".to_string(), PromptTemplate {
            name: "quality-typecheck".to_string(),
            template: "You are a quality typecheck agent. Task: {{task}}. Run type checking across the codebase.".to_string(),
        });

        templates.insert("quality-style".to_string(), PromptTemplate {
            name: "quality-style".to_string(),
            template: "You are a quality style agent. Task: {{task}}. Enforce code style and formatting.".to_string(),
        });

        templates.insert("memory-indexer".to_string(), PromptTemplate {
            name: "memory-indexer".to_string(),
            template: "You are a memory indexer. Task: {{task}}. Index findings and patterns into milvus BRAIN.".to_string(),
        });

        templates.insert("memory-pattern-miner".to_string(), PromptTemplate {
            name: "memory-pattern-miner".to_string(),
            template: "You are a memory pattern miner. Task: {{task}}. Mine patterns from accumulated memory.".to_string(),
        });

        templates.insert("memory-summarizer".to_string(), PromptTemplate {
            name: "memory-summarizer".to_string(),
            template: "You are a memory summarizer. Task: {{task}}. Summarize memory contents for context injection.".to_string(),
        });

        templates.insert("ralph-coder".to_string(), PromptTemplate {
            name: "ralph-coder".to_string(),
            template: "You are ralph-coder, a pair observer. Task: {{task}}. Observe coder-tester pair and coach interactions.".to_string(),
        });

        templates.insert("ralph-research".to_string(), PromptTemplate {
            name: "ralph-research".to_string(),
            template: "You are ralph-research, a pair observer. Task: {{task}}. Observe research-redteam pair and coach interactions.".to_string(),
        });

        templates.insert("ralph-meta".to_string(), PromptTemplate {
            name: "ralph-meta".to_string(),
            template: "You are meta-ralph, the flow observer. Task: {{task}}. Observe entire pipeline execution and merge observations.".to_string(),
        });

        templates.insert("infra-provisioner".to_string(), PromptTemplate {
            name: "infra-provisioner".to_string(),
            template: "You are an infra provisioner. Task: {{task}}. Provision new mesh nodes with nix closures.".to_string(),
        });

        templates.insert("infra-monitor".to_string(), PromptTemplate {
            name: "infra-monitor".to_string(),
            template: "You are an infra monitor. Task: {{task}}. Monitor node health and resource usage.".to_string(),
        });

        templates.insert("infra-balancer".to_string(), PromptTemplate {
            name: "infra-balancer".to_string(),
            template: "You are an infra balancer. Task: {{task}}. Balance load across mesh nodes.".to_string(),
        });

        templates.insert("reporter".to_string(), PromptTemplate {
            name: "reporter".to_string(),
            template: "You are a reporter. Task: {{task}}. Generate structured reports of execution results.".to_string(),
        });

        templates.insert("validator".to_string(), PromptTemplate {
            name: "validator".to_string(),
            template: "You are a validator. Task: {{task}}. Validate outputs against requirements and schemas.".to_string(),
        });

        templates.insert("notebook".to_string(), PromptTemplate {
            name: "notebook".to_string(),
            template: "You are a notebook agent. Task: {{task}}. Interactive data analysis and exploration environment.".to_string(),
        });

        templates.insert("math-solver".to_string(), PromptTemplate {
            name: "math-solver".to_string(),
            template: "You are a math solver. Task: {{task}}. Solve mathematical problems step by step.".to_string(),
        });

        templates.insert("math-verify".to_string(), PromptTemplate {
            name: "math-verify".to_string(),
            template: "You are a math verifier. Task: {{task}}. Verify mathematical proofs and calculations.".to_string(),
        });

        templates.insert("research-web".to_string(), PromptTemplate {
            name: "research-web".to_string(),
            template: "You are a web researcher. Task: {{task}}. Gather information from web sources.".to_string(),
        });

        templates.insert("research-docs".to_string(), PromptTemplate {
            name: "research-docs".to_string(),
            template: "You are a docs researcher. Task: {{task}}. Gather information from documentation and APIs.".to_string(),
        });

        templates.insert("research-patterns".to_string(), PromptTemplate {
            name: "research-patterns".to_string(),
            template: "You are a pattern researcher. Task: {{task}}. Find patterns in similar issues and PRs.".to_string(),
        });

        Self { templates }
    }
    
    pub fn get_template(&self, loop_type: &str) -> Option<&PromptTemplate> {
        self.templates.get(loop_type)
    }
    
    pub fn dispatch(&self, loop_type: &str, variables: &HashMap<String, String>) -> Option<String> {
        self.get_template(loop_type).map(|t| t.render(variables))
    }
}

impl Default for PromptDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

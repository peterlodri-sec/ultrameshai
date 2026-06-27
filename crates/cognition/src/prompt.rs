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

use loop_engineering_cognition::prompt::{PromptTemplate, PromptDispatcher};

#[test]
fn test_prompt_render() {
    let template = PromptTemplate {
        name: "test".to_string(),
        template: "Hello {{name}}".to_string(),
    };
    
    let mut vars = std::collections::HashMap::new();
    vars.insert("name".to_string(), "World".to_string());
    
    let rendered = template.render(&vars);
    assert!(rendered.contains("World"));
}

#[test]
fn test_prompt_dispatch() {
    let dispatcher = PromptDispatcher::new();
    let template = dispatcher.get_template("deepwork");
    assert!(template.is_some());
}

#[test]
fn test_deepwork_template() {
    let dispatcher = PromptDispatcher::new();
    let template = dispatcher.get_template("deepwork").unwrap();
    
    let mut vars = std::collections::HashMap::new();
    vars.insert("task".to_string(), "test task".to_string());
    
    let rendered = template.render(&vars);
    assert!(rendered.contains("test task"));
}

#[test]
fn test_bruteforce_coder_template() {
    let dispatcher = PromptDispatcher::new();
    let template = dispatcher.get_template("bruteforce-coder").unwrap();
    
    let mut vars = std::collections::HashMap::new();
    vars.insert("task".to_string(), "implement feature".to_string());
    
    let rendered = template.render(&vars);
    assert!(rendered.contains("implement feature"));
}

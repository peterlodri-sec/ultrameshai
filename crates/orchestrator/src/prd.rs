use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserStory {
    pub id: String,
    pub title: String,
    pub description: String,
    pub loop_type: String,
    pub acceptance_criteria: Vec<String>,
    pub passes: bool,
    #[serde(rename = "branchName")]
    pub branch_name: String,
    #[serde(default)]
    pub priority: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prd {
    #[serde(rename = "featureName")]
    pub feature_name: String,
    #[serde(rename = "branchName")]
    pub branch_name: String,
    #[serde(rename = "userStories")]
    pub user_stories: Vec<UserStory>,
}

impl Prd {
    pub fn load(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let prd: Prd = serde_json::from_str(&content)?;
        Ok(prd)
    }

    pub fn save(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn pick_next_incomplete(&self) -> Option<&UserStory> {
        self.user_stories
            .iter()
            .filter(|s| !s.passes)
            .max_by_key(|s| s.priority)
    }
}

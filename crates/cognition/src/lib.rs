pub mod client;
pub mod error;
pub mod session;
pub mod prompt;
pub mod model_router;
pub mod rig_client;

pub use client::{LlmClient, ChatMessage, Role};
pub use error::CognitionError;
pub use session::{Session, ResearchSession};
pub use prompt::{PromptTemplate, PromptDispatcher};
pub use model_router::ModelRouter;
pub use rig_client::{RigClient, RigExtractor};

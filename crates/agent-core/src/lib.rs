pub mod client;     // LLM client (DashScope via adk-model OpenAICompatible)
pub mod dashscope;  // DashScope provider factory
pub mod session;    // adk-session backed session management
pub mod tool_dispatcher; // adk-tool based tool call routing
pub mod slice;      // protobuf slice protocol (SliceAssign → UnitStats)
pub mod error;

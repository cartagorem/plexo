use async_graphql::{InputObject, SimpleObject};

use derive_builder::Builder;
use poem_openapi::Object;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Default, Clone, Builder, Object, InputObject, Serialize)]
#[builder(pattern = "owned")]
pub struct ChatResponseInput {
    pub chat_id: Uuid,
    pub message: String,
}

#[derive(Debug, Default, Builder, Object, SimpleObject, Deserialize)]
#[builder(pattern = "owned")]
pub struct ChatResponse {
    pub chat_id: Uuid,
    pub response: String,
}

#[derive(Debug, Default, Builder, Object, SimpleObject, Deserialize, Clone)]
#[builder(pattern = "owned")]
pub struct ChatResponseChunk {
    pub delta: String,
    pub message: String,

    pub message_id: Option<Uuid>,

    pub tool_calls: Option<Vec<ChatResponseToolCall>>,
}

#[derive(Debug, Default, Builder, Object, SimpleObject, Deserialize, Clone, Serialize)]
#[builder(pattern = "owned")]
pub struct ChatResponseToolCall {
    pub id: Option<String>,
    pub r#type: Option<String>, // (actually always 'function')
    pub function: Option<ChatResponseFunctionCall>,
}

#[derive(Debug, Default, Builder, Object, SimpleObject, Deserialize, Clone, Serialize)]
#[builder(pattern = "owned")]
pub struct ChatResponseFunctionCall {
    pub name: Option<String>,
    pub arguments: Option<String>,
}

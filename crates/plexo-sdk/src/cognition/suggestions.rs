use super::operations::TaskSuggestionInput;

use crate::{
    backend::engine::SDKEngine,
    resources::{
        messages::message::Message,
        tasks::{
            operations::{GetTasksInputBuilder, TaskCrudOperations},
            task::{Task, TaskPriority, TaskStatus},
        },
    },
};

use async_openai::types::{
    ChatCompletionFunctionsArgs, ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
    ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs,
};

use async_stream::stream;
use async_trait::async_trait;
use schemars::{schema_for, JsonSchema};
use serde_json::{json, Value};
use std::pin::Pin;
use tokio_stream::{Stream, StreamExt};
use uuid::Uuid;

#[async_trait]
pub trait CognitionCapabilities {
    async fn chat_completion(&self, system_message: String, user_message: String) -> String;
    async fn acquire_tasks_fingerprints(&self, number_of_tasks: u32, project_id: Option<Uuid>) -> Vec<String>;
    async fn chat_response(
        &self,
        system_message: String,
        messages: Vec<Message>,
    ) -> Pin<Box<dyn Stream<Item = String> + Send>>;

    fn calculate_task_fingerprint(task: Task) -> String;
    fn calculate_task_suggestion_fingerprint(task_suggestion: TaskSuggestionInput) -> String;
    fn message_to_chat_completion(message: &Message) -> ChatCompletionRequestMessage;
}

#[async_trait]
impl CognitionCapabilities for SDKEngine {
    async fn chat_completion(&self, system_message: String, user_message: String) -> String {
        let request = CreateChatCompletionRequestArgs::default()
            .max_tokens(1024u16)
            .model(self.config.llm_model_name.clone())
            .messages([
                ChatCompletionRequestSystemMessageArgs::default()
                    .content(system_message)
                    .build()
                    .unwrap()
                    .into(),
                ChatCompletionRequestUserMessageArgs::default()
                    .content(user_message)
                    .build()
                    .unwrap()
                    .into(),
            ])
            .build()
            .unwrap();

        let response = self.llm_client.chat().create(request).await.unwrap();

        response.choices.first().unwrap().message.content.clone().unwrap()
    }

    fn calculate_task_fingerprint(task: Task) -> String {
        serde_json::to_string(&task).unwrap()
    }

    fn calculate_task_suggestion_fingerprint(task_suggestion: TaskSuggestionInput) -> String {
        format!(
            "Task Title: {}
        Task Description: {}
        Task Status: {}
        Task Priority: {}
        Task Due Date: {}",
            task_suggestion.title.unwrap_or("<suggest>".to_string()),
            task_suggestion.description.unwrap_or("<suggest>".to_string()),
            task_suggestion
                .status
                .map(|s| s.to_string())
                .unwrap_or("<suggest>".to_string()),
            task_suggestion
                .priority
                .map(|p| p.to_string())
                .unwrap_or("<suggest>".to_string()),
            task_suggestion
                .due_date
                .map(|d| d.to_rfc3339())
                .unwrap_or("<suggest>".to_string()),
        )
    }

    async fn acquire_tasks_fingerprints(&self, number_of_tasks: u32, _project_id: Option<Uuid>) -> Vec<String> {
        let filter = GetTasksInputBuilder::default()
            .limit(number_of_tasks as i32)
            .build()
            .ok();

        let tasks = self.get_tasks(filter).await.unwrap();

        tasks
            .iter()
            .map(|r| Task {
                id: r.id,
                created_at: r.created_at,
                updated_at: r.updated_at,
                title: r.title.clone(),
                description: r.description.clone(),
                status: r.status,
                priority: r.priority,
                due_date: r.due_date,
                project_id: r.project_id,
                lead_id: r.lead_id,
                owner_id: r.owner_id,
                count: r.count,
                parent_id: r.parent_id,
            })
            .map(Self::calculate_task_fingerprint)
            .collect::<Vec<String>>()
    }

    async fn chat_response(
        &self,
        system_message: String,
        messages: Vec<Message>,
    ) -> Pin<Box<dyn Stream<Item = String> + Send>> {
        let mut conversation_messages: Vec<ChatCompletionRequestMessage> =
            messages.iter().map(Self::message_to_chat_completion).collect();

        let mut messages: Vec<ChatCompletionRequestMessage> = vec![ChatCompletionRequestSystemMessageArgs::default()
            .content(system_message)
            .build()
            .unwrap()
            .into()];

        messages.append(&mut conversation_messages);

        let create_task_input_schema = schema_for!(CreateTaskLLMFunctionInput);

        let create_task_function_def = json!({
            "type": "object",
            "properties": {
                "input": &create_task_input_schema,
            },
            "required": ["input"],
        });

        println!("create_task_function_def: {}", create_task_function_def);

        let request = CreateChatCompletionRequestArgs::default()
            .max_tokens(1024u16)
            .model(self.config.llm_model_name.clone())
            .messages(messages)
            // .tools(value)
            .functions([ChatCompletionFunctionsArgs::default()
                .name("create_task")
                .description("Create a task, complete the input object parameter inferred from the user's input.")
                .parameters(create_task_function_def)
                .build()
                .unwrap()])
            .function_call("auto")
            .build()
            .unwrap();

        let mut response = self.llm_client.chat().create_stream(request).await.unwrap();

        Box::pin(stream! {
            while let Some(response) = response.next().await {
                println!("response: {:?}", response);

                match response.unwrap().choices.first().unwrap().delta.content.clone() {
                    Some(content) => yield content,
                    None => break
                }
            }
        })
    }

    fn message_to_chat_completion(message: &'_ Message) -> ChatCompletionRequestMessage {
        let val: Value = serde_json::from_str(message.content.as_str()).unwrap();

        match val.clone() {
            Value::Object(obj) => match obj.get("role").unwrap().as_str().unwrap() {
                "user" => ChatCompletionRequestMessage::User(serde_json::from_value(val).unwrap()),
                "assistant" => ChatCompletionRequestMessage::Assistant(serde_json::from_value(val).unwrap()),
                "tool" | "function" => todo!(),
                _ => todo!(),
            },
            _ => todo!(),
        }
    }
}

#[derive(Clone, Default, JsonSchema)]
pub struct CreateTaskLLMFunctionInput {
    pub title: String,

    pub status: Option<TaskStatus>,
    pub priority: Option<TaskPriority>,
    pub description: Option<String>,
    pub due_date: Option<String>,
    pub project_id: Option<String>,
    pub lead_id: Option<String>,
    pub parent_id: Option<String>,
    pub labels: Option<Vec<String>>,
    pub assignees: Option<Vec<String>>,
    // pub subtasks: Option<Vec<CreateTaskInput>>,
    // pub assets: Option<Vec<String>>,
}

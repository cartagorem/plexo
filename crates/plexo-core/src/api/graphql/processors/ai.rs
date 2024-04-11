use async_graphql::{Context, Object, Result, Subscription};

use plexo_sdk::{
    cognition::{
        operations::{SubdivideTaskInput, TaskSuggestion, TaskSuggestionInput},
        v2::{
            chat::{ChatResponseChunk, ChatResponseInput},
            operations::CognitionOperationsV2,
            projects::{ProjectSuggestion, ProjectSuggestionInput},
        },
    },
    errors::sdk::SDKError,
    resources::chats::{
        chat::Chat,
        operations::{ChatCrudOperations, CreateChatInput},
    },
};

use tokio_stream::{Stream, StreamExt};

use crate::api::graphql::commons::extract_context;

#[derive(Default)]
pub struct AIProcessorGraphQLQuery;

#[derive(Default)]
pub struct AIProcessorGraphQLMutation;

#[derive(Default)]
pub struct AIProcessorGraphQLSubscription;

#[Object]
impl AIProcessorGraphQLQuery {
    async fn suggest_next_task(&self, ctx: &Context<'_>, input: TaskSuggestionInput) -> Result<TaskSuggestion> {
        let (core, _member_id) = extract_context(ctx)?;

        core.engine
            .get_suggestions_v2(input)
            .await
            .map_err(|err| async_graphql::Error::new(err.to_string()))
    }

    async fn subdivide_task(&self, ctx: &Context<'_>, input: SubdivideTaskInput) -> Result<Vec<TaskSuggestion>> {
        let (core, _member_id) = extract_context(ctx)?;

        core.engine
            .subdivide_task_v2(input)
            .await
            .map_err(|err| async_graphql::Error::new(err.to_string()))
    }

    async fn suggest_next_project(
        &self,
        ctx: &Context<'_>,
        input: ProjectSuggestionInput,
    ) -> Result<ProjectSuggestion> {
        let (core, _member_id) = extract_context(ctx)?;

        core.engine
            .get_project_suggestion(input)
            .await
            .map_err(|err| async_graphql::Error::new(err.to_string()))
    }
}

#[Object]
impl AIProcessorGraphQLMutation {
    async fn create_chat(&self, ctx: &Context<'_>, input: CreateChatInput) -> Result<Chat> {
        let (core, member_id) = extract_context(ctx)?;
        let mut input = input;

        input.owner_id = member_id;

        core.engine
            .create_chat(input)
            .await
            .map_err(|err| async_graphql::Error::new(err.to_string()))
    }

    async fn chat(&self, ctx: &Context<'_>, input: ChatResponseInput) -> Result<ChatResponseChunk> {
        let (core, _member_id) = extract_context(ctx).unwrap();

        let mut chat_stream = core.engine.get_chat_response(input).await.unwrap();
        let mut last_chunk = None;

        while let Some(chunk) = chat_stream.next().await {
            last_chunk = Some(chunk);
        }

        last_chunk.ok_or(SDKError::LLMStreamError.into())
    }
}

#[Subscription]
impl AIProcessorGraphQLSubscription {
    async fn chat(&self, ctx: &Context<'_>, input: ChatResponseInput) -> impl Stream<Item = ChatResponseChunk> {
        let (core, _member_id) = extract_context(ctx).unwrap();

        core.engine.get_chat_response(input).await.unwrap()
    }
}

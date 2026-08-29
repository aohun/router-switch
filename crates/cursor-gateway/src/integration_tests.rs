use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use async_stream::stream;
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    routing::post,
    Router,
};
use bytes::Bytes;
use domain::{CursorKind, CursorSettings};
use futures_util::StreamExt;
use parking_lot::RwLock;
use prost::Message;
use serde_json::json;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tower::ServiceExt;

use crate::{
    agent::project_history_for_tests,
    bidi_append,
    connect::{self, decode_frames},
    handlers::build_router,
    model::{FinishReason, ModelInvocation, PromptSpec, ToolCall},
    proto::{agent::v1 as pb, aiserver::v1 as ai},
    provider::{ModelEvent, Provider, ProviderStream, SharedProvider},
    proxy::CursorProxy,
    sessions::CursorSessionRegistry,
};

fn third_party_settings() -> CursorSettings {
    CursorSettings {
        kind: CursorKind::ThirdParty,
        api_key: "sk-mock-key-12345".into(),
        base_url: "https://api.example.com/v1".into(),
        model: "mock-model".into(),
        provider_type: "openai-chat".into(),
        options: json!({}),
        model_mappings: Vec::new(),
    }
}

fn official_settings() -> CursorSettings {
    domain::official_cursor_settings()
}

fn encode_append(request_id: &str, seqno: i64, message: pb::AgentClientMessage) -> Bytes {
    let req = ai::BidiAppendRequest {
        data: hex::encode(message.encode_to_vec()),
        request_id: Some(ai::BidiRequestId {
            request_id: request_id.into(),
        }),
        append_seqno: seqno,
        data_binary: Vec::new(),
    };
    connect::encode_message(&req).unwrap()
}

fn run_request(text: &str, model: &str) -> pb::AgentClientMessage {
    pb::AgentClientMessage {
        message: Some(pb::agent_client_message::Message::RunRequest(
            pb::AgentRunRequest {
                action: Some(pb::ConversationAction {
                    action: Some(pb::conversation_action::Action::UserMessageAction(
                        pb::UserMessageAction {
                            user_message: Some(pb::UserMessage {
                                text: text.into(),
                                message_id: "m1".into(),
                                ..Default::default()
                            }),
                            ..Default::default()
                        },
                    )),
                    ..Default::default()
                }),
                requested_model: Some(pb::RequestedModel {
                    model_id: model.into(),
                    ..Default::default()
                }),
                conversation_id: Some("conv-1".into()),
                ..Default::default()
            },
        )),
    }
}

struct ScriptedProvider {
    rounds: Mutex<Vec<Vec<ModelEvent>>>,
}

impl ScriptedProvider {
    fn new(rounds: Vec<Vec<ModelEvent>>) -> SharedProvider {
        Arc::new(Self {
            rounds: Mutex::new(rounds),
        })
    }
}

impl Provider for ScriptedProvider {
    fn stream(
        &self,
        _invocation: ModelInvocation,
        cancellation: CancellationToken,
    ) -> ProviderStream {
        let rounds = self.rounds.clone();
        Box::pin(stream! {
            let events = {
                let mut guard = rounds.lock().await;
                if guard.is_empty() {
                    vec![ModelEvent::TextDelta("fallback".into()), ModelEvent::Done(FinishReason::Stop)]
                } else {
                    guard.remove(0)
                }
            };
            for event in events {
                if cancellation.is_cancelled() {
                    return;
                }
                yield Ok(event);
            }
        })
    }
}

#[tokio::test]
async fn decodes_exec_client_message_fixture() {
    let message = pb::AgentClientMessage {
        message: Some(pb::agent_client_message::Message::ExecClientMessage(
            pb::ExecClientMessage {
                id: 3,
                exec_id: "tool-read-1".into(),
                message: Some(pb::exec_client_message::Message::ReadResult(
                    pb::ReadResult::default(),
                )),
                ..Default::default()
            },
        )),
    };
    let request = ai::BidiAppendRequest {
        data: hex::encode(message.encode_to_vec()),
        request_id: Some(ai::BidiRequestId {
            request_id: "fixture-req".into(),
        }),
        append_seqno: 2,
        data_binary: Vec::new(),
    };
    let decoded = bidi_append::decode(&request).unwrap();
    match decoded.message.message.unwrap() {
        pb::agent_client_message::Message::ExecClientMessage(exec) => {
            assert_eq!(exec.id, 3);
            assert_eq!(exec.exec_id, "tool-read-1");
        }
        other => panic!("exec_client_message was dropped: {other:?}"),
    }
}

#[tokio::test]
async fn third_party_routes_local_and_official_routes_upstream() {
    let upstream_hits = Arc::new(AtomicUsize::new(0));
    let hits = upstream_hits.clone();
    let upstream = Router::new().route(
        "/aiserver.v1.BidiService/BidiAppend",
        post(move || {
            let hits = hits.clone();
            async move {
                hits.fetch_add(1, Ordering::SeqCst);
                StatusCode::OK
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, upstream).await.unwrap();
    });

    // Official → upstream
    {
        let settings = Arc::new(RwLock::new(Some(official_settings())));
        let registry = CursorSessionRegistry::new(settings).unwrap();
        let proxy = CursorProxy::for_upstream(&format!("http://{addr}"));
        let router = build_router(registry.clone(), proxy);
        let body = encode_append("official-1", 0, run_request("hi", "claude-3.7-sonnet"));
        let res = router
            .oneshot(
                Request::post("/aiserver.v1.BidiService/BidiAppend")
                    .header("content-type", "application/proto")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert!(registry.upstream("official-1").await);
        assert_eq!(upstream_hits.load(Ordering::SeqCst), 1);
    }

    // ThirdParty → local (no upstream hit)
    {
        let settings = Arc::new(RwLock::new(Some(third_party_settings())));
        let registry = CursorSessionRegistry::new(settings).unwrap();
        registry.set_provider_override(Some(ScriptedProvider::new(vec![vec![
            ModelEvent::TextDelta("hello".into()),
            ModelEvent::Done(FinishReason::Stop),
        ]])));
        let proxy = CursorProxy::for_upstream(&format!("http://{addr}"));
        let router = build_router(registry.clone(), proxy);
        let body = encode_append("local-1", 0, run_request("hi", "mock-model"));
        let res = router
            .oneshot(
                Request::post("/aiserver.v1.BidiService/BidiAppend")
                    .header("content-type", "application/proto")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert!(registry.local("local-1").await.is_some());
        assert_eq!(upstream_hits.load(Ordering::SeqCst), 1);
    }
}

#[tokio::test]
async fn run_sse_rendezvous_does_not_miss_early_frames() {
    let settings = Arc::new(RwLock::new(Some(third_party_settings())));
    let registry = CursorSessionRegistry::new(settings).unwrap();
    registry.set_provider_override(Some(ScriptedProvider::new(vec![vec![
        ModelEvent::TextDelta("streamed".into()),
        ModelEvent::Done(FinishReason::Stop),
    ]])));
    let proxy = CursorProxy::default_upstream();
    let router = build_router(registry.clone(), proxy);

    // First append creates the actor and emits frames into OutputHub history.
    let append = encode_append("rz-1", 0, run_request("ping", "mock-model"));
    let res = router
        .clone()
        .oneshot(
            Request::post("/aiserver.v1.BidiService/BidiAppend")
                .header("content-type", "application/proto")
                .body(Body::from(append))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Give the actor a moment to emit.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let sse_req = pb::BidiRequestId {
        request_id: "rz-1".into(),
    };
    let sse_body = connect::encode_message(&sse_req).unwrap();
    let res = router
        .oneshot(
            Request::post("/agent.v1.AgentService/RunSSE")
                .header("content-type", "application/proto")
                .body(Body::from(sse_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    assert!(!body.is_empty(), "RunSSE should replay buffered frames");
}

#[tokio::test]
async fn tool_round_trip_resumes_model_until_done() {
    let settings = Arc::new(RwLock::new(Some(third_party_settings())));
    let registry = CursorSessionRegistry::new(settings).unwrap();
    registry.set_provider_override(Some(ScriptedProvider::new(vec![
        vec![
            ModelEvent::ToolCallStart {
                index: 0,
                call_id: "call-read".into(),
                name: "Read".into(),
            },
            ModelEvent::ToolCallArgumentsDelta {
                index: 0,
                delta: r#"{"path":"README.md"}"#.into(),
            },
            ModelEvent::ToolCallEnd { index: 0 },
            ModelEvent::Done(FinishReason::ToolUse),
        ],
        vec![
            ModelEvent::TextDelta("file contents summarized".into()),
            ModelEvent::Done(FinishReason::Stop),
        ],
    ])));

    let handle = registry.get_or_create("tool-1").await.unwrap();
    let mut subscriber = handle.subscribe();

    handle
        .command(crate::actor::CursorCommand::Append {
            seqno: 0,
            message: Box::new(run_request("read the readme", "mock-model")),
        })
        .await
        .unwrap();

    // Wait until an exec_server_message appears.
    let mut exec_id = None;
    for _ in 0..40 {
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        while let Ok(frame) = subscriber.try_recv() {
            if let Ok(frames) = decode_frames(&frame) {
                for (flags, payload) in frames {
                    if flags != 0 {
                        continue;
                    }
                    if let Ok(msg) = pb::AgentServerMessage::decode(payload.as_ref()) {
                        if let Some(pb::agent_server_message::Message::ExecServerMessage(exec)) =
                            msg.message
                        {
                            exec_id = Some(exec.id);
                        }
                    }
                }
            }
        }
        if exec_id.is_some() {
            break;
        }
    }
    let exec_id = exec_id.expect("expected exec_server_message for Read");

    // Client returns exec_client_message result.
    let result = pb::AgentClientMessage {
        message: Some(pb::agent_client_message::Message::ExecClientMessage(
            pb::ExecClientMessage {
                id: exec_id,
                exec_id: "call-read".into(),
                message: Some(pb::exec_client_message::Message::ReadResult(
                    pb::ReadResult::default(),
                )),
                ..Default::default()
            },
        )),
    };
    handle
        .command(crate::actor::CursorCommand::Append {
            seqno: 1,
            message: Box::new(result),
        })
        .await
        .unwrap();

    // Wait for end stream / final text.
    let mut saw_text = false;
    let mut saw_end = false;
    for _ in 0..80 {
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        while let Ok(frame) = subscriber.try_recv() {
            if frame.first().is_some_and(|f| f & connect::END_STREAM_FLAG != 0) {
                saw_end = true;
            }
            if let Ok(frames) = decode_frames(&frame) {
                for (flags, payload) in frames {
                    if flags != 0 {
                        continue;
                    }
                    if let Ok(msg) = pb::AgentServerMessage::decode(payload.as_ref()) {
                        if let Some(pb::agent_server_message::Message::InteractionUpdate(update)) =
                            msg.message
                        {
                            if let Some(pb::interaction_update::Message::TextDelta(delta)) =
                                update.message
                            {
                                if delta.text.contains("summarized") {
                                    saw_text = true;
                                }
                            }
                        }
                    }
                }
            }
        }
        if saw_text && saw_end {
            break;
        }
    }
    assert!(saw_text, "model should resume after tool result");
    assert!(saw_end, "run should finish with end stream");
}

#[tokio::test]
async fn web_fetch_server_path_after_client_approval() {
    let settings = Arc::new(RwLock::new(Some(third_party_settings())));
    let registry = CursorSessionRegistry::new(settings).unwrap();
    registry.set_provider_override(Some(ScriptedProvider::new(vec![
        vec![
            ModelEvent::ToolCallStart {
                index: 0,
                call_id: "call-fetch".into(),
                name: "WebFetch".into(),
            },
            ModelEvent::ToolCallArgumentsDelta {
                index: 0,
                delta: r#"{"url":"https://example.com"}"#.into(),
            },
            ModelEvent::ToolCallEnd { index: 0 },
            ModelEvent::Done(FinishReason::ToolUse),
        ],
        vec![
            ModelEvent::TextDelta("fetched".into()),
            ModelEvent::Done(FinishReason::Stop),
        ],
    ])));

    // Local HTTP fixture for WebFetch.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(
            listener,
            Router::new().route("/", axum::routing::get(|| async { "hello fixture" })),
        )
        .await
        .unwrap();
    });

    let handle = registry.get_or_create("fetch-1").await.unwrap();
    let mut subscriber = handle.subscribe();
    let mut run = run_request("fetch page", "mock-model");
    if let Some(pb::agent_client_message::Message::RunRequest(req)) = run.message.as_mut() {
        if let Some(pb::conversation_action::Action::UserMessageAction(action)) =
            req.action.as_mut().and_then(|a| a.action.as_mut())
        {
            if let Some(user) = action.user_message.as_mut() {
                user.text = format!("fetch http://{addr}/");
            }
        }
    }
    // Override tool args via scripted provider already points at example.com — patch provider instead.
    registry.set_provider_override(Some(ScriptedProvider::new(vec![
        vec![
            ModelEvent::ToolCallStart {
                index: 0,
                call_id: "call-fetch".into(),
                name: "WebFetch".into(),
            },
            ModelEvent::ToolCallArgumentsDelta {
                index: 0,
                delta: format!(r#"{{"url":"http://{addr}/"}}"#),
            },
            ModelEvent::ToolCallEnd { index: 0 },
            ModelEvent::Done(FinishReason::ToolUse),
        ],
        vec![
            ModelEvent::TextDelta("fetched ok".into()),
            ModelEvent::Done(FinishReason::Stop),
        ],
    ])));

    handle
        .command(crate::actor::CursorCommand::Append {
            seqno: 0,
            message: Box::new(run_request("fetch", "mock-model")),
        })
        .await
        .unwrap();

    let mut query_id = None;
    for _ in 0..40 {
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        while let Ok(frame) = subscriber.try_recv() {
            if let Ok(frames) = decode_frames(&frame) {
                for (flags, payload) in frames {
                    if flags != 0 {
                        continue;
                    }
                    if let Ok(msg) = pb::AgentServerMessage::decode(payload.as_ref()) {
                        if let Some(pb::agent_server_message::Message::InteractionQuery(query)) =
                            msg.message
                        {
                            query_id = Some(query.id);
                        }
                    }
                }
            }
        }
        if query_id.is_some() {
            break;
        }
    }
    let query_id = query_id.expect("expected WebFetch interaction query");

    let response = pb::AgentClientMessage {
        message: Some(pb::agent_client_message::Message::InteractionResponse(
            pb::InteractionResponse {
                id: query_id,
                result: Some(
                    pb::interaction_response::Result::WebFetchRequestResponse(
                        pb::WebFetchRequestResponse {
                            result: Some(pb::web_fetch_request_response::Result::Approved(
                                pb::web_fetch_request_response::Approved {},
                            )),
                        },
                    ),
                ),
            },
        )),
    };
    handle
        .command(crate::actor::CursorCommand::Append {
            seqno: 1,
            message: Box::new(response),
        })
        .await
        .unwrap();

    let mut saw_end = false;
    for _ in 0..80 {
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        while let Ok(frame) = subscriber.try_recv() {
            if frame.first().is_some_and(|f| f & connect::END_STREAM_FLAG != 0) {
                saw_end = true;
            }
        }
        if saw_end {
            break;
        }
    }
    assert!(saw_end, "WebFetch path should complete the run");
}

#[test]
fn empty_user_text_with_blob_and_history_is_not_empty_prompt() {
    let request = pb::AgentRunRequest {
        action: Some(pb::ConversationAction {
            action: Some(pb::conversation_action::Action::UserMessageAction(
                pb::UserMessageAction {
                    user_message: Some(pb::UserMessage {
                        text: String::new(),
                        message_id: "m".into(),
                        ..Default::default()
                    }),
                    conversation_history: Some(pb::ConversationHistory {
                        messages: vec![pb::ConversationHistoryMessage {
                            message: Some(pb::conversation_history_message::Message::User(
                                pb::ConversationHistoryUserMessage {
                                    content: vec![pb::ConversationHistoryUserContent {
                                        content: Some(
                                            pb::conversation_history_user_content::Content::Text(
                                                pb::ConversationHistoryTextContent {
                                                    text: "prior question".into(),
                                                },
                                            ),
                                        ),
                                    }],
                                },
                            )),
                        }],
                        replace_user_info: None,
                    }),
                    ..Default::default()
                },
            )),
            ..Default::default()
        }),
        pre_fetched_blobs: vec![pb::PreFetchedBlob {
            id: b"blob-1".to_vec(),
            value: b"blob context payload".to_vec(),
        }],
        conversation_state: Some(pb::ConversationStateStructure {
            root_prompt_messages_json: vec![b"system root context".to_vec()],
            ..Default::default()
        }),
        ..Default::default()
    };
    let projected = project_history_for_tests(&request);
    assert!(!projected.trim().is_empty());
    assert!(projected.contains("prior question"));
    assert!(projected.contains("blob context payload"));
    assert!(projected.contains("system root context"));
}

#[test]
fn prompt_compiler_embeds_system_and_tools() {
    let compiler = crate::prompting::PromptCompiler::embedded().unwrap();
    let spec = compiler
        .prompt_spec(
            crate::prompting::Mode::Agent,
            "mock-model",
            Some("Mock Model"),
            &[],
            false,
            false,
        )
        .unwrap();
    assert!(!spec.instructions.is_empty());
    assert!(spec.tools.iter().any(|t| t.name == "Read" || t.name == "Shell"));
    let _ = PromptSpec {
        instructions: spec.instructions,
        tools: spec.tools,
    };
}

#[test]
fn merge_extra_params_rejects_protected_keys() {
    let mut body = json!({"model": "x", "messages": []});
    let err = crate::provider::merge_extra_params(
        &mut body,
        &json!({"model": "hijack", "temperature": 0.2}),
    )
    .unwrap_err();
    assert!(err.to_string().contains("cannot replace model"));
}

#[test]
fn tool_call_struct_roundtrip_fields() {
    let call = ToolCall {
        index: 0,
        call_id: "c1".into(),
        model_call_id: "c1".into(),
        name: "Read".into(),
        arguments_text: r#"{"path":"a"}"#.into(),
        arguments: json!({"path":"a"}),
    };
    assert_eq!(call.name, "Read");
}

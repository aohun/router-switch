use axum::{
    body::{Body, Bytes},
    extract::{Extension, State},
    http::{header, HeaderValue, Request, Response, StatusCode},
};
use bytes::{BufMut, BytesMut};
use domain::CursorSettings;
use parking_lot::RwLock;
use prost::Message;
use std::sync::Arc;

use crate::{
    handlers::AppState,
    proto::{
        agent::v1 as agent_pb,
        catalog::{
            AvailableModel, AvailableModelVendor, AvailableModelsAddition, BooleanParameter,
            BooleanParameterValue, EnumParameter, EnumParameterValue, ModelParameterDefinition,
            ModelParameterType, ModelParameterValue, ModelPickerBadge, ModelVariant, TooltipData,
            UsableModelsAddition,
        },
    },
    proxy::{self, CursorProxy},
    GatewayError, Result,
};
use domain::{
    normalize_thinking_effort, RequestProtocol, DEFAULT_THINKING_EFFORT, THINKING_EFFORTS,
};

const CONTEXTS: [(&str, &str); 4] = [
    ("200k", "200K"),
    ("356k", "356K"),
    ("800k", "800K"),
    ("1m", "1M"),
];
const DEFAULT_CONTEXT: &str = "200k";

pub async fn available_models(
    State(state): State<AppState>,
    Extension(proxy): Extension<CursorProxy>,
    request: Request<Body>,
) -> Result<Response<Body>> {
    let catalog = catalog_from_settings(&state.registry.settings());
    let model_names: Vec<String> = catalog.models.iter().map(|m| m.id.clone()).collect();
    let models: Vec<AvailableModel> = catalog
        .models
        .iter()
        .map(|m| build_available_model(m, &catalog))
        .collect();

    let local = AvailableModelsAddition {
        model_names,
        models,
    }
    .encode_to_vec();

    match proxy::forward_buffered(&proxy, request).await {
        Ok(upstream) => merge_response(upstream, local),
        Err(error) => {
            tracing::warn!(%error, "Cursor AvailableModels upstream unavailable; using local catalog");
            Ok(local_response(local))
        }
    }
}

pub async fn usable_models(
    State(state): State<AppState>,
    Extension(proxy): Extension<CursorProxy>,
    request: Request<Body>,
) -> Result<Response<Body>> {
    let catalog = catalog_from_settings(&state.registry.settings());
    let models: Vec<agent_pb::ModelDetails> = catalog
        .models
        .iter()
        .map(|m| build_model_details(&m.id, &m.display_name))
        .collect();

    let local = UsableModelsAddition { models }.encode_to_vec();

    match proxy::forward_buffered(&proxy, request).await {
        Ok(upstream) => merge_response(upstream, local),
        Err(error) => {
            tracing::warn!(%error, "Cursor GetUsableModels upstream unavailable; using local catalog");
            Ok(local_response(local))
        }
    }
}

struct CatalogModel {
    id: String,
    display_name: String,
}

struct CatalogConfig {
    protocol: RequestProtocol,
    default_effort: String,
    models: Vec<CatalogModel>,
}

fn catalog_from_settings(settings: &Arc<RwLock<Option<CursorSettings>>>) -> CatalogConfig {
    let guard = settings.read();
    let protocol = guard
        .as_ref()
        .map(|s| RequestProtocol::parse(&s.provider_type))
        .unwrap_or(RequestProtocol::OpenAiChat);
    let default_effort = guard
        .as_ref()
        .map(|s| normalize_thinking_effort(&s.default_reasoning_effort))
        .unwrap_or_else(|| DEFAULT_THINKING_EFFORT.to_string());
    CatalogConfig {
        protocol,
        default_effort,
        models: get_active_models(settings),
    }
}

fn build_available_model(model: &CatalogModel, catalog: &CatalogConfig) -> AvailableModel {
    let short_name = model
        .display_name
        .split('/')
        .last()
        .unwrap_or(&model.display_name)
        .to_string();
    let contexts = context_options(None);
    let variants = model_variants(model, &contexts, &catalog.default_effort);
    let legacy_slugs = variants
        .iter()
        .filter_map(|variant| variant.legacy_slug.clone())
        .collect();
    let tooltip = model_tooltip(&model.display_name, "200K", &catalog.default_effort, false);
    let badge = catalog.protocol.picker_badge();
    AvailableModel {
        name: model.id.clone(),
        default_on: true,
        supports_agent: Some(true),
        degradation_status: Some(0),
        tooltip_data: Some(tooltip.clone()),
        supports_thinking: Some(true),
        supports_images: Some(true),
        supports_max_mode: Some(true),
        client_display_name: Some(model.display_name.clone()),
        server_model_name: Some(model.id.clone()),
        supports_non_max_mode: Some(true),
        tooltip_data_for_max_mode: Some(tooltip),
        is_recommended_for_background_composer: Some(true),
        supports_plan_mode: Some(true),
        inputbox_short_model_name: Some(short_name),
        supports_sandboxing: Some(true),
        supports_cmd_k: Some(true),
        parameter_definitions: model_parameters(&contexts),
        variants,
        legacy_slugs,
        named_model_section_index: Some(1),
        vendor_name: Some(badge.to_string()),
        vendor: Some(AvailableModelVendor {
            id: if catalog.protocol == RequestProtocol::Anthropic {
                1
            } else {
                2
            },
            display_name: badge.to_string(),
        }),
        model_picker_badges: vec![ModelPickerBadge {
            label: badge.to_string(),
            variant: 1,
            dismiss_on_selection: false,
        }],
    }
}

fn context_options(custom: Option<u64>) -> Vec<(String, String)> {
    let mut contexts = CONTEXTS
        .into_iter()
        .map(|(value, display_name)| (value.to_owned(), display_name.to_owned()))
        .collect::<Vec<_>>();
    if let Some(tokens) = custom {
        if !contexts
            .iter()
            .any(|(existing, _)| existing == &tokens.to_string())
        {
            let label = if tokens >= 1_000_000 {
                format!("{}M (Custom)", tokens / 1_000_000)
            } else {
                format!("{}K (Custom)", tokens / 1000)
            };
            contexts.push((tokens.to_string(), label));
        }
    }
    contexts
}

fn model_parameters(contexts: &[(String, String)]) -> Vec<ModelParameterDefinition> {
    vec![
        ModelParameterDefinition {
            id: "context".into(),
            name: "Context".into(),
            markdown_tooltip: Some("Context size used to trigger conversation compaction.".into()),
            parameter_type: Some(ModelParameterType {
                boolean_parameter: None,
                enum_parameter: Some(EnumParameter {
                    values: contexts
                        .iter()
                        .map(|(value, display_name)| EnumParameterValue {
                            value: value.clone(),
                            display_name: Some(display_name.clone()),
                        })
                        .collect(),
                }),
            }),
            is_cycleable_by_hotkey: Some(false),
        },
        ModelParameterDefinition {
            id: "effort".into(),
            name: "Effort".into(),
            markdown_tooltip: Some("Effort the model uses to generate its response.".into()),
            parameter_type: Some(ModelParameterType {
                boolean_parameter: None,
                enum_parameter: Some(EnumParameter {
                    values: THINKING_EFFORTS
                        .into_iter()
                        .map(|(value, display_name)| EnumParameterValue {
                            value: value.into(),
                            display_name: Some(display_name.into()),
                        })
                        .collect(),
                }),
            }),
            is_cycleable_by_hotkey: Some(true),
        },
        ModelParameterDefinition {
            id: "fast".into(),
            name: "Fast".into(),
            markdown_tooltip: Some("Significantly faster but consumes more usage".into()),
            parameter_type: Some(ModelParameterType {
                boolean_parameter: Some(BooleanParameter {
                    values: vec![
                        BooleanParameterValue {
                            value: "false".into(),
                            display_name: None,
                            increases_model_cost: None,
                        },
                        BooleanParameterValue {
                            value: "true".into(),
                            display_name: Some("Fast".into()),
                            increases_model_cost: Some(true),
                        },
                    ],
                }),
                enum_parameter: None,
            }),
            is_cycleable_by_hotkey: Some(false),
        },
    ]
}

fn model_variants(
    model: &CatalogModel,
    contexts: &[(String, String)],
    default_effort: &str,
) -> Vec<ModelVariant> {
    let mut variants = Vec::new();
    for (context, context_name) in contexts {
        for (effort, effort_name) in THINKING_EFFORTS {
            for fast in [false, true] {
                let mut suffix = Vec::new();
                if context != DEFAULT_CONTEXT {
                    suffix.push(context_name.as_str());
                }
                suffix.push(effort_name);
                if fast {
                    suffix.push("Fast");
                }
                let suffix = suffix.join(" ");
                let display_name = format!(
                    "{} <span style=\"color: var(--cursor-text-tertiary);\">{suffix}</span>",
                    model.display_name
                );
                let is_default = context == DEFAULT_CONTEXT && effort == default_effort && !fast;
                variants.push(ModelVariant {
                    parameter_values: vec![
                        ModelParameterValue {
                            id: "context".into(),
                            value: context.clone(),
                        },
                        ModelParameterValue {
                            id: "effort".into(),
                            value: effort.into(),
                        },
                        ModelParameterValue {
                            id: "fast".into(),
                            value: fast.to_string(),
                        },
                    ],
                    display_name: display_name.clone(),
                    is_max_mode: false,
                    is_default_max_config: is_default.then_some(true),
                    is_default_non_max_config: is_default.then_some(true),
                    tooltip_data: Some(model_tooltip(
                        &model.display_name,
                        context_name,
                        effort,
                        fast,
                    )),
                    display_name_outside_picker: Some(display_name),
                    variant_string_representation: Some(format!(
                        "{}[context={context},effort={effort},fast={fast}]",
                        model.id
                    )),
                    legacy_slug: Some(format!(
                        "{}-{context}-{effort}{}",
                        model.id,
                        if fast { "-fast" } else { "" }
                    )),
                });
            }
        }
    }
    variants
}

fn model_tooltip(display_name: &str, context_name: &str, effort: &str, fast: bool) -> TooltipData {
    let fast_label = if fast { " (Fast)" } else { "" };
    TooltipData {
        markdown_content: Some(format!(
            "**{display_name}{fast_label}**<br /><br />{context_name} context window<br /><br />*Version: {effort} effort*"
        )),
    }
}

fn build_model_details(id: &str, display_name: &str) -> agent_pb::ModelDetails {
    let short_name = display_name
        .split('/')
        .last()
        .unwrap_or(display_name)
        .to_string();

    agent_pb::ModelDetails {
        model_id: id.to_string(),
        thinking_details: Some(agent_pb::ThinkingDetails {}),
        display_model_id: id.to_string(),
        display_name: display_name.to_string(),
        display_name_short: short_name,
        aliases: vec![id.to_string()],
        max_mode: Some(false),
        ..Default::default()
    }
}

fn get_active_models(settings: &Arc<RwLock<Option<CursorSettings>>>) -> Vec<CatalogModel> {
    let mut models: Vec<CatalogModel> = Vec::new();
    if let Some(s) = settings.read().as_ref() {
        if !s.model.is_empty() {
            let id = s.model.clone();
            models.push(CatalogModel {
                id: id.clone(),
                display_name: id,
            });
        }
        for mapping in &s.model_mappings {
            if !mapping.model.is_empty() {
                let id = mapping.model.clone();
                let display = if mapping.display_name.is_empty() {
                    id.clone()
                } else {
                    mapping.display_name.clone()
                };
                if !models.iter().any(|existing| existing.id == id) {
                    models.push(CatalogModel {
                        id,
                        display_name: display,
                    });
                }
            }
        }
    }
    if models.is_empty() {
        models.push(CatalogModel {
            id: "claude-3.7-sonnet".into(),
            display_name: "Claude 3.7 Sonnet".into(),
        });
    }
    models
}

fn merge_response(upstream: proxy::BufferedResponse, extra: Vec<u8>) -> Result<Response<Body>> {
    if !upstream.status.is_success() {
        tracing::warn!(status = %upstream.status, "Cursor model catalog upstream rejected request; using local catalog");
        return Ok(local_response(extra));
    }
    let (framed, payload) = unary_payload(&upstream.body)?;
    let body = if framed {
        let mut merged = BytesMut::with_capacity(5 + payload.len() + extra.len());
        merged.put_u8(0);
        merged.put_u32((payload.len() + extra.len()) as u32);
        merged.extend_from_slice(payload);
        merged.extend_from_slice(&extra);
        merged.freeze()
    } else {
        let mut merged = BytesMut::with_capacity(payload.len() + extra.len());
        merged.extend_from_slice(payload);
        merged.extend_from_slice(&extra);
        merged.freeze()
    };
    Ok(upstream.with_body(body))
}

fn local_response(body: Vec<u8>) -> Response<Body> {
    let length = body.len();
    let mut response = Response::new(Body::from(body));
    *response.status_mut() = StatusCode::OK;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/proto"),
    );
    response
        .headers_mut()
        .insert(header::CONTENT_LENGTH, length.to_string().parse().unwrap());
    response
}

fn unary_payload(body: &Bytes) -> Result<(bool, &[u8])> {
    if body.len() < 5 {
        return Ok((false, body));
    }
    let flags = body[0];
    let length = u32::from_be_bytes([body[1], body[2], body[3], body[4]]) as usize;
    if length != body.len() - 5 {
        return Ok((false, body));
    }
    if flags != 0 {
        return Err(GatewayError::Protocol(format!(
            "cannot merge compressed or terminal model catalog frame: flags={flags}"
        )));
    }
    Ok((true, &body[5..]))
}

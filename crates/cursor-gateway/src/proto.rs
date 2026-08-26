pub mod aiserver {
    pub mod v1 {
        use prost::Message;

        #[derive(Clone, PartialEq, Message)]
        pub struct BidiRequestId {
            #[prost(string, tag = "1")]
            pub request_id: String,
        }

        #[derive(Clone, PartialEq, Message)]
        pub struct BidiAppendRequest {
            #[prost(string, tag = "1")]
            pub data: String,
            #[prost(message, optional, tag = "2")]
            pub request_id: Option<BidiRequestId>,
            #[prost(int64, tag = "3")]
            pub append_seqno: i64,
            #[prost(bytes = "vec", tag = "4")]
            pub data_binary: Vec<u8>,
        }

        #[derive(Clone, Copy, PartialEq, Message)]
        pub struct BidiAppendResponse {}

        #[derive(Clone, PartialEq, Message)]
        pub struct StreamChatRequest {
            #[prost(string, optional, tag = "1")]
            pub prompt: Option<String>,
            #[prost(string, optional, tag = "2")]
            pub model_id: Option<String>,
            #[prost(string, optional, tag = "3")]
            pub conversation_id: Option<String>,
        }

        #[derive(Clone, PartialEq, Message)]
        pub struct StreamChatResponse {
            #[prost(string, tag = "1")]
            pub text: String,
            #[prost(string, optional, tag = "2")]
            pub model_name: Option<String>,
        }
    }
}

pub mod agent {
    pub mod v1 {
        use prost::Message;

        #[derive(Clone, PartialEq, Message)]
        pub struct BidiRequestId {
            #[prost(string, tag = "1")]
            pub request_id: String,
        }

        #[derive(Clone, PartialEq, Message)]
        pub struct AgentClientMessage {
            #[prost(message, optional, tag = "1")]
            pub run_request: Option<AgentRunRequest>,
        }

        #[derive(Clone, PartialEq, Message)]
        pub struct ConversationStateStructure {
            #[prost(string, repeated, tag = "1")]
            pub root_prompt_messages_json: Vec<String>,
        }

        #[derive(Clone, PartialEq, Message)]
        pub struct ConversationAction {
            #[prost(message, optional, tag = "1")]
            pub user_message_action: Option<UserMessageAction>,
        }

        #[derive(Clone, PartialEq, Message)]
        pub struct UserMessageAction {
            #[prost(message, optional, tag = "1")]
            pub user_message: Option<UserMessage>,
        }

        #[derive(Clone, PartialEq, Message)]
        pub struct UserMessage {
            #[prost(string, tag = "1")]
            pub text: String,
            #[prost(string, optional, tag = "2")]
            pub message_id: Option<String>,
        }

        #[derive(Clone, PartialEq, Message)]
        pub struct AgentRunRequest {
            #[prost(message, optional, tag = "1")]
            pub conversation_state: Option<ConversationStateStructure>,
            #[prost(message, optional, tag = "2")]
            pub action: Option<ConversationAction>,
            #[prost(message, optional, tag = "3")]
            pub model_details: Option<ModelDetails>,
            #[prost(string, optional, tag = "5")]
            pub conversation_id: Option<String>,
            #[prost(string, optional, tag = "8")]
            pub custom_system_prompt: Option<String>,
            #[prost(message, optional, tag = "9")]
            pub requested_model: Option<RequestedModel>,
        }

        #[derive(Clone, PartialEq, Message)]
        pub struct RequestedModel {
            #[prost(string, tag = "1")]
            pub model_id: String,
            #[prost(string, optional, tag = "2")]
            pub display_name: Option<String>,
        }

        #[derive(Clone, PartialEq, Message)]
        pub struct ModelDetails {
            #[prost(string, tag = "1")]
            pub model_id: String,
            #[prost(message, optional, tag = "2")]
            pub thinking_details: Option<ThinkingDetails>,
            #[prost(string, tag = "3")]
            pub display_model_id: String,
            #[prost(string, tag = "4")]
            pub display_name: String,
            #[prost(string, tag = "5")]
            pub display_name_short: String,
            #[prost(string, repeated, tag = "6")]
            pub aliases: Vec<String>,
            #[prost(bool, optional, tag = "7")]
            pub max_mode: Option<bool>,
        }

        #[derive(Clone, Copy, PartialEq, Message)]
        pub struct ThinkingDetails {}

        #[derive(Clone, PartialEq, Message)]
        pub struct AgentServerMessage {
            #[prost(message, optional, tag = "1")]
            pub interaction_update: Option<InteractionUpdate>,
        }

        #[derive(Clone, PartialEq, Message)]
        pub struct InteractionUpdate {
            #[prost(message, optional, tag = "1")]
            pub text_delta: Option<TextDeltaUpdate>,
            #[prost(message, optional, tag = "4")]
            pub thinking_delta: Option<ThinkingDeltaUpdate>,
            #[prost(message, optional, tag = "7")]
            pub partial_tool_call: Option<PartialToolCallUpdate>,
            #[prost(message, optional, tag = "14")]
            pub turn_ended: Option<TurnEndedUpdate>,
        }

        #[derive(Clone, PartialEq, Message)]
        pub struct TextDeltaUpdate {
            #[prost(string, tag = "1")]
            pub text: String,
            #[prost(bool, tag = "2")]
            pub is_server_notice: bool,
        }

        #[derive(Clone, PartialEq, Message)]
        pub struct ThinkingDeltaUpdate {
            #[prost(string, tag = "1")]
            pub text: String,
        }

        #[derive(Clone, PartialEq, Message)]
        pub struct PartialToolCallUpdate {
            #[prost(string, tag = "1")]
            pub call_id: String,
            #[prost(string, tag = "2")]
            pub tool_name: String,
            #[prost(string, tag = "3")]
            pub args_text_delta: String,
        }

        #[derive(Clone, PartialEq, Message)]
        pub struct TurnEndedUpdate {
            #[prost(int64, optional, tag = "1")]
            pub input_tokens: Option<i64>,
            #[prost(int64, optional, tag = "2")]
            pub output_tokens: Option<i64>,
        }
    }
}

pub mod account {
    use prost::Message;

    #[derive(Clone, PartialEq, Message)]
    pub struct GetEmailResponse {
        #[prost(string, tag = "1")]
        pub email: String,
        #[prost(int32, tag = "2")]
        pub sign_up_type: i32,
    }

    #[derive(Clone, PartialEq, Message)]
    pub struct GetMeResponse {
        #[prost(string, tag = "1")]
        pub auth_id: String,
        #[prost(int32, tag = "2")]
        pub user_id: i32,
        #[prost(string, optional, tag = "3")]
        pub email: Option<String>,
        #[prost(string, optional, tag = "4")]
        pub first_name: Option<String>,
        #[prost(string, optional, tag = "5")]
        pub last_name: Option<String>,
        #[prost(string, optional, tag = "8")]
        pub created_at: Option<String>,
        #[prost(bool, optional, tag = "9")]
        pub is_enterprise_user: Option<bool>,
        #[prost(string, optional, tag = "11")]
        pub email_domain_type: Option<String>,
        #[prost(string, optional, tag = "12")]
        pub country: Option<String>,
    }

    #[derive(Clone, PartialEq, Message)]
    pub struct GetUserProfileResponse {
        #[prost(bool, optional, tag = "4")]
        pub public_visibility_allowed: Option<bool>,
        #[prost(string, optional, tag = "5")]
        pub max_visibility: Option<String>,
    }

    #[derive(Clone, PartialEq, Message)]
    pub struct GetCurrentPeriodUsageResponse {
        #[prost(int64, tag = "1")]
        pub billing_cycle_start: i64,
        #[prost(int64, tag = "2")]
        pub billing_cycle_end: i64,
        #[prost(message, optional, tag = "3")]
        pub plan_usage: Option<PlanUsage>,
        #[prost(message, optional, tag = "4")]
        pub spend_limit_usage: Option<SpendLimitUsage>,
        #[prost(int32, optional, tag = "5")]
        pub display_threshold: Option<i32>,
        #[prost(bool, tag = "6")]
        pub enabled: bool,
        #[prost(string, tag = "7")]
        pub display_message: String,
        #[prost(string, optional, tag = "11")]
        pub auto_model_selected_display_message: Option<String>,
        #[prost(string, optional, tag = "12")]
        pub named_model_selected_display_message: Option<String>,
    }

    #[derive(Clone, PartialEq, Message)]
    pub struct PlanUsage {
        #[prost(int32, tag = "1")]
        pub total_spend: i32,
        #[prost(int32, tag = "2")]
        pub included_spend: i32,
        #[prost(int32, tag = "4")]
        pub remaining: i32,
        #[prost(int32, tag = "5")]
        pub limit: i32,
        #[prost(bool, optional, tag = "6")]
        pub remaining_bonus: Option<bool>,
        #[prost(string, optional, tag = "7")]
        pub bonus_tooltip: Option<String>,
        #[prost(int32, optional, tag = "8")]
        pub auto_spend: Option<i32>,
        #[prost(int32, optional, tag = "9")]
        pub api_spend: Option<i32>,
        #[prost(double, optional, tag = "12")]
        pub auto_percent_used: Option<f64>,
        #[prost(double, optional, tag = "13")]
        pub api_percent_used: Option<f64>,
        #[prost(double, optional, tag = "14")]
        pub total_percent_used: Option<f64>,
    }

    #[derive(Clone, PartialEq, Message)]
    pub struct SpendLimitUsage {
        #[prost(string, tag = "8")]
        pub limit_type: String,
    }

    #[derive(Clone, PartialEq, Message)]
    pub struct GetUsageLimitStatusAndActiveGrantsResponse {
        #[prost(message, optional, tag = "1")]
        pub usage_limit_policy_status: Option<UsageLimitPolicyStatus>,
    }

    #[derive(Clone, PartialEq, Message)]
    pub struct UsageLimitPolicyStatus {
        #[prost(bool, tag = "1")]
        pub is_in_slow_pool: bool,
        #[prost(map = "string, string", tag = "5")]
        pub features: std::collections::HashMap<String, String>,
        #[prost(bool, tag = "6")]
        pub can_configure_spend_limit: bool,
        #[prost(bool, tag = "8")]
        pub has_pending_request: bool,
        #[prost(string, repeated, tag = "9")]
        pub allowed_model_ids: Vec<String>,
        #[prost(string, repeated, tag = "10")]
        pub allowed_model_tags: Vec<String>,
    }

    #[derive(Clone, Copy, PartialEq, Message)]
    pub struct Empty {}
}

pub mod catalog {
    use prost::Message;

    #[derive(Clone, PartialEq, Message)]
    pub struct AvailableModelsAddition {
        #[prost(string, repeated, tag = "1")]
        pub model_names: Vec<String>,
        #[prost(message, repeated, tag = "2")]
        pub models: Vec<AvailableModel>,
    }

    #[derive(Clone, PartialEq, Message)]
    pub struct AvailableModel {
        #[prost(string, tag = "1")]
        pub name: String,
        #[prost(bool, tag = "2")]
        pub default_on: bool,
        #[prost(bool, optional, tag = "5")]
        pub supports_agent: Option<bool>,
        #[prost(int32, optional, tag = "6")]
        pub degradation_status: Option<i32>,
        #[prost(message, optional, tag = "8")]
        pub tooltip_data: Option<TooltipData>,
        #[prost(bool, optional, tag = "9")]
        pub supports_thinking: Option<bool>,
        #[prost(bool, optional, tag = "10")]
        pub supports_images: Option<bool>,
        #[prost(bool, optional, tag = "14")]
        pub supports_max_mode: Option<bool>,
        #[prost(string, optional, tag = "17")]
        pub client_display_name: Option<String>,
        #[prost(string, optional, tag = "18")]
        pub server_model_name: Option<String>,
        #[prost(bool, optional, tag = "19")]
        pub supports_non_max_mode: Option<bool>,
        #[prost(message, optional, tag = "20")]
        pub tooltip_data_for_max_mode: Option<TooltipData>,
        #[prost(bool, optional, tag = "21")]
        pub is_recommended_for_background_composer: Option<bool>,
        #[prost(bool, optional, tag = "22")]
        pub supports_plan_mode: Option<bool>,
        #[prost(string, optional, tag = "24")]
        pub inputbox_short_model_name: Option<String>,
        #[prost(bool, optional, tag = "25")]
        pub supports_sandboxing: Option<bool>,
        #[prost(bool, optional, tag = "26")]
        pub supports_cmd_k: Option<bool>,
        #[prost(message, repeated, tag = "29")]
        pub parameter_definitions: Vec<ModelParameterDefinition>,
        #[prost(message, repeated, tag = "30")]
        pub variants: Vec<ModelVariant>,
        #[prost(string, repeated, tag = "36")]
        pub legacy_slugs: Vec<String>,
        #[prost(int32, optional, tag = "38")]
        pub named_model_section_index: Option<i32>,
        #[prost(string, optional, tag = "41")]
        pub vendor_name: Option<String>,
        #[prost(message, optional, tag = "42")]
        pub vendor: Option<AvailableModelVendor>,
        #[prost(message, repeated, tag = "48")]
        pub model_picker_badges: Vec<ModelPickerBadge>,
    }

    #[derive(Clone, PartialEq, Message)]
    pub struct TooltipData {
        #[prost(string, optional, tag = "7")]
        pub markdown_content: Option<String>,
    }

    #[derive(Clone, PartialEq, Message)]
    pub struct ModelParameterDefinition {
        #[prost(string, tag = "1")]
        pub id: String,
        #[prost(string, tag = "2")]
        pub name: String,
        #[prost(string, optional, tag = "3")]
        pub markdown_tooltip: Option<String>,
        #[prost(message, optional, tag = "4")]
        pub parameter_type: Option<ModelParameterType>,
        #[prost(bool, optional, tag = "5")]
        pub is_cycleable_by_hotkey: Option<bool>,
    }

    #[derive(Clone, PartialEq, Message)]
    pub struct ModelParameterType {
        #[prost(message, optional, tag = "1")]
        pub boolean_parameter: Option<BooleanParameter>,
        #[prost(message, optional, tag = "2")]
        pub enum_parameter: Option<EnumParameter>,
    }

    #[derive(Clone, PartialEq, Message)]
    pub struct BooleanParameter {
        #[prost(message, repeated, tag = "1")]
        pub values: Vec<BooleanParameterValue>,
    }

    #[derive(Clone, PartialEq, Message)]
    pub struct BooleanParameterValue {
        #[prost(string, tag = "1")]
        pub value: String,
        #[prost(string, optional, tag = "2")]
        pub display_name: Option<String>,
        #[prost(bool, optional, tag = "3")]
        pub increases_model_cost: Option<bool>,
    }

    #[derive(Clone, PartialEq, Message)]
    pub struct EnumParameter {
        #[prost(message, repeated, tag = "1")]
        pub values: Vec<EnumParameterValue>,
    }

    #[derive(Clone, PartialEq, Message)]
    pub struct EnumParameterValue {
        #[prost(string, tag = "1")]
        pub value: String,
        #[prost(string, optional, tag = "2")]
        pub display_name: Option<String>,
    }

    #[derive(Clone, PartialEq, Message)]
    pub struct ModelVariant {
        #[prost(message, repeated, tag = "1")]
        pub parameter_values: Vec<ModelParameterValue>,
        #[prost(string, tag = "2")]
        pub display_name: String,
        #[prost(bool, tag = "3")]
        pub is_max_mode: bool,
        #[prost(bool, optional, tag = "4")]
        pub is_default_max_config: Option<bool>,
        #[prost(bool, optional, tag = "5")]
        pub is_default_non_max_config: Option<bool>,
        #[prost(message, optional, tag = "6")]
        pub tooltip_data: Option<TooltipData>,
        #[prost(string, optional, tag = "8")]
        pub display_name_outside_picker: Option<String>,
        #[prost(string, optional, tag = "9")]
        pub variant_string_representation: Option<String>,
        #[prost(string, optional, tag = "11")]
        pub legacy_slug: Option<String>,
    }

    #[derive(Clone, PartialEq, Message)]
    pub struct ModelParameterValue {
        #[prost(string, tag = "1")]
        pub id: String,
        #[prost(string, tag = "2")]
        pub value: String,
    }

    #[derive(Clone, PartialEq, Message)]
    pub struct ModelPickerBadge {
        #[prost(string, tag = "1")]
        pub label: String,
        #[prost(int32, tag = "2")]
        pub variant: i32,
        #[prost(bool, tag = "3")]
        pub dismiss_on_selection: bool,
    }

    #[derive(Clone, PartialEq, Message)]
    pub struct AvailableModelVendor {
        #[prost(int32, tag = "1")]
        pub id: i32,
        #[prost(string, tag = "2")]
        pub display_name: String,
    }

    #[derive(Clone, PartialEq, Message)]
    pub struct UsableModelsAddition {
        #[prost(message, repeated, tag = "1")]
        pub models: Vec<super::agent::v1::ModelDetails>,
    }
}

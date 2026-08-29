pub mod agent {
    #[allow(clippy::large_enum_variant)]
    pub mod v1 {
        include!(concat!(env!("OUT_DIR"), "/agent.v1.rs"));
    }
}

pub mod aiserver {
    pub mod v1 {
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct BidiRequestId {
            #[prost(string, tag = "1")]
            pub request_id: String,
        }

        #[derive(Clone, PartialEq, ::prost::Message)]
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

        #[derive(Clone, Copy, PartialEq, ::prost::Message)]
        pub struct BidiAppendResponse {}

        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct CustomErrorDetails {
            #[prost(string, tag = "1")]
            pub title: String,
            #[prost(string, tag = "2")]
            pub detail: String,
            #[prost(bool, optional, tag = "3")]
            pub allow_command_links_potentially_unsafe_please_only_use_for_handwritten_trusted_markdown:
                Option<bool>,
            #[prost(bool, optional, tag = "4")]
            pub is_retryable: Option<bool>,
            #[prost(bool, optional, tag = "5")]
            pub show_request_id: Option<bool>,
            #[prost(bool, optional, tag = "6")]
            pub should_show_immediate_error: Option<bool>,
        }

        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct ErrorDetails {
            #[prost(enumeration = "error_details::Error", tag = "1")]
            pub error: i32,
            #[prost(message, optional, tag = "2")]
            pub details: Option<CustomErrorDetails>,
            #[prost(bool, optional, tag = "3")]
            pub is_expected: Option<bool>,
        }

        pub mod error_details {
            #[derive(
                Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration,
            )]
            #[repr(i32)]
            pub enum Error {
                Unspecified = 0,
                CustomMessage = 29,
                ProviderError = 57,
                Internal = 59,
            }
        }

        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct StreamChatRequest {
            #[prost(string, optional, tag = "1")]
            pub prompt: Option<String>,
            #[prost(string, optional, tag = "2")]
            pub model_id: Option<String>,
            #[prost(string, optional, tag = "3")]
            pub conversation_id: Option<String>,
        }

        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct StreamChatResponse {
            #[prost(string, tag = "1")]
            pub text: String,
            #[prost(string, optional, tag = "2")]
            pub model_name: Option<String>,
        }
    }
}

pub mod catalog {
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct AvailableModelsAddition {
        #[prost(string, repeated, tag = "1")]
        pub model_names: Vec<String>,
        #[prost(message, repeated, tag = "2")]
        pub models: Vec<AvailableModel>,
    }

    #[derive(Clone, PartialEq, ::prost::Message)]
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
        #[prost(string, optional, tag = "41")]
        pub vendor_name: Option<String>,
    }

    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct TooltipData {
        #[prost(string, optional, tag = "7")]
        pub markdown_content: Option<String>,
    }

    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct UsableModelsAddition {
        #[prost(message, repeated, tag = "1")]
        pub models: Vec<super::agent::v1::ModelDetails>,
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

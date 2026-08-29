use std::collections::BTreeMap;

use crate::{
    model::{PromptSpec, ToolDefinition},
    GatewayError, Result,
};

use super::{assets::runtime_expression, Mode, PromptAssets};

#[derive(Clone)]
pub struct PromptCompiler {
    assets: PromptAssets,
}

impl PromptCompiler {
    pub fn new(assets: PromptAssets) -> Self {
        Self { assets }
    }

    pub fn runtime_message(&self, mode: Mode, values: &BTreeMap<&str, String>) -> Result<String> {
        render(&self.assets.mode(mode).runtime, values)
    }

    pub fn prompt_spec(
        &self,
        mode: Mode,
        model_id: &str,
        display_name: Option<&str>,
        dynamic_tools: &[ToolDefinition],
        suppress_subagent_progress: bool,
        supports_image_generation: bool,
    ) -> Result<PromptSpec> {
        let mut tools = self.tools(mode, suppress_subagent_progress);
        let mut dynamic_tools = dynamic_tools.to_vec();
        dynamic_tools.sort_by(|left, right| left.name.cmp(&right.name));
        append_dynamic_tools(&mut tools, dynamic_tools)?;
        if !supports_image_generation {
            tools.retain(|tool| tool.name != "GenerateImage");
        }
        // SemanticSearch is degraded locally — keep catalog entry but tools dispatch falls back.
        let fake_model_name = display_name.unwrap_or(model_id);
        Ok(PromptSpec {
            instructions: self
                .assets
                .mode(mode)
                .prompt
                .replace("{{FAKE_MODEL_NAME}}", fake_model_name),
            tools,
        })
    }

    pub fn embedded() -> Result<Self> {
        Ok(Self::new(crate::prompting::PromptAssets::embedded()?))
    }

    fn tools(&self, mode: Mode, suppress_subagent_progress: bool) -> Vec<ToolDefinition> {
        let mut tools = self.assets.mode(mode).tools.clone();
        if mode == Mode::Subagent && suppress_subagent_progress {
            tools.retain(|tool| tool.name != "UpdateCurrentStep");
        }
        tools
    }
}

fn render(template: &str, values: &BTreeMap<&str, String>) -> Result<String> {
    let expression = runtime_expression();
    let mut output = String::with_capacity(template.len());
    let mut cursor = 0;
    for capture in expression.captures_iter(template) {
        let token = capture.get(0).expect("runtime template token");
        let name = &capture[1];
        let value = values
            .get(name)
            .ok_or_else(|| GatewayError::Protocol(format!("runtime template value is missing: {name}")))?;
        output.push_str(&template[cursor..token.start()]);
        output.push_str(value);
        cursor = token.end();
    }
    output.push_str(&template[cursor..]);
    Ok(output.trim().to_string())
}

fn append_dynamic_tools(
    tools: &mut Vec<ToolDefinition>,
    additions: Vec<ToolDefinition>,
) -> Result<()> {
    for tool in additions {
        if tools.iter().any(|existing| existing.name == tool.name) {
            return Err(GatewayError::Protocol(format!(
                "dynamic MCP tool conflicts with a mode tool: {}",
                tool.name
            )));
        }
        tools.push(tool);
    }
    Ok(())
}

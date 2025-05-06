use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
#[allow(non_camel_case_types)]
pub enum Role {
    user,
    model,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash)]
pub struct InlineData {
    mime_type: String,
    data: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash)]
#[allow(non_camel_case_types)]
pub enum Language {
    ///Unspecified language. This value should not be used.
    LANGUAGE_UNSPECIFIED,
    ///Python >= 3.10, with numpy and simpy available.
    PYTHON,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash)]
pub struct ExecutableCode {
    language: Language,
    code: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash)]
pub struct FunctionCall {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    args: Option<Value>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash)]
pub struct FunctionResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    name: String,
    response: Value,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash)]
#[allow(non_snake_case)]
pub struct FileData {
    #[serde(skip_serializing_if = "Option::is_none")]
    mimeType: Option<String>,
    fileUrl: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash)]
#[allow(non_camel_case_types)]
pub enum Outcome {
    /// Unspecified status. This value should not be used.
    OUTCOME_UNSPECIFIED,
    /// Code execution completed successfully.
    OUTCOME_OK,
    /// Code execution finished but with a failure. `stderr` should contain the reason.
    OUTCOME_FAILED,
    /// Code execution ran for too long, and was cancelled.
    /// There may or may not be a partial output present.
    OUTCOME_DEADLINE_EXCEEDED,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash)]
pub struct CodeExecuteResult {
    outcome: Outcome,
    #[serde(skip_serializing_if = "Option::is_none")]
    output: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash)]
#[allow(non_camel_case_types)]
pub enum Part {
    text(String),
    #[serde(alias = "inlineData")]
    inline_data(InlineData),
    #[serde(alias = "executableCode")]
    executable_code(ExecutableCode),
    #[serde(alias = "codeExecutionResult")]
    code_execution_result(CodeExecuteResult),
    functionCall(FunctionCall),
    functionResponse(FunctionResponse),
    fileData(FileData),
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
pub struct Chat {
    role: Role,
    parts: Vec<Part>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
pub struct SystemInstruction {
    parts: Vec<Part>,
}
impl SystemInstruction {
    pub fn from_str(prompt: impl Into<String>) -> Self {
        Self {
            parts: vec![Part::text(prompt.into())],
        }
    }
}

#[derive(Serialize, Deserialize, Hash, Clone)]
pub struct GeminiRequestBody {
    pub system_instruction: Option<SystemInstruction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    pub contents: Vec<Chat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
#[allow(non_camel_case_types)]
pub enum Tool {
    /// Generally it can be `Tool::google_search(json!({}))`
    google_search(Value),
    /// It is of form `Tool::function_calling(`[functionDeclaration](https://ai.google.dev/gemini-api/docs/function-calling?example=meeting)`)`
    functionDeclarations(Vec<Value>),
    /// Generally it can be `Tool::code_execution(json!({}))`,
    code_execution(Value),
}

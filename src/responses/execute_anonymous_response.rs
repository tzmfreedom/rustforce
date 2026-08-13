use serde::{Deserialize, Serialize};

/// Response from the Tooling API's executeAnonymous endpoint.
///
/// When `compiled` is `false`, `success` will also be `false` and `compile_problem`
/// will contain the compilation error. When `compiled` is `true` but `success` is
/// `false`, the code compiled but threw a runtime exception.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteAnonymousResult {
    /// Whether the Apex code compiled successfully.
    pub compiled: bool,
    /// Compilation error message, if any.
    pub compile_problem: Option<String>,
    /// Whether the Apex code executed without runtime errors.
    pub success: bool,
    /// Line number of the error (-1 if no error).
    pub line: i32,
    /// Column number of the error (-1 if no error).
    pub column: i32,
    /// Runtime exception message, if any.
    pub exception_message: Option<String>,
    /// Runtime exception stack trace, if any.
    pub exception_stack_trace: Option<String>,
}

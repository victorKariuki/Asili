#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Span {
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Diagnostic {
    pub code: &'static str,
    pub stage: &'static str,
    pub message: String,
    pub file: Option<String>,
    pub span: Option<Span>,
    pub context_map: Option<ContextMap>,
}

impl Diagnostic {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            stage: "jumla",
            message: message.into(),
            file: None,
            span: None,
            context_map: None,
        }
    }

    pub fn with_stage(mut self, stage: &'static str) -> Self {
        self.stage = stage;
        self
    }

    pub fn with_file(mut self, file: impl Into<String>) -> Self {
        self.file = Some(file.into());
        self
    }

    pub fn with_span(mut self, line: usize, column: usize) -> Self {
        self.span = Some(Span { line, column });
        self
    }

    pub fn with_span_span(mut self, span: &Span) -> Self {
        self.span = Some(span.clone());
        self
    }

    pub fn with_context_map(mut self, context_map: ContextMap) -> Self {
        self.context_map = Some(context_map);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct ContextMap {
    pub symbol: String,
    pub created_at: Option<Span>,
    pub moved_at: Option<Span>,
    pub borrowed_at: Vec<Span>,
    pub dropped_at: Option<Span>,
}

/// Main file with the error logging logic. Transforms all the compilation errors to ariadne messages.
use std::ops::Range;

/// Represents a single diagnostic error
pub struct Diagnostic {
    pub kind: DiagnosticKind,
    pub message: String,
    pub labels: Vec<DiagnosticLabel>,
    pub notes: Vec<String>,
    pub help: Option<String>,
}

pub enum DiagnosticKind {
    Error,
    Warning,
}

pub struct DiagnosticLabel {
    pub position: Range<usize>,
    pub message: String,
    pub is_primary: bool,
}

/// Main Diagnostic functionality.
impl Diagnostic {
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            kind: DiagnosticKind::Error,
            message: message.into(),
            labels: Vec::new(),
            notes: Vec::new(),
            help: None,
        }
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            kind: DiagnosticKind::Warning,
            message: message.into(),
            labels: Vec::new(),
            notes: Vec::new(),
            help: None,
        }
    }

    pub fn with_label(mut self, span: Range<usize>, message: impl Into<String>) -> Self {
        if !span.is_empty() {
            self.labels.push(DiagnosticLabel {
                position: span,
                message: message.into(),
                is_primary: true,
            });
        }
        self
    }

    pub fn with_secondary_label(mut self, span: Range<usize>, message: impl Into<String>) -> Self {
        if !span.is_empty() {
            self.labels.push(DiagnosticLabel {
                position: span,
                message: message.into(),
                is_primary: false,
            });
        }
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn with_help(mut self, note: impl Into<String>) -> Self {
        self.help = Some(note.into());
        self
    }
}

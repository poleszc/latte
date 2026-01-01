use ariadne::{ColorGenerator, Config, Label, Report, ReportKind, Source};
/// Main file with the error logging logic. Transforms all the compilation errors to ariadne messages.
use std::ops::Range;

pub trait ToDiagnostic {
    fn to_diagnostic(&self) -> Diagnostic;
}

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

impl Diagnostic {
    /// Render the diagnostic message with Ariadne.
    pub fn render(&self, file_path: &str, source: &str) {
        let source_cache = Source::from(source);
        let config = Config::default().with_color(true).with_underlines(true);
        let report_kind = match self.kind {
            DiagnosticKind::Error => ReportKind::Error,
            DiagnosticKind::Warning => ReportKind::Warning,
        };

        // Location of the error is the position of the first label.
        let offset = self
            .labels
            .first()
            .map(|label| label.position.start)
            .unwrap_or(0);
        let mut report = Report::build(report_kind, file_path, offset)
            .with_config(config)
            .with_message(&self.message);
        let mut colors = ColorGenerator::new();

        for label in &self.labels {
            let color = colors.next();
            let ariadne_label = Label::new((file_path, label.position.clone()))
                .with_message(&label.message)
                .with_color(color);
            report = report.with_label(ariadne_label);
        }

        for note in &self.notes {
            report = report.with_note(note);
        }

        if let Some(help) = &self.help {
            report = report.with_help(help);
        }

        report
            .finish()
            .eprint((file_path, source_cache))
            .expect("Failed to report an ariadne error")
    }
}

// ======================
// Basic functionality.
// ======================
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

    pub fn label(mut self, span: Range<usize>, message: impl Into<String>) -> Self {
        if !span.is_empty() {
            self.labels.push(DiagnosticLabel {
                position: span,
                message: message.into(),
                is_primary: true,
            });
        }
        self
    }

    pub fn secondary_label(mut self, span: Range<usize>, message: impl Into<String>) -> Self {
        if !span.is_empty() {
            self.labels.push(DiagnosticLabel {
                position: span,
                message: message.into(),
                is_primary: false,
            });
        }
        self
    }

    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn help(mut self, note: impl Into<String>) -> Self {
        self.help = Some(note.into());
        self
    }
}

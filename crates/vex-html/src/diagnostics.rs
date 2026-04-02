// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Lightweight parse diagnostics for malformed HTML inputs.

/// Severity of a parse diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Info,
    Warning,
    Error,
}

/// A parser diagnostic entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseDiagnostic {
    pub severity: DiagnosticSeverity,
    pub message: String,
}

/// Heuristic diagnostics pass over raw HTML input.
///
/// This is intentionally conservative and never blocks parsing.
pub fn diagnose_html(input: &str) -> Vec<ParseDiagnostic> {
    let mut out = Vec::new();

    if input.contains('\0') {
        out.push(ParseDiagnostic {
            severity: DiagnosticSeverity::Warning,
            message: "input contains NUL characters; replaced during parsing".to_string(),
        });
    }

    let lt = input.matches('<').count();
    let gt = input.matches('>').count();
    if lt != gt {
        out.push(ParseDiagnostic {
            severity: DiagnosticSeverity::Warning,
            message: format!(
                "tag delimiter imbalance detected: '<' count = {lt}, '>' count = {gt}"
            ),
        });
    }

    if input.contains("</html") && !input.to_ascii_lowercase().contains("<html") {
        out.push(ParseDiagnostic {
            severity: DiagnosticSeverity::Info,
            message: "closing </html> tag present without opening <html> tag".to_string(),
        });
    }

    if input.len() > 5 * 1024 * 1024 {
        out.push(ParseDiagnostic {
            severity: DiagnosticSeverity::Info,
            message: "very large HTML payload; incremental parse recommended".to_string(),
        });
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_nul_bytes() {
        let d = diagnose_html("<p>a\0b</p>");
        assert!(d.iter().any(|x| x.message.contains("NUL")));
    }

    #[test]
    fn reports_delimiter_imbalance() {
        let d = diagnose_html("<div><span");
        assert!(d.iter().any(|x| x.message.contains("imbalance")));
    }
}

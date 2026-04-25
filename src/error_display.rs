use crate::lexer::SourceLocation;

/// Formats an error message with source code context
pub struct ErrorFormatter<'a> {
    source: &'a str,
    filename: &'a str,
}

impl<'a> ErrorFormatter<'a> {
    pub fn new(source: &'a str, filename: &'a str) -> Self {
        ErrorFormatter { source, filename }
    }

    /// Format an error with source context
    pub fn format_error(
        &self,
        error_type: &str,
        message: &str,
        location: &SourceLocation,
        hint: Option<&str>,
    ) -> String {
        let mut output = String::new();

        // Error header
        output.push_str(&format!("\x1b[1;31merror\x1b[0m: {}\n", message));

        // Location indicator
        output.push_str(&format!(
            "  \x1b[1;34m-->\x1b[0m {}:{}:{}\n",
            self.filename, location.line, location.column
        ));

        // Get the source line
        if let Some(line_content) = self.get_line(location.line) {
            let line_num_width = location.line.to_string().len();

            // Empty line with gutter
            output.push_str(&format!("{} \x1b[1;34m|\x1b[0m\n", " ".repeat(line_num_width)));

            // Line number and content
            output.push_str(&format!(
                "\x1b[1;34m{}\x1b[0m \x1b[1;34m|\x1b[0m {}\n",
                location.line, line_content
            ));

            // Caret line
            let spaces = " ".repeat(location.column - 1);
            output.push_str(&format!(
                "{} \x1b[1;34m|\x1b[0m {}\x1b[1;31m^\x1b[0m {}\n",
                " ".repeat(line_num_width),
                spaces,
                error_type
            ));

            // Hint if provided
            if let Some(hint_text) = hint {
                output.push_str(&format!(
                    "{} \x1b[1;34m|\x1b[0m\n",
                    " ".repeat(line_num_width)
                ));
                output.push_str(&format!(
                    "{} \x1b[1;34m=\x1b[0m \x1b[1mhint\x1b[0m: {}\n",
                    " ".repeat(line_num_width),
                    hint_text
                ));
            }
        }

        output
    }

    /// Get a specific line from the source code
    fn get_line(&self, line_num: usize) -> Option<String> {
        self.source
            .lines()
            .nth(line_num.saturating_sub(1))
            .map(|s| s.to_string())
    }
}

/// Trait for displaying errors with source context
pub trait FormattedError {
    fn error_type(&self) -> &str;
    fn message(&self) -> String;
    fn location(&self) -> &SourceLocation;
    fn hint(&self) -> Option<String> {
        None
    }

    fn format_with_source(&self, source: &str, filename: &str) -> String {
        let formatter = ErrorFormatter::new(source, filename);
        formatter.format_error(&self.error_type(), &self.message(), self.location(), self.hint().as_deref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_formatter() {
        let source = "int x := y;";
        let formatter = ErrorFormatter::new(source, "test.eris");

        let location = SourceLocation { line: 1, column: 10 };
        let formatted = formatter.format_error(
            "undeclared variable",
            "Undeclared variable 'y'",
            &location,
            Some("make sure the variable is declared before use"),
        );

        // Check for error message (accounting for ANSI codes)
        assert!(formatted.contains("Undeclared variable 'y'"));
        assert!(formatted.contains("test.eris:1:10"));
        assert!(formatted.contains("int x := y;"));
        assert!(formatted.contains("^"));
        assert!(formatted.contains("undeclared variable"));
    }

    #[test]
    fn test_multiline_source() {
        let source = "int x := 10;\nint y := z;\nint a := 5;";
        let formatter = ErrorFormatter::new(source, "test.eris");

        let location = SourceLocation { line: 2, column: 10 };
        let formatted = formatter.format_error(
            "undeclared variable",
            "Undeclared variable 'z'",
            &location,
            None,
        );

        assert!(formatted.contains("int y := z;"));
        assert!(!formatted.contains("int x := 10;"));
    }
}

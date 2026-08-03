//! Shared PDF helpers used by both the ChromaDB document pipeline and the
//! PageIndex ingestion pipeline.

use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use tempfile::NamedTempFile;

/// Extract text from a PDF using `pdftotext`, optionally restricted to a page range.
///
/// `page_range` is `Some((start, end))` using 1-based, inclusive page numbers, mirroring
/// the `-f`/`-l` flags of `pdftotext`.
pub(crate) fn extract_pdf_text(
    data: &[u8],
    page_range: Option<(u32, u32)>,
) -> Result<(String, HashMap<String, String>), String> {
    // Create a temporary file to write the PDF data to
    let mut temp_file =
        NamedTempFile::new().map_err(|e| format!("Failed to create temp file: {}", e))?;
    temp_file
        .write_all(data)
        .map_err(|e| format!("Failed to write PDF data: {}", e))?;

    let temp_path = temp_file.path().to_owned();

    let mut cmd = Command::new("pdftotext");
    cmd.arg("-layout").arg("-enc").arg("UTF-8");

    if let Some((start, end)) = page_range {
        cmd.arg("-f")
            .arg(start.to_string())
            .arg("-l")
            .arg(end.to_string());
    }

    let output = cmd
        .arg(&temp_path)
        .arg("-") // Output to stdout
        .output()
        .map_err(|e| format!("Failed to execute pdftotext: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("pdftotext failed: {}", stderr));
    }

    let text = String::from_utf8(output.stdout)
        .map_err(|e| format!("Invalid UTF-8 output from pdftotext: {}", e))?;

    let mut metadata = HashMap::new();
    metadata.insert("file_type".to_string(), "pdf".to_string());
    metadata.insert("parser".to_string(), "pdftotext".to_string());

    Ok((text, metadata))
}

/// Formats plain text as markdown by normalizing whitespace/blank lines.
pub(crate) fn format_text_as_markdown(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let mut markdown = String::new();
    let mut prev_empty = false;

    for line in lines {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            if !prev_empty {
                markdown.push_str("\n\n");
                prev_empty = true;
            }
        } else {
            // Preserve the line, but ensure proper spacing
            markdown.push_str(trimmed);
            markdown.push('\n');
            prev_empty = false;
        }
    }

    markdown.trim().to_string()
}

/// Count the number of pages in a PDF file on disk using `pdfinfo`.
pub(crate) fn count_pdf_pages(path: &Path) -> Result<u32, String> {
    let output = Command::new("pdfinfo")
        .arg(path)
        .output()
        .map_err(|e| format!("Failed to execute pdfinfo: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("pdfinfo failed: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if let Some(rest) = line.strip_prefix("Pages:") {
            let count = rest
                .trim()
                .parse::<u32>()
                .map_err(|e| format!("Failed to parse page count from pdfinfo: {}", e))?;
            return Ok(count);
        }
    }

    Err("pdfinfo output did not contain a 'Pages:' line".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_text_as_markdown_collapses_blank_lines() {
        let input = "Line one\n\n\n\nLine two\n   \nLine three";
        let result = format_text_as_markdown(input);
        assert!(result.contains("Line one"));
        assert!(result.contains("Line two"));
        assert!(result.contains("Line three"));
        // Runs of blank lines collapse to a single blank-line separator
        assert!(!result.contains("\n\n\n\n"));
    }

    #[test]
    fn test_format_text_as_markdown_exact_output() {
        // Lines are trimmed, and a run of blank lines becomes a single "\n\n" -
        // on top of the "\n" the preceding content line already added, which is
        // why paragraphs come out separated by three newlines.
        assert_eq!(
            format_text_as_markdown("  A  \n\n\n  B  \nC"),
            "A\n\n\nB\nC"
        );
    }

    #[test]
    fn test_format_text_as_markdown_on_blank_input() {
        assert_eq!(format_text_as_markdown(""), "");
        assert_eq!(format_text_as_markdown("\n\n\n"), "");
        assert_eq!(format_text_as_markdown("   \t  "), "");
    }

    #[test]
    fn test_format_text_as_markdown_single_line_is_unchanged() {
        assert_eq!(format_text_as_markdown("just one line"), "just one line");
    }

    #[test]
    fn test_count_pdf_pages_reports_a_missing_file() {
        let dir = tempfile::tempdir().unwrap();

        let err = count_pdf_pages(&dir.path().join("absent.pdf")).unwrap_err();

        // Either pdfinfo is missing entirely, or it ran and rejected the path -
        // both surface as an error string rather than a panic.
        assert!(
            err.starts_with("Failed to execute pdfinfo:") || err.starts_with("pdfinfo failed:"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn test_count_pdf_pages_reports_a_file_that_is_not_a_pdf() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("not.pdf");
        std::fs::write(&path, b"definitely not a PDF").unwrap();

        let err = count_pdf_pages(&path).unwrap_err();
        assert!(!err.is_empty());
    }

    #[test]
    fn test_extract_pdf_text_rejects_data_that_is_not_a_pdf() {
        let err = extract_pdf_text(b"definitely not a PDF", None).unwrap_err();
        assert!(!err.is_empty());
    }

    #[test]
    fn test_extract_pdf_text_rejects_a_page_range_over_invalid_data() {
        // The page range only adds -f/-l flags; invalid input still fails.
        let err = extract_pdf_text(b"not a pdf", Some((2, 5))).unwrap_err();
        assert!(!err.is_empty());
    }
}

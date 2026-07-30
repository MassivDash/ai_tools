//! "Fast path" table-of-contents extraction from a PDF's embedded bookmarks/outline
//! (the `/Outlines` catalog entry), used before falling back to LLM-based structure
//! inference. Built on top of `lopdf`'s `Document::get_toc()` helper, which already
//! walks the `First`/`Next` outline tree and resolves destinations to page numbers.

use crate::api::pageindex::types::PageIndexNode;
use std::path::Path;

/// Attempt to build a hierarchical node tree from a PDF's embedded outline.
///
/// Returns `None` if the PDF has no outline, the outline can't be parsed, or it
/// contains no usable (non-blank) titles - callers should fall back to LLM-based
/// structure inference in that case.
pub(crate) fn extract_bookmark_tree(
    pdf_path: &Path,
    total_pages: u32,
) -> Option<Vec<PageIndexNode>> {
    let doc = lopdf::Document::load(pdf_path).ok()?;
    let toc = doc.get_toc().ok()?;

    let entries: Vec<(usize, String, u32)> = toc
        .toc
        .into_iter()
        .filter(|t| !t.title.trim().is_empty())
        .map(|t| (t.level, t.title.trim().to_string(), t.page as u32))
        .collect();

    if entries.is_empty() {
        return None;
    }

    let mut counter = 0u32;
    let mut idx = 0usize;
    let mut nodes = build_level(&entries, &mut idx, &mut counter);
    fill_page_ends(&mut nodes, total_pages);

    Some(nodes)
}

/// Recursively group a flat, depth-annotated list of (level, title, page_start) entries
/// into a nested tree. Entries at the level of the first item in `entries[*idx..]` become
/// siblings; a run of deeper-level entries immediately following one becomes its children.
fn build_level(
    entries: &[(usize, String, u32)],
    idx: &mut usize,
    counter: &mut u32,
) -> Vec<PageIndexNode> {
    let mut nodes = Vec::new();
    if *idx >= entries.len() {
        return nodes;
    }
    let level = entries[*idx].0;

    while *idx < entries.len() && entries[*idx].0 == level {
        let (_, title, page_start) = entries[*idx].clone();
        *idx += 1;
        *counter += 1;
        let id = format!("n{}", counter);

        let children = if *idx < entries.len() && entries[*idx].0 > level {
            build_level(entries, idx, counter)
        } else {
            Vec::new()
        };

        nodes.push(PageIndexNode {
            id,
            title,
            page_start,
            page_end: page_start, // filled in by fill_page_ends
            summary: String::new(),
            children,
        });
    }

    nodes
}

/// Fill in `page_end` for every node: the page before the next sibling's start, or
/// `bound` (the parent's page_end / document's last page) for the last child at each level.
pub(crate) fn fill_page_ends(nodes: &mut [PageIndexNode], bound: u32) {
    let n = nodes.len();
    for i in 0..n {
        let end = if i + 1 < n {
            nodes[i + 1]
                .page_start
                .saturating_sub(1)
                .max(nodes[i].page_start)
        } else {
            bound.max(nodes[i].page_start)
        };
        nodes[i].page_end = end;
        if !nodes[i].children.is_empty() {
            fill_page_ends(&mut nodes[i].children, end);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_level_and_fill_page_ends_nests_by_depth() {
        let entries = vec![
            (1, "Chapter 1".to_string(), 1u32),
            (2, "1.1 Intro".to_string(), 2u32),
            (2, "1.2 Details".to_string(), 5u32),
            (1, "Chapter 2".to_string(), 10u32),
        ];

        let mut counter = 0u32;
        let mut idx = 0usize;
        let mut nodes = build_level(&entries, &mut idx, &mut counter);
        fill_page_ends(&mut nodes, 20);

        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0].title, "Chapter 1");
        assert_eq!(nodes[0].page_start, 1);
        assert_eq!(nodes[0].page_end, 9); // right before Chapter 2
        assert_eq!(nodes[0].children.len(), 2);
        assert_eq!(nodes[0].children[0].page_end, 4); // right before 1.2
        assert_eq!(nodes[0].children[1].page_end, 9); // inherits parent's end

        assert_eq!(nodes[1].title, "Chapter 2");
        assert_eq!(nodes[1].page_start, 10);
        assert_eq!(nodes[1].page_end, 20); // inherits document bound
    }
}

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
    use lopdf::{dictionary, Document, Object};

    /// A bookmark to place in the generated PDF's `/Outlines` tree.
    struct Bookmark {
        title: &'static str,
        /// 0-based index into the generated pages.
        page: usize,
        children: Vec<Bookmark>,
    }

    fn bookmark(title: &'static str, page: usize, children: Vec<Bookmark>) -> Bookmark {
        Bookmark {
            title,
            page,
            children,
        }
    }

    /// Write a minimal, valid PDF with `page_count` blank pages and the given
    /// `/Outlines` tree, and return its path (kept alive by the returned dir).
    ///
    /// Real bookmark extraction is the whole point of this module, so the tests
    /// exercise it against an actual PDF rather than a stubbed `lopdf` document.
    fn pdf_with_bookmarks(
        page_count: usize,
        bookmarks: Vec<Bookmark>,
    ) -> (tempfile::TempDir, std::path::PathBuf) {
        let mut doc = Document::with_version("1.5");

        let pages_id = doc.new_object_id();
        let page_ids: Vec<lopdf::ObjectId> = (0..page_count)
            .map(|_| {
                doc.add_object(dictionary! {
                    "Type" => "Page",
                    "Parent" => pages_id,
                    "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
                })
            })
            .collect();

        doc.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages",
                "Count" => page_count as i64,
                "Kids" => page_ids.iter().map(|id| Object::Reference(*id)).collect::<Vec<_>>(),
            }),
        );

        // Outline items are linked with First/Next, and destinations point at a
        // page reference - the shape lopdf's `get_toc` walks.
        fn add_items(
            doc: &mut Document,
            page_ids: &[lopdf::ObjectId],
            items: &[Bookmark],
        ) -> Option<(lopdf::ObjectId, lopdf::ObjectId)> {
            let ids: Vec<lopdf::ObjectId> = items.iter().map(|_| doc.new_object_id()).collect();

            for (i, item) in items.iter().enumerate() {
                let mut dict = dictionary! {
                    "Title" => Object::string_literal(item.title),
                    "Dest" => vec![
                        Object::Reference(page_ids[item.page]),
                        Object::Name(b"Fit".to_vec()),
                    ],
                };
                if i + 1 < ids.len() {
                    dict.set("Next", Object::Reference(ids[i + 1]));
                }
                if i > 0 {
                    dict.set("Prev", Object::Reference(ids[i - 1]));
                }
                if let Some((first_child, last_child)) = add_items(doc, page_ids, &item.children) {
                    dict.set("First", Object::Reference(first_child));
                    dict.set("Last", Object::Reference(last_child));
                    dict.set("Count", item.children.len() as i64);
                }
                doc.objects.insert(ids[i], Object::Dictionary(dict));
            }

            match (ids.first(), ids.last()) {
                (Some(first), Some(last)) => Some((*first, *last)),
                _ => None,
            }
        }

        let mut catalog = dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        };
        if let Some((first, last)) = add_items(&mut doc, &page_ids, &bookmarks) {
            let outlines_id = doc.add_object(dictionary! {
                "Type" => "Outlines",
                "First" => Object::Reference(first),
                "Last" => Object::Reference(last),
                "Count" => bookmarks.len() as i64,
            });
            catalog.set("Outlines", Object::Reference(outlines_id));
        }

        let catalog_id = doc.add_object(catalog);
        doc.trailer.set("Root", catalog_id);

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("book.pdf");
        doc.save(&path).unwrap();
        (dir, path)
    }

    #[test]
    fn test_extract_bookmark_tree_returns_none_for_a_non_pdf_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("not.pdf");
        std::fs::write(&path, b"this is definitely not a PDF").unwrap();

        assert!(extract_bookmark_tree(&path, 10).is_none());
    }

    #[test]
    fn test_extract_bookmark_tree_returns_none_for_a_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        assert!(extract_bookmark_tree(&dir.path().join("absent.pdf"), 10).is_none());
    }

    #[test]
    fn test_extract_bookmark_tree_returns_none_when_the_pdf_has_no_outline() {
        let (_dir, path) = pdf_with_bookmarks(3, Vec::new());

        // A structurally valid PDF, but with nothing to build a tree from.
        assert!(extract_bookmark_tree(&path, 3).is_none());
    }

    #[test]
    fn test_extract_bookmark_tree_returns_none_when_every_title_is_blank() {
        let (_dir, path) = pdf_with_bookmarks(3, vec![bookmark("   ", 0, vec![])]);

        assert!(extract_bookmark_tree(&path, 3).is_none());
    }

    #[test]
    fn test_extract_bookmark_tree_builds_a_flat_tree_with_page_ranges() {
        let (_dir, path) = pdf_with_bookmarks(
            20,
            vec![
                bookmark("Chapter 1", 0, vec![]),
                bookmark("Chapter 2", 9, vec![]),
            ],
        );

        let nodes = extract_bookmark_tree(&path, 20).expect("expected a bookmark tree");

        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0].id, "n1");
        assert_eq!(nodes[0].title, "Chapter 1");
        assert_eq!(nodes[0].page_start, 1);
        assert_eq!(nodes[0].page_end, 9); // right before Chapter 2
        assert!(nodes[0].summary.is_empty());
        assert_eq!(nodes[1].id, "n2");
        assert_eq!(nodes[1].page_start, 10);
        assert_eq!(nodes[1].page_end, 20); // the document bound
    }

    #[test]
    fn test_extract_bookmark_tree_nests_child_bookmarks() {
        let (_dir, path) = pdf_with_bookmarks(
            30,
            vec![
                bookmark(
                    "  Chapter 1  ",
                    0,
                    vec![
                        bookmark("1.1 Intro", 1, vec![]),
                        bookmark("1.2 Deep", 4, vec![]),
                    ],
                ),
                bookmark("Chapter 2", 19, vec![]),
            ],
        );

        let nodes = extract_bookmark_tree(&path, 30).expect("expected a bookmark tree");

        assert_eq!(nodes.len(), 2);
        // Titles are trimmed.
        assert_eq!(nodes[0].title, "Chapter 1");
        assert_eq!(nodes[0].page_end, 19);

        let children = &nodes[0].children;
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].title, "1.1 Intro");
        assert_eq!(children[0].page_start, 2);
        assert_eq!(children[0].page_end, 4); // right before 1.2
        assert_eq!(children[1].title, "1.2 Deep");
        assert_eq!(children[1].page_start, 5);
        assert_eq!(children[1].page_end, 19); // inherits the parent's end

        assert_eq!(nodes[1].title, "Chapter 2");
        assert_eq!(nodes[1].page_end, 30);
        assert!(nodes[1].children.is_empty());
    }

    #[test]
    fn test_extract_bookmark_tree_skips_blank_titles_but_keeps_the_rest() {
        let (_dir, path) = pdf_with_bookmarks(
            10,
            vec![
                bookmark("Real Chapter", 0, vec![]),
                bookmark("  ", 4, vec![]),
                bookmark("Another Chapter", 6, vec![]),
            ],
        );

        let nodes = extract_bookmark_tree(&path, 10).expect("expected a bookmark tree");

        let titles: Vec<&str> = nodes.iter().map(|n| n.title.as_str()).collect();
        assert_eq!(titles, vec!["Real Chapter", "Another Chapter"]);
        // Page ranges are computed from the surviving entries only.
        assert_eq!(nodes[0].page_end, 6);
    }

    #[test]
    fn test_build_level_on_empty_entries_returns_no_nodes() {
        let mut idx = 0usize;
        let mut counter = 0u32;
        assert!(build_level(&[], &mut idx, &mut counter).is_empty());
        assert_eq!(counter, 0);
    }

    #[test]
    fn test_fill_page_ends_never_produces_an_end_before_the_start() {
        // Two sections claiming the same start page: the first must not get an
        // end of 0 (page_start - 1) but be clamped to its own start.
        let mut nodes = vec![
            PageIndexNode {
                id: "n1".into(),
                title: "A".into(),
                page_start: 5,
                page_end: 0,
                summary: String::new(),
                children: vec![],
            },
            PageIndexNode {
                id: "n2".into(),
                title: "B".into(),
                page_start: 5,
                page_end: 0,
                summary: String::new(),
                children: vec![],
            },
        ];

        fill_page_ends(&mut nodes, 3);

        assert_eq!(nodes[0].page_end, 5);
        // The bound is smaller than the start, so the start wins.
        assert_eq!(nodes[1].page_end, 5);
    }

    #[test]
    fn test_fill_page_ends_on_an_empty_slice_is_a_no_op() {
        let mut nodes: Vec<PageIndexNode> = Vec::new();
        fill_page_ends(&mut nodes, 10);
        assert!(nodes.is_empty());
    }

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

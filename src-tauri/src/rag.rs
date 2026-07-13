//! Local RAG: load .txt / .pdf manuals from knowledge_base/ and retrieve snippets.

use crate::error::{SmaError, SmaResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentChunk {
    pub source: String,
    pub content: String,
}

#[derive(Debug, Default, Clone)]
pub struct KnowledgeBase {
    pub root: PathBuf,
    pub chunks: Vec<DocumentChunk>,
}

impl KnowledgeBase {
    pub fn empty() -> Self {
        Self {
            root: PathBuf::from("knowledge_base"),
            chunks: Vec::new(),
        }
    }

    pub fn load_from_dir(path: impl AsRef<Path>) -> SmaResult<Self> {
        let root = path.as_ref().to_path_buf();
        if !root.exists() {
            return Err(SmaError::KnowledgeBase(format!(
                "Directory not found: {}",
                root.display()
            )));
        }

        let mut chunks = Vec::new();

        for entry in WalkDir::new(&root).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();

            let text = match ext.as_str() {
                "txt" | "md" => std::fs::read_to_string(path)
                    .map_err(|e| SmaError::KnowledgeBase(e.to_string()))?,
                "pdf" => extract_pdf(path)?,
                _ => continue,
            };

            let source = path
                .strip_prefix(&root)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string();

            for piece in chunk_text(&text, 800) {
                if piece.trim().len() > 40 {
                    chunks.push(DocumentChunk {
                        source: source.clone(),
                        content: piece,
                    });
                }
            }
        }

        Ok(Self { root, chunks })
    }

    pub fn search(&self, query: &str, top_k: usize) -> Vec<DocumentChunk> {
        let q = query.to_lowercase();
        let terms: Vec<&str> = q.split_whitespace().filter(|t| t.len() > 2).collect();

        let mut scored: Vec<(usize, &DocumentChunk)> = self
            .chunks
            .iter()
            .map(|c| {
                let lower = c.content.to_lowercase();
                let score = terms
                    .iter()
                    .filter(|t| lower.contains(*t))
                    .count()
                    .saturating_mul(10)
                    + if lower.contains(&q) { 25 } else { 0 };
                (score, c)
            })
            .filter(|(s, _)| *s > 0)
            .collect();

        scored.sort_by(|a, b| b.0.cmp(&a.0));
        scored
            .into_iter()
            .take(top_k)
            .map(|(_, c)| c.clone())
            .collect()
    }

    pub fn context_for(&self, query: &str, top_k: usize) -> String {
        let hits = self.search(query, top_k);
        if hits.is_empty() {
            return "No relevant knowledge-base passages found.".into();
        }
        hits.iter()
            .map(|h| format!("[source: {}]\n{}", h.source, h.content))
            .collect::<Vec<_>>()
            .join("\n\n---\n\n")
    }
}

fn chunk_text(text: &str, max_chars: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut buf = String::new();
    for para in text.split(['\n', '\r']) {
        let p = para.trim();
        if p.is_empty() {
            continue;
        }
        if buf.len() + p.len() + 1 > max_chars && !buf.is_empty() {
            out.push(buf.clone());
            buf.clear();
        }
        if !buf.is_empty() {
            buf.push('\n');
        }
        buf.push_str(p);
    }
    if !buf.is_empty() {
        out.push(buf);
    }
    out
}

fn extract_pdf(path: &Path) -> SmaResult<String> {
    pdf_extract::extract_text(path).map_err(|e| SmaError::KnowledgeBase(format!("PDF {}: {e}", path.display())))
}

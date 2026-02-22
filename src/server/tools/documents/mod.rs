//! Document management tool handlers.
//!
//! Handles document CRUD (create, update, search, read).

use std::sync::Arc;

use serde_json::{json, Value};

use crate::db::traits::{CreateDocumentInput, DocumentRepo};
use crate::types::UserId;

use super::haiku::haiku_summarize;
use crate::server::state::AppState;

mod tests;

/// Generate a kebab-case ref_tag from a title.
pub(crate) fn title_to_ref_tag(title: &str) -> String {
    title
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .collect()
}

/// Spawn a background task to generate and store a document summary.
fn spawn_summary_task(doc_repo: Arc<dyn DocumentRepo>, doc_id: uuid::Uuid, content: String) {
    tokio::spawn(async move {
        if let Some(summary) = haiku_summarize(&content).await {
            if let Err(e) = doc_repo.update_document_summary(doc_id, summary).await {
                tracing::error!("Failed to update document summary: {}", e);
            }
        }
    });
}

pub(crate) async fn execute_create_doc(input: &Value, state: &AppState, user_id: UserId) -> Value {
    let doc_repo = Arc::clone(&state.repos().documents);

    let Some(title) = input["title"].as_str() else {
        return json!({ "error": "title is required" });
    };
    let Some(content) = input["content"].as_str() else {
        return json!({ "error": "content is required" });
    };

    let tags: Vec<String> = input["tags"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let ref_tag = title_to_ref_tag(title);

    match doc_repo
        .create_document(CreateDocumentInput {
            user_id: user_id.0,
            session_id: None,
            title: title.to_string(),
            content: content.to_string(),
            doc_type: "architecture".to_string(),
            ref_tag: ref_tag.clone(),
            tags,
        })
        .await
    {
        Ok(row) => {
            // Spawn background summary generation
            spawn_summary_task(Arc::clone(&doc_repo), row.id, content.to_string());

            json!({
                "doc_id": row.id.to_string(),
                "ref_tag": ref_tag,
                "title": title
            })
        }
        Err(e) => json!({ "error": e.to_string() }),
    }
}

pub(crate) async fn execute_update_doc(input: &Value, state: &AppState) -> Value {
    let doc_repo = Arc::clone(&state.repos().documents);

    let Some(id_str) = input["doc_id"].as_str() else {
        return json!({ "error": "doc_id is required" });
    };
    let Ok(doc_id) = uuid::Uuid::parse_str(id_str) else {
        return json!({ "error": format!("Invalid UUID: {}", id_str) });
    };

    let content = input["content"].as_str().map(String::from);
    let title = input["title"].as_str().map(String::from);
    let tags: Option<Vec<String>> = input["tags"].as_array().map(|arr| {
        arr.iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect()
    });

    match doc_repo
        .update_document(doc_id, content.clone(), title.clone(), tags)
        .await
    {
        Ok(row) => {
            // Spawn background summary regeneration using updated content
            let summary_content = content.unwrap_or(row.content.clone());
            spawn_summary_task(Arc::clone(&doc_repo), doc_id, summary_content);

            json!({
                "updated": true,
                "doc_id": doc_id.to_string(),
                "title": row.title
            })
        }
        Err(e) => json!({ "error": e.to_string() }),
    }
}

pub(crate) async fn execute_search_docs(input: &Value, state: &AppState, user_id: UserId) -> Value {
    let doc_repo = &*state.repos().documents;

    let Some(query) = input["query"].as_str() else {
        return json!({ "error": "query is required" });
    };

    match doc_repo.search_documents(user_id.0, query).await {
        Ok(results) => {
            let items: Vec<Value> = results
                .into_iter()
                .map(|r| {
                    json!({
                        "id": r.id.to_string(),
                        "title": r.title,
                        "ref_tag": r.ref_tag,
                        "summary": r.summary,
                        "snippet": r.snippet
                    })
                })
                .collect();
            json!({ "results": items, "count": items.len() })
        }
        Err(e) => json!({ "error": e.to_string() }),
    }
}

pub(crate) async fn execute_read_document(input: &Value, state: &AppState) -> Value {
    let doc_repo = &*state.repos().documents;

    let Some(id_str) = input["document_id"].as_str() else {
        return json!({ "error": "document_id is required" });
    };

    let Ok(doc_id) = uuid::Uuid::parse_str(id_str) else {
        return json!({ "error": format!("Invalid UUID: {}", id_str) });
    };

    match doc_repo.get_document(doc_id).await {
        Ok(Some(doc)) => json!({
            "document_id": doc.id.to_string(),
            "title": doc.title,
            "content": doc.content,
            "doc_type": doc.doc_type,
            "ref_tag": doc.ref_tag,
            "tags": doc.tags,
            "summary": doc.summary,
        }),
        Ok(None) => json!({ "error": format!("Document not found: {}", id_str) }),
        Err(e) => json!({ "error": e.to_string() }),
    }
}


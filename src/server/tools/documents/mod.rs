//! Document management and structured output tool handlers.
//!
//! Handles document CRUD (create, update, search), PRD submission
//! with validation and markdown generation, and ticket validation.

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

pub(crate) async fn execute_submit_prd(input: &Value, state: &AppState, user_id: UserId) -> Value {
    let mut errors = Vec::new();

    // Validate required string fields
    let title = input["title"].as_str().unwrap_or("");
    if title.is_empty() {
        errors.push("Missing field: title".to_string());
    }
    let problem_statement = input["problem_statement"].as_str().unwrap_or("");
    if problem_statement.is_empty() {
        errors.push("Missing field: problem_statement".to_string());
    }
    let technical_approach = input["technical_approach"].as_str().unwrap_or("");
    if technical_approach.is_empty() {
        errors.push("Missing field: technical_approach".to_string());
    }

    // Validate required array fields
    let goals = input["goals"].as_array();
    if goals.is_none_or(|a| a.is_empty()) {
        errors.push("goals must have at least 1 entry".to_string());
    }
    let non_goals = input["non_goals"].as_array();
    if non_goals.is_none_or(|a| a.is_empty()) {
        errors.push("non_goals must have at least 1 entry".to_string());
    }
    let user_stories = input["user_stories"].as_array();
    if user_stories.is_none_or(|a| a.is_empty()) {
        errors.push("user_stories must have at least 1 entry".to_string());
    }

    // Validate milestones
    let milestones = input["milestones"].as_array();
    if milestones.is_none_or(|a| a.is_empty()) {
        errors.push("milestones must have at least 1 entry".to_string());
    } else if let Some(ms) = milestones {
        for (i, m) in ms.iter().enumerate() {
            if m["name"].as_str().unwrap_or("").is_empty() {
                errors.push(format!("milestones[{}] missing name", i));
            }
            if m["deliverables"].as_array().is_none_or(|a| a.is_empty()) {
                errors.push(format!(
                    "milestones[{}] must have at least 1 deliverable",
                    i
                ));
            }
        }
    }

    // Validate complexity
    let complexity = input["complexity"].as_str().unwrap_or("");
    if !matches!(complexity, "S" | "M" | "L" | "XL") {
        errors.push("complexity must be one of: S, M, L, XL".to_string());
    }

    if !errors.is_empty() {
        return json!({ "valid": false, "errors": errors });
    }

    // Format PRD as markdown
    let goals_arr = goals.unwrap();
    let non_goals_arr = non_goals.unwrap();
    let user_stories_arr = user_stories.unwrap();
    let milestones_arr = milestones.unwrap();

    let mut md = format!("# PRD: {}\n\n## Status: APPROVED\n\n", title);
    md.push_str(&format!(
        "## Problem Statement\n\n{}\n\n",
        problem_statement
    ));

    md.push_str("## Goals\n\n");
    for g in goals_arr {
        md.push_str(&format!("- {}\n", g.as_str().unwrap_or("")));
    }

    md.push_str("\n## Non-Goals\n\n");
    for ng in non_goals_arr {
        md.push_str(&format!("- {}\n", ng.as_str().unwrap_or("")));
    }

    md.push_str("\n## User Stories\n\n");
    for us in user_stories_arr {
        md.push_str(&format!("- {}\n", us.as_str().unwrap_or("")));
    }

    md.push_str(&format!(
        "\n## Technical Approach\n\n{}\n\n",
        technical_approach
    ));

    md.push_str("## Milestones\n\n");
    for m in milestones_arr {
        md.push_str(&format!("### {}\n\n", m["name"].as_str().unwrap_or("")));
        if let Some(deliverables) = m["deliverables"].as_array() {
            for d in deliverables {
                md.push_str(&format!("- {}\n", d.as_str().unwrap_or("")));
            }
        }
        md.push('\n');
    }

    md.push_str(&format!("## Complexity: {}\n\n", complexity));

    if let Some(metrics) = input["success_metrics"].as_array() {
        if !metrics.is_empty() {
            md.push_str("## Success Metrics\n\n");
            for m in metrics {
                md.push_str(&format!("- {}\n", m.as_str().unwrap_or("")));
            }
            md.push('\n');
        }
    }

    if let Some(risks) = input["risks"].as_array() {
        if !risks.is_empty() {
            md.push_str("## Risks\n\n");
            for r in risks {
                md.push_str(&format!("- {}\n", r.as_str().unwrap_or("")));
            }
            md.push('\n');
        }
    }

    // Store as document
    let doc_repo = Arc::clone(&state.repos().documents);

    let ref_tag = title_to_ref_tag(title);

    match doc_repo
        .create_document(CreateDocumentInput {
            user_id: user_id.0,
            session_id: None,
            title: title.to_string(),
            content: md.clone(),
            doc_type: "prd".to_string(),
            ref_tag: ref_tag.clone(),
            tags: vec!["prd".to_string()],
        })
        .await
    {
        Ok(row) => {
            spawn_summary_task(Arc::clone(&doc_repo), row.id, md);
            json!({
                "valid": true,
                "doc_id": row.id.to_string(),
                "ref_tag": ref_tag
            })
        }
        Err(e) => json!({ "error": e.to_string() }),
    }
}

pub(crate) async fn execute_submit_ticket(input: &Value) -> Value {
    let mut errors = Vec::new();

    let title = input["title"].as_str().unwrap_or("");
    if title.is_empty() {
        errors.push("Missing field: title".to_string());
    }
    let description = input["description"].as_str().unwrap_or("");
    if description.is_empty() {
        errors.push("Missing field: description".to_string());
    }

    let acceptance_criteria = input["acceptance_criteria"].as_array();
    if acceptance_criteria.is_none_or(|a| a.is_empty()) {
        errors.push("acceptance_criteria must have at least 1 entry".to_string());
    }
    let files_to_modify = input["files_to_modify"].as_array();
    if files_to_modify.is_none_or(|a| a.is_empty()) {
        errors.push("files_to_modify must have at least 1 entry".to_string());
    }

    let complexity = input["complexity"].as_str().unwrap_or("");
    if !matches!(complexity, "S" | "M" | "L" | "XL") {
        errors.push("complexity must be one of: S, M, L, XL".to_string());
    }

    let role = input["role"].as_str().unwrap_or("");
    if !matches!(role, "worker" | "reviewer" | "utility") {
        errors.push("role must be one of: worker, reviewer, utility".to_string());
    }

    if !errors.is_empty() {
        return json!({ "valid": false, "errors": errors });
    }

    let dependencies: Vec<String> = input["dependencies"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    json!({
        "valid": true,
        "ticket": {
            "title": title,
            "description": description,
            "acceptance_criteria": acceptance_criteria.unwrap(),
            "files_to_modify": files_to_modify.unwrap(),
            "complexity": complexity,
            "role": role,
            "dependencies": dependencies
        }
    })
}

//! Workflow, step, edge, document attachment, and execution endpoints

pub mod types;

pub mod document_handlers;
pub mod edge_handlers;
pub mod execution_handlers;
pub mod last_run_handlers;
pub mod run_detail_handlers;
pub mod run_handlers;
pub mod step_chat_handlers;
pub mod step_handlers;
pub mod sub_dag_handlers;
pub mod template_handlers;
pub mod workflow_handlers;
pub mod workshop_handlers;

pub use document_handlers::{add_step_document, list_step_documents, remove_step_document};
pub use edge_handlers::{
    add_workflow_edge, delete_workflow_edge_by_id, list_workflow_edges, remove_workflow_edge,
};
pub use execution_handlers::list_workflow_executions;
pub use last_run_handlers::get_step_last_run;
pub use run_detail_handlers::{get_run_detail, get_step_run_for_execution};
pub use run_handlers::run_workflow;
pub use step_chat_handlers::{
    clear_step_messages, get_or_create_step_session, get_step_chat_debug, get_step_session,
};
pub use step_handlers::{
    create_workflow_step, delete_workflow_step, get_step_config, get_workflow_notes,
    get_workflow_step, list_workflow_steps, toggle_step_pin, update_workflow_step,
};
pub use sub_dag_handlers::get_step_sub_dag;
pub use template_handlers::{
    create_template, delete_template, get_template, list_templates, rebase_workshop,
};
pub use types::{
    CreateStepRequest, CreateTemplateRequest, CreateWorkflowRequest, EdgeRequest, EdgeResponse,
    RebaseResponse, RunDetailResponse, RunStepResultResponse, RunTemplateDetailResponse,
    RunTemplateResponse, RunWorkflowRequest, StepDocumentRequest, StepDocumentResponse,
    UpdateStepRequest, UpdateWorkflowRequest, WorkflowExecutionResponse, WorkflowResponse,
    WorkflowRunResponse, WorkflowStepResponse, WorkshopResponse, WorkshopStatusResponse,
    WorkshopStepResponse,
};
pub use workflow_handlers::{
    create_workflow, delete_workflow, get_workflow, list_workflows, update_workflow,
};
pub use workshop_handlers::{execute_workshop_step, get_or_create_workshop, get_workshop};

#[cfg(test)]
mod tests;

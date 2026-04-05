use serde::Serialize;

use crate::models::InstructionAccountView;

#[derive(Serialize)]
pub(crate) struct RootView {
    pub(crate) program_id: String,
    pub(crate) root_pda: String,
    pub(crate) bump: u8,
    pub(crate) last_id: u16,
    pub(crate) threshold: u8,
    pub(crate) destination_program: String,
    pub(crate) operators: Vec<String>,
}

#[derive(Serialize)]
pub(crate) struct SessionView {
    pub(crate) program_id: String,
    pub(crate) root_pda: String,
    pub(crate) root_last_id: u16,
    pub(crate) destination_program: String,
    pub(crate) threshold: u8,
    pub(crate) session_pda: String,
    pub(crate) session_id: u16,
    pub(crate) bump: u8,
    pub(crate) phase: String,
    pub(crate) is_latest: bool,
    pub(crate) approvals: Vec<String>,
    pub(crate) approvals_count: usize,
    pub(crate) instruction_data_hex: String,
    pub(crate) instruction_data_base64: String,
    pub(crate) instruction_data_utf8: Option<String>,
    pub(crate) instruction_accounts: Vec<InstructionAccountView>,
    pub(crate) execute_root_writable: bool,
    pub(crate) execute_remaining_accounts: Vec<InstructionAccountView>,
    pub(crate) execute_required_outer_signers: Vec<String>,
}

#[derive(Serialize)]
pub(crate) struct SessionSummaryView {
    pub(crate) session_pda: String,
    pub(crate) session_id: u16,
    pub(crate) phase: String,
    pub(crate) approvals: Vec<String>,
    pub(crate) approvals_count: usize,
    pub(crate) bump: u8,
}

#[derive(Serialize)]
pub(crate) struct ListSessionsView {
    pub(crate) program_id: String,
    pub(crate) root_pda: String,
    pub(crate) root_last_id: u16,
    pub(crate) sessions: Vec<SessionSummaryView>,
}

#[derive(Serialize)]
pub(crate) struct InitRootResultView {
    pub(crate) signature: String,
    pub(crate) program_id: String,
    pub(crate) root_pda: String,
    pub(crate) bump: u8,
    pub(crate) threshold: u8,
    pub(crate) destination_program: String,
    pub(crate) operators: Vec<String>,
}

#[derive(Serialize)]
pub(crate) struct InitSessionResultView {
    pub(crate) signature: String,
    pub(crate) program_id: String,
    pub(crate) root_pda: String,
    pub(crate) session_pda: String,
    pub(crate) session_id: u16,
    pub(crate) bump: u8,
}

#[derive(Serialize)]
pub(crate) struct SessionActionResultView {
    pub(crate) action: String,
    pub(crate) signature: String,
    pub(crate) program_id: String,
    pub(crate) root_pda: String,
    pub(crate) session_pda: String,
    pub(crate) session_id: u16,
}

#[derive(Serialize)]
pub(crate) struct ExecuteResultView {
    pub(crate) signature: String,
    pub(crate) program_id: String,
    pub(crate) root_pda: String,
    pub(crate) session_pda: String,
    pub(crate) session_id: u16,
    pub(crate) destination_program: String,
    pub(crate) remaining_accounts: Vec<InstructionAccountView>,
    pub(crate) additional_signers: Vec<String>,
}

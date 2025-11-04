use rustc_middle::mir::Local;
use rustc_span::Span;

#[derive(Debug)]
pub struct CpiCallsInfo {
    pub span: Span,
    pub local: Local,
}

#[derive(Debug)]
pub struct CpiContextsInfo {
    pub cpi_ctx_local: Local,
    pub program_id_local: Local,
}

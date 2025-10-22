use crate::Program;
use std::collections::{HashMap, HashSet};
use syn::{Expr, ExprCall, ExprMethodCall, ItemFn, Stmt};

/// Check for missing reload() calls after CPI in the program module.
pub fn check_program(program: &Program) -> Vec<String> {
    let mut checker = ReloadChecker::new(program);
    checker.analyze();
    checker.warnings
}

struct ReloadChecker<'a> {
    program: &'a Program,
    /// All helper functions in the module (non-instruction handlers)
    helper_functions: HashMap<String, &'a ItemFn>,
    /// Warnings to emit
    warnings: Vec<String>,
}

impl<'a> ReloadChecker<'a> {
    fn new(program: &'a Program) -> Self {
        // Collect helper functions from the module
        let helper_functions = if let Some((_, items)) = &program.program_mod.content {
            items
                .iter()
                .filter_map(|item| {
                    if let syn::Item::Fn(func) = item {
                        Some((func.sig.ident.to_string(), func))
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            HashMap::new()
        };

        Self {
            program,
            helper_functions,
            warnings: Vec::new(),
        }
    }

    fn analyze(&mut self) {
        // Analyze each instruction handler
        for ix in &self.program.ixs {
            self.check_instruction(&ix.raw_method);
        }
    }

    fn check_instruction(&mut self, func: &ItemFn) {
        let func_name = func.sig.ident.to_string();
        let mut ctx = FunctionContext::new(func_name.clone());

        // Analyze the function body with inlining
        self.analyze_block(&func.block, &mut ctx, 0);

        // Report issues
        for issue in &ctx.issues {
            self.warnings
                .push(format!("Function `{}`: {}", func_name, issue));
        }
    }

    fn analyze_block(&mut self, block: &syn::Block, ctx: &mut FunctionContext, depth: usize) {
        // Prevent infinite recursion
        if depth > 10 {
            return;
        }

        for stmt in &block.stmts {
            self.analyze_stmt(stmt, ctx, depth);
        }
    }

    fn analyze_stmt(&mut self, stmt: &Stmt, ctx: &mut FunctionContext, depth: usize) {
        match stmt {
            Stmt::Local(local) => {
                if let Some((_eq, init_expr)) = &local.init {
                    self.analyze_expr(init_expr, ctx, depth);
                }
            }
            Stmt::Expr(expr) => {
                self.analyze_expr(expr, ctx, depth);
            }
            Stmt::Semi(expr, _) => {
                self.analyze_expr(expr, ctx, depth);
            }
            Stmt::Item(_) => {}
        }
    }

    fn analyze_expr(&mut self, expr: &Expr, ctx: &mut FunctionContext, depth: usize) {
        match expr {
            // Function calls - might be CPI or helper function
            Expr::Call(call) => {
                self.handle_call(call, ctx, depth);
            }

            // Method calls - check for invoke() or reload()
            Expr::MethodCall(method_call) => {
                self.handle_method_call(method_call, ctx, depth);
            }

            // Field access - check if account is accessed unsafely
            Expr::Field(field) => {
                if let Some(account) = Self::extract_account_name(&field.base) {
                    ctx.check_account_access(&account);
                }
                self.analyze_expr(&field.base, ctx, depth);
            }

            // Control flow
            Expr::If(if_expr) => {
                self.analyze_expr(&if_expr.cond, ctx, depth);
                self.analyze_block(&if_expr.then_branch, ctx, depth);
                if let Some((_, else_branch)) = &if_expr.else_branch {
                    self.analyze_expr(else_branch, ctx, depth);
                }
            }

            Expr::Match(match_expr) => {
                self.analyze_expr(&match_expr.expr, ctx, depth);
                for arm in &match_expr.arms {
                    self.analyze_expr(&arm.body, ctx, depth);
                }
            }

            Expr::ForLoop(for_loop) => {
                self.analyze_expr(&for_loop.expr, ctx, depth);
                self.analyze_block(&for_loop.body, ctx, depth);
            }

            Expr::Loop(loop_expr) => {
                self.analyze_block(&loop_expr.body, ctx, depth);
            }

            Expr::While(while_expr) => {
                self.analyze_expr(&while_expr.cond, ctx, depth);
                self.analyze_block(&while_expr.body, ctx, depth);
            }

            Expr::Block(block) => {
                self.analyze_block(&block.block, ctx, depth);
            }

            // Binary/unary operations
            Expr::Binary(binary) => {
                self.analyze_expr(&binary.left, ctx, depth);
                self.analyze_expr(&binary.right, ctx, depth);
            }

            Expr::Unary(unary) => {
                self.analyze_expr(&unary.expr, ctx, depth);
            }

            Expr::Reference(reference) => {
                self.analyze_expr(&reference.expr, ctx, depth);
            }

            Expr::Paren(paren) => {
                self.analyze_expr(&paren.expr, ctx, depth);
            }

            Expr::Try(try_expr) => {
                self.analyze_expr(&try_expr.expr, ctx, depth);
            }

            Expr::Assign(assign) => {
                self.analyze_expr(&assign.left, ctx, depth);
                self.analyze_expr(&assign.right, ctx, depth);
            }

            _ => {}
        }
    }

    fn handle_call(&mut self, call: &ExprCall, ctx: &mut FunctionContext, depth: usize) {
        // Check if it's a CPI function
        if let Expr::Path(path) = &*call.func {
            let path_str = path
                .path
                .segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect::<Vec<_>>()
                .join("::");

            // Check for invoke/invoke_signed
            if path_str.ends_with("::invoke")
                || path_str.ends_with("::invoke_signed")
                || path_str.contains("program::invoke")
                || path_str.contains("program::invoke_signed")
            {
                // Extract accounts from arguments
                for arg in &call.args {
                    if let Some(account) = Self::extract_account_name(arg) {
                        ctx.mark_cpi_account(account);
                    }
                }
            }
            // Check if it's a helper function we can inline
            else if let Some(last_segment) = path.path.segments.last() {
                let func_name = last_segment.ident.to_string();

                // Try to inline helper function
                if let Some(helper_func) = self.helper_functions.get(&func_name) {
                    // Inline this function's body
                    self.analyze_block(&helper_func.block, ctx, depth + 1);
                }
            }
        }

        // Also analyze the arguments
        for arg in &call.args {
            self.analyze_expr(arg, ctx, depth);
        }
    }

    fn handle_method_call(
        &mut self,
        method_call: &ExprMethodCall,
        ctx: &mut FunctionContext,
        depth: usize,
    ) {
        let method_name = method_call.method.to_string();

        // Check for reload()
        if method_name == "reload" {
            if let Some(account) = Self::extract_account_name(&method_call.receiver) {
                ctx.mark_reloaded(account);
            }
        }
        // Check for invoke() or invoke_signed() (builder pattern)
        else if method_name == "invoke" || method_name == "invoke_signed" {
            // Mark accounts in the receiver chain
            if let Some(account) = Self::extract_account_name(&method_call.receiver) {
                ctx.mark_cpi_account(account);
            }
        }
        // Other method calls might be accessing data
        // Exclude methods that only access metadata
        else if method_name != "key"
            && method_name != "to_account_info"
            && method_name != "as_ref"
            && method_name != "clone"
        {
            if let Some(account) = Self::extract_account_name(&method_call.receiver) {
                ctx.check_account_access(&account);
            }
        }

        // Recurse into receiver and arguments
        self.analyze_expr(&method_call.receiver, ctx, depth);
        for arg in &method_call.args {
            self.analyze_expr(arg, ctx, depth);
        }
    }

    fn extract_account_name(expr: &Expr) -> Option<String> {
        match expr {
            Expr::Path(path) => {
                // Look for ctx.accounts.ACCOUNT_NAME pattern
                if path.path.segments.len() >= 3 {
                    let segs: Vec<_> = path
                        .path
                        .segments
                        .iter()
                        .map(|s| s.ident.to_string())
                        .collect();

                    if segs.len() >= 3 && segs[0] == "ctx" && segs[1] == "accounts" {
                        return Some(segs[2].clone());
                    }
                }
                None
            }
            Expr::Field(field) => {
                // Check if this is ctx.accounts.ACCOUNT_NAME pattern
                if let Some(account_name) = Self::extract_account_name(&field.base) {
                    return Some(account_name);
                }

                // Check if this field access completes the pattern
                if let syn::Member::Named(ident) = &field.member {
                    if ident.to_string() == "accounts" {
                        // Check if base is ctx
                        if let Expr::Path(path) = &*field.base {
                            if path.path.segments.len() == 1
                                && path.path.segments[0].ident.to_string() == "ctx"
                            {
                                return None; // Need one more level
                            }
                        }
                    } else {
                        // This might be the account name
                        if let Expr::Field(inner_field) = &*field.base {
                            if let syn::Member::Named(inner_ident) = &inner_field.member {
                                if inner_ident.to_string() == "accounts" {
                                    if let Expr::Path(path) = &*inner_field.base {
                                        if path.path.segments.len() == 1
                                            && path.path.segments[0].ident.to_string() == "ctx"
                                        {
                                            return Some(ident.to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                None
            }
            Expr::Reference(reference) => Self::extract_account_name(&reference.expr),
            Expr::Unary(unary) => Self::extract_account_name(&unary.expr),
            Expr::Paren(paren) => Self::extract_account_name(&paren.expr),
            Expr::Array(array) => {
                // Handle array arguments - extract accounts from each element
                for elem in &array.elems {
                    if let Some(account) = Self::extract_account_name(elem) {
                        return Some(account);
                    }
                }
                None
            }
            Expr::MethodCall(method_call) => {
                // Handle to_account_info() calls
                if method_call.method.to_string() == "to_account_info" {
                    return Self::extract_account_name(&method_call.receiver);
                }
                None
            }
            _ => None,
        }
    }
}

/// Context for tracking state within a function (including inlined calls)
struct FunctionContext {
    _function_name: String,
    /// Accounts that have been passed to CPI
    cpi_accounts: HashSet<String>,
    /// Accounts that have been reloaded after CPI
    reloaded_accounts: HashSet<String>,
    /// Issues found
    issues: Vec<String>,
}

impl FunctionContext {
    fn new(function_name: String) -> Self {
        Self {
            _function_name: function_name,
            cpi_accounts: HashSet::new(),
            reloaded_accounts: HashSet::new(),
            issues: Vec::new(),
        }
    }

    fn mark_cpi_account(&mut self, account: String) {
        // If this account was already reloaded after a previous CPI,
        // remove it from reloaded_accounts because the reload is no longer valid
        if self.cpi_accounts.contains(&account) && self.reloaded_accounts.contains(&account) {
            self.reloaded_accounts.remove(&account);
        }
        self.cpi_accounts.insert(account);
    }

    fn mark_reloaded(&mut self, account: String) {
        self.reloaded_accounts.insert(account);
    }

    fn check_account_access(&mut self, account: &str) {
        if self.cpi_accounts.contains(account) && !self.reloaded_accounts.contains(account) {
            let issue = format!(
                "Account `{}` is accessed after CPI without calling reload(). \
                This can lead to stale data or security vulnerabilities.",
                account
            );

            // Only add if not already reported
            if !self.issues.iter().any(|i| i.contains(account)) {
                self.issues.push(issue);
            }
        }
    }
}

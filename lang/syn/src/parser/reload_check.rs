use crate::Program;
use std::collections::{HashMap, HashSet};
use syn::{spanned::Spanned, Expr, ExprCall, ExprMethodCall, ItemFn, Stmt};

/// Represents a reload violation with span information for better diagnostics
pub struct ReloadViolation {
    pub function_name: String,
    pub account_name: String,
    pub span: proc_macro2::Span,
}

impl ReloadViolation {
    pub fn to_error(&self) -> syn::Error {
        syn::Error::new(
            self.span,
            format!(
                "Missing reload() after CPI\n\
                Account `{}` is accessed after a CPI without calling reload().\n\n\
                = help: Call `.reload()?` on the account after the CPI and before accessing its data",
                self.account_name,
            ),
        )
    }
}

/// Check for missing reload() calls after CPI in the program module.
pub fn check_program(program: &Program) -> Vec<ReloadViolation> {
    check_program_with_file_items(program, &[])
}

/// Check for missing reload() calls after CPI, including file-level items outside the #[program] module.
/// This is useful for CLI tools or IDE plugins that have access to the entire file.
pub fn check_program_with_file_items(
    program: &Program,
    file_items: &[syn::Item],
) -> Vec<ReloadViolation> {
    let mut checker = ReloadChecker::new(program, file_items);
    checker.analyze();
    checker.violations
}

struct ReloadChecker<'a> {
    program: &'a Program,
    /// All helper functions in the module (non-instruction handlers)
    helper_functions: HashMap<String, &'a ItemFn>,
    /// Impl block methods: (type_name, method_name) -> method
    impl_methods: HashMap<(String, String), &'a syn::ImplItemMethod>,
    /// Violations found
    violations: Vec<ReloadViolation>,
}

impl<'a> ReloadChecker<'a> {
    fn new(program: &'a Program, file_items: &'a [syn::Item]) -> Self {
        let mut helper_functions = HashMap::new();
        let mut impl_methods = HashMap::new();

        // Collect helper functions and impl methods from the program module
        if let Some((_, items)) = &program.program_mod.content {
            Self::collect_items_from_slice(items, &mut helper_functions, &mut impl_methods);
        }

        // Also collect from file-level items (outside the #[program] module)
        Self::collect_items_from_slice(file_items, &mut helper_functions, &mut impl_methods);

        Self {
            program,
            helper_functions,
            impl_methods,
            violations: Vec::new(),
        }
    }

    fn collect_items_from_slice(
        items: &'a [syn::Item],
        helper_functions: &mut HashMap<String, &'a ItemFn>,
        impl_methods: &mut HashMap<(String, String), &'a syn::ImplItemMethod>,
    ) {
        for item in items {
            match item {
                syn::Item::Fn(func) => {
                    helper_functions.insert(func.sig.ident.to_string(), func);
                }
                syn::Item::Impl(impl_block) => {
                    // Extract the type name from the impl block
                    if let Some(type_name) = Self::extract_impl_type_name(impl_block) {
                        for impl_item in &impl_block.items {
                            if let syn::ImplItem::Method(method) = impl_item {
                                let method_name = method.sig.ident.to_string();
                                impl_methods.insert((type_name.clone(), method_name), method);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn extract_impl_type_name(impl_block: &syn::ItemImpl) -> Option<String> {
        if let syn::Type::Path(type_path) = &*impl_block.self_ty {
            type_path
                .path
                .segments
                .last()
                .map(|seg| seg.ident.to_string())
        } else {
            None
        }
    }

    fn analyze(&mut self) {
        // Analyze each instruction handler
        for ix in &self.program.ixs {
            let context_type = ix.anchor_ident.to_string();
            self.check_instruction(&ix.raw_method, &context_type);
        }
    }

    fn check_instruction(&mut self, func: &ItemFn, context_type: &str) {
        let func_name = func.sig.ident.to_string();
        let mut ctx = FunctionContext::new(func_name.clone());
        ctx.context_type = Some(context_type.to_string());

        // Analyze the function body with inlining
        self.analyze_block(&func.block, &mut ctx, 0);

        // Collect violations
        self.violations.extend(ctx.violations);
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
                // Track variable assignments like: let user_account = ctx.accounts.user_account;
                if let Some((_eq, init_expr)) = &local.init {
                    // Check if this is a safe method call (key, to_account_info, etc.)
                    let is_safe_method_call =
                        if let Expr::MethodCall(method_call) = init_expr.as_ref() {
                            let method_name = method_call.method.to_string();
                            method_name == "key"
                                || method_name == "to_account_info"
                                || method_name == "as_ref"
                                || method_name == "clone"
                        } else {
                            false
                        };

                    if let Some(account_name) = Self::extract_account_name(init_expr) {
                        // Only check access if it's not a safe method call
                        if !is_safe_method_call {
                            ctx.check_account_access(&account_name, init_expr.span());
                        }

                        // Extract variable name from pattern
                        if let syn::Pat::Ident(pat_ident) = &local.pat {
                            let var_name = pat_ident.ident.to_string();
                            ctx.var_to_account.insert(var_name, account_name);
                        } else if let syn::Pat::Type(pat_type) = &local.pat {
                            if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                                let var_name = pat_ident.ident.to_string();
                                ctx.var_to_account.insert(var_name, account_name);
                            }
                        }
                    }
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

            // Field access - check if account data is accessed unsafely
            Expr::Field(field) => {
                // Check if the field base is an account and we're accessing its data
                // (e.g., ctx.accounts.user_account.balance)
                if let Some(account) = Self::extract_account_name_with_ctx(&field.base, ctx) {
                    ctx.check_account_access(&account, field.span());
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
                    if let Some(account) = Self::extract_account_name_with_ctx(arg, ctx) {
                        ctx.mark_cpi_account(account);
                    }
                }
            }
            // Check if it's a helper function we can inline
            else if let Some(last_segment) = path.path.segments.last() {
                let func_name = last_segment.ident.to_string();

                // Try to inline helper function
                if let Some(helper_func) = self.helper_functions.get(&func_name) {
                    // Build parameter mapping: param_name -> account_name
                    let mut param_mapping = HashMap::new();

                    // Get parameter names from function signature
                    let param_names: Vec<String> = helper_func
                        .sig
                        .inputs
                        .iter()
                        .filter_map(|arg| {
                            if let syn::FnArg::Typed(pat_type) = arg {
                                if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                                    return Some(pat_ident.ident.to_string());
                                }
                            }
                            None
                        })
                        .collect();

                    // Match arguments to parameters
                    for (i, arg) in call.args.iter().enumerate() {
                        if i < param_names.len() {
                            if let Some(account_name) = Self::extract_account_name(arg) {
                                param_mapping.insert(param_names[i].clone(), account_name);
                            }
                        }
                    }

                    // Save current mappings and merge with new one
                    let saved_param_mapping = ctx.param_to_account.clone();
                    let saved_var_mapping = ctx.var_to_account.clone();
                    ctx.param_to_account.extend(param_mapping);

                    // Inline this function's body
                    self.analyze_block(&helper_func.block, ctx, depth + 1);

                    // Restore previous mappings
                    ctx.param_to_account = saved_param_mapping;
                    ctx.var_to_account = saved_var_mapping;
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
            if let Some(account) = Self::extract_account_name_with_ctx(&method_call.receiver, ctx) {
                ctx.mark_reloaded(account);
            }
        }
        // Check for invoke() or invoke_signed() (builder pattern)
        else if method_name == "invoke" || method_name == "invoke_signed" {
            // Mark accounts in the receiver chain
            if let Some(account) = Self::extract_account_name_with_ctx(&method_call.receiver, ctx) {
                ctx.mark_cpi_account(account);
            }
        }
        // Try to inline impl block methods
        else if self
            .try_inline_impl_method(method_call, ctx, depth)
            .is_some()
        {
            // Method was inlined, nothing more to do
            return;
        }
        // Other method calls might be accessing data
        // Exclude methods that only access metadata
        else if method_name != "key"
            && method_name != "to_account_info"
            && method_name != "as_ref"
            && method_name != "clone"
        {
            if let Some(account) = Self::extract_account_name_with_ctx(&method_call.receiver, ctx) {
                ctx.check_account_access(&account, method_call.span());
            }
        }

        // Recurse into receiver and arguments
        self.analyze_expr(&method_call.receiver, ctx, depth);
        for arg in &method_call.args {
            self.analyze_expr(arg, ctx, depth);
        }
    }

    fn try_inline_impl_method(
        &mut self,
        method_call: &ExprMethodCall,
        ctx: &mut FunctionContext,
        depth: usize,
    ) -> Option<()> {
        // Prevent infinite recursion
        if depth > 10 {
            return None;
        }

        let method_name = method_call.method.to_string();

        // Try to determine the type of the receiver
        // For now, we'll check if it matches common patterns like ctx.accounts
        let type_name = Self::extract_receiver_type(&method_call.receiver, ctx)?;

        // Look up the impl method
        let impl_method = self
            .impl_methods
            .get(&(type_name.clone(), method_name.clone()))?;

        // Save current mappings
        let saved_param_mapping = ctx.param_to_account.clone();
        let saved_var_mapping = ctx.var_to_account.clone();

        // Map 'self' to the receiver's accounts
        // For example, if receiver is ctx.accounts, we need to map self.user_account -> user_account
        if let Some(receiver_base) = Self::extract_receiver_base(&method_call.receiver, ctx) {
            ctx.param_to_account
                .insert("self".to_string(), receiver_base);
        }

        // Map method parameters to arguments
        let param_names: Vec<String> = impl_method
            .sig
            .inputs
            .iter()
            .filter_map(|arg| {
                match arg {
                    syn::FnArg::Receiver(_) => None, // Skip 'self'
                    syn::FnArg::Typed(pat_type) => {
                        if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                            Some(pat_ident.ident.to_string())
                        } else {
                            None
                        }
                    }
                }
            })
            .collect();

        for (i, arg) in method_call.args.iter().enumerate() {
            if i < param_names.len() {
                if let Some(account_name) = Self::extract_account_name(arg) {
                    ctx.param_to_account
                        .insert(param_names[i].clone(), account_name);
                }
            }
        }

        // Inline the method body
        self.analyze_block(&impl_method.block, ctx, depth + 1);

        // Restore previous mappings
        ctx.param_to_account = saved_param_mapping;
        ctx.var_to_account = saved_var_mapping;

        Some(())
    }

    fn extract_receiver_type(receiver: &Expr, ctx: &FunctionContext) -> Option<String> {
        // Check if this is ctx.accounts (which would be the Accounts struct type)
        if let Expr::Field(field) = receiver {
            if let Expr::Path(path) = &*field.base {
                if path.path.segments.len() == 1 && path.path.segments[0].ident == "ctx" {
                    if let syn::Member::Named(ident) = &field.member {
                        if ident == "accounts" {
                            // Use the Context<T> type from the function context
                            return ctx.context_type.clone();
                        }
                    }
                }
            }
        }
        None
    }

    fn extract_receiver_base(receiver: &Expr, _ctx: &FunctionContext) -> Option<String> {
        // For ctx.accounts, return the base that self should map to
        if let Expr::Field(field) = receiver {
            if let Expr::Path(path) = &*field.base {
                if path.path.segments.len() == 1 && path.path.segments[0].ident == "ctx" {
                    if let syn::Member::Named(ident) = &field.member {
                        if ident == "accounts" {
                            return Some("ctx_accounts".to_string());
                        }
                    }
                }
            }
        }
        None
    }

    fn extract_account_name(expr: &Expr) -> Option<String> {
        Self::extract_account_name_impl(expr, &HashMap::new())
    }

    fn extract_account_name_with_ctx(expr: &Expr, ctx: &FunctionContext) -> Option<String> {
        // First try with param mapping
        if let Some(account) = Self::extract_account_name_impl(expr, &ctx.param_to_account) {
            return Some(account);
        }
        // Then try with variable mapping
        Self::extract_account_name_impl(expr, &ctx.var_to_account)
    }

    fn extract_account_name_impl(
        expr: &Expr,
        param_mapping: &HashMap<String, String>,
    ) -> Option<String> {
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

                // Check if this is a parameter name that maps to an account
                if path.path.segments.len() == 1 {
                    let name = path.path.segments[0].ident.to_string();
                    if let Some(account_name) = param_mapping.get(&name) {
                        return Some(account_name.clone());
                    }
                }
                None
            }
            Expr::Field(field) => {
                // Check if this is self.field_name pattern (for impl methods)
                if let Expr::Path(path) = &*field.base {
                    if path.path.segments.len() == 1 && path.path.segments[0].ident == "self" {
                        // self.field_name should map to the account name
                        if let syn::Member::Named(field_name) = &field.member {
                            return Some(field_name.to_string());
                        }
                    }
                }

                // Check if this is ctx.accounts.ACCOUNT_NAME pattern
                if let Some(account_name) =
                    Self::extract_account_name_impl(&field.base, param_mapping)
                {
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
            Expr::Reference(reference) => {
                Self::extract_account_name_impl(&reference.expr, param_mapping)
            }
            Expr::Unary(unary) => Self::extract_account_name_impl(&unary.expr, param_mapping),
            Expr::Paren(paren) => Self::extract_account_name_impl(&paren.expr, param_mapping),
            Expr::Array(array) => {
                // Handle array arguments - extract accounts from each element
                for elem in &array.elems {
                    if let Some(account) = Self::extract_account_name_impl(elem, param_mapping) {
                        return Some(account);
                    }
                }
                None
            }
            Expr::MethodCall(method_call) => {
                // Handle to_account_info() calls
                if method_call.method.to_string() == "to_account_info" {
                    return Self::extract_account_name_impl(&method_call.receiver, param_mapping);
                }
                None
            }
            _ => None,
        }
    }
}

/// Context for tracking state within a function (including inlined calls)
struct FunctionContext {
    function_name: String,
    /// The Context<T> type name (e.g., "Transfer", "Initialize")
    context_type: Option<String>,
    /// Accounts that have been passed to CPI
    cpi_accounts: HashSet<String>,
    /// Accounts that have been reloaded after CPI
    reloaded_accounts: HashSet<String>,
    /// Violations found with span information
    violations: Vec<ReloadViolation>,
    /// Maps parameter names to account names when inlining helper functions
    /// e.g., "user_account" (param) -> "user_account" (ctx.accounts.user_account)
    param_to_account: HashMap<String, String>,
    /// Maps local variable names to account names
    /// e.g., "my_var" -> "user_account" when: let my_var = ctx.accounts.user_account;
    var_to_account: HashMap<String, String>,
}

impl FunctionContext {
    fn new(function_name: String) -> Self {
        Self {
            function_name,
            context_type: None,
            cpi_accounts: HashSet::new(),
            reloaded_accounts: HashSet::new(),
            violations: Vec::new(),
            param_to_account: HashMap::new(),
            var_to_account: HashMap::new(),
        }
    }

    fn mark_cpi_account(&mut self, account: String) {
        // Any CPI invalidates previous reloads (whether before or after previous CPIs)
        // because the CPI may modify the account data
        self.reloaded_accounts.remove(&account);
        self.cpi_accounts.insert(account);
    }

    fn mark_reloaded(&mut self, account: String) {
        self.reloaded_accounts.insert(account);
    }

    fn check_account_access(&mut self, account: &str, span: proc_macro2::Span) {
        if self.cpi_accounts.contains(account) && !self.reloaded_accounts.contains(account) {
            // Report the first violation for this account after CPI
            // Once reported, mark as "reloaded" to avoid duplicate reports for the same CPI
            self.violations.push(ReloadViolation {
                function_name: self.function_name.clone(),
                account_name: account.to_string(),
                span,
            });
            // Mark as reloaded to prevent reporting the same issue multiple times
            // for the same CPI until another CPI happens
            self.reloaded_accounts.insert(account.to_string());
        }
    }
}

#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use std::collections::{HashMap, HashSet, VecDeque};

use clippy_utils::diagnostics::span_lint_and_note;
use clippy_utils::fn_has_unsatisfiable_preds;
use clippy_utils::ty::is_type_diagnostic_item;
use rustc_hir::{Body as HirBody, FnDecl, def_id::LocalDefId, intravisit::FnKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::mir::{
    BasicBlock, BasicBlocks, Body as MirBody, HasLocalDecls, Operand, TerminatorKind,
};
use rustc_middle::ty::{self as rustc_ty, Ty};
use rustc_span::source_map::Spanned;
use rustc_span::{Span, Symbol};

dylint_linting::impl_late_lint! {
    /// ### What it does
    /// Identifies access of an account without calling `reload()` after a CPI.
    ///
    /// ### Why is this bad?
    /// After a CPI, deserialized accounts do not have their data updated automatically.
    /// Accessing them without calling `reload` may lead to stale data being loaded.
    /// ```
    pub MISSING_ACCOUNT_RELOAD,
    Warn,
    "account accessed after a CPI without reloading",
    MissingAccountReload::default()
}

#[derive(Default)]
pub struct MissingAccountReload;

impl<'tcx> LateLintPass<'tcx> for MissingAccountReload {
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        kind: FnKind<'tcx>,
        _: &FnDecl<'tcx>,
        _: &HirBody<'tcx>,
        _: Span,
        def_id: LocalDefId,
    ) {
        match kind {
            FnKind::ItemFn(ident, ..) if ident.as_str() == "initialize" => (),
            _ => return,
        }
        // Building MIR for `fn`s with unsatisfiable preds results in ICE.
        if fn_has_unsatisfiable_preds(cx, def_id.to_def_id()) {
            return;
        }

        let account_reload_sym = Symbol::intern("AnchorAccountReload");
        let deref_method_sym = Symbol::intern("deref_method");
        let cpi_invoke_syms = [
            Symbol::intern("AnchorCpiInvoke"),
            Symbol::intern("AnchorCpiInvokeUnchecked"),
            Symbol::intern("AnchorCpiInvokeSigned"),
            Symbol::intern("AnchorCpiInvokeSignedUnchecked"),
        ];

        let mir = cx.tcx.optimized_mir(def_id.to_def_id());

        // We need to identify
        // A) CPI invocations
        // Then, for each account type T
        // B<T>) Account data accesses (i.e. a call to `Deref` on `Account<T>`)
        // C<T>) Account reloads (i.e. a call to `Account::<T>::reload`)
        // We need to identify all (B<T>) which are dominated by (A) and *not* dominated by a corresponding (C<T>)

        // BBs terminated by a CPI
        let mut cpi_calls: HashMap<BasicBlock, Span> = HashMap::new();
        // Map of account fields to BBs accessing them
        // FIXME: Use a proper Place. Currently we assume there is exactly one account of each kind
        let mut account_accesses: HashMap<Ty, HashMap<BasicBlock, Span>> = HashMap::new();
        // Map of account fields to BBs reloading them
        let mut account_reloads: HashMap<Ty, HashSet<BasicBlock>> = HashMap::new();

        for (bb, bbdata) in mir.basic_blocks.iter_enumerated() {
            // Locate blocks ending with a call
            if let TerminatorKind::Call {
                func: Operand::Constant(func),
                args,
                fn_span,
                ..
            } = &bbdata.terminator().kind
                && let rustc_ty::FnDef(fn_def_id, generics) = func.ty().kind()
            {
                // Check that it is a diag item
                if let Some(diag_item) = cx
                    .tcx
                    .diagnostic_items(fn_def_id.krate)
                    .id_to_name
                    .get(fn_def_id)
                {
                    // Check if it is Account::reload...
                    if *diag_item == account_reload_sym {
                        // Extract the receiver
                        if let Some(account) = args.get(0)
                        && let Operand::Move(account) = account.node
                        // Get the corresponding local variable (should be a temporary &mut account.field)
                        && let Some(local) = account.as_local()
                        // Get the field type being accessed (`Account<AccountType>`)
                        && let Some(ty) = mir.local_decls().get(local).map(|d| d.ty.peel_refs())
                        {
                            account_reloads
                                .entry(ty)
                                .or_insert_with(HashSet::new)
                                .insert(bb);
                        }
                    }
                    // Or a CPI invoke function
                    else if cpi_invoke_syms.contains(diag_item) {
                        cpi_calls.insert(bb, *fn_span);
                    } else if *diag_item == deref_method_sym
                        && let Some(deref_impl_ty_arg) = generics.first()
                        && let Some(ty) = deref_impl_ty_arg.as_type()
                    {
                        account_accesses
                            .entry(ty)
                            .or_insert_with(HashMap::new)
                            .insert(bb, *fn_span);
                    }
                } else if takes_cpi_context(cx, mir, args) {
                    cpi_calls.insert(bb, *fn_span);
                }
            }
        }

        let cpi_call_blocks: HashSet<_> = cpi_calls.keys().copied().collect();
        for (ty, accesses) in account_accesses.into_iter() {
            let access_blocks = accesses.keys().copied().collect();
            let reloads = account_reloads.remove(&ty).unwrap_or_default();
            for (access, cpi) in reachable_without_passing(
                &mir.basic_blocks,
                cpi_call_blocks.clone(),
                access_blocks,
                reloads,
            ) {
                span_lint_and_note(
                    cx,
                    MISSING_ACCOUNT_RELOAD,
                    accesses[&access],
                    "accessing an account after a CPI without calling `reload()`",
                    Some(cpi_calls[&cpi]),
                    "CPI is here",
                );
            }
        }
    }
}

fn takes_cpi_context(cx: &LateContext<'_>, mir: &MirBody<'_>, args: &[Spanned<Operand>]) -> bool {
    args.iter().any(|arg| {
        if let Operand::Copy(place) | Operand::Move(place) = &arg.node
            && let Some(local) = place.as_local()
            && let Some(decl) = mir.local_decls().get(local)
        {
            is_type_diagnostic_item(cx, decl.ty.peel_refs(), Symbol::intern("AnchorCpiContext"))
        } else {
            false
        }
    })
}

/// Finds blocks in `to` that are reachable from `from` nodes without passing through `without` nodes
/// Returns a list of `to` nodes with the `from` node they are reachable from
fn reachable_without_passing(
    graph: &BasicBlocks,
    from: HashSet<BasicBlock>,
    to: HashSet<BasicBlock>,
    without: HashSet<BasicBlock>,
) -> Vec<(BasicBlock, BasicBlock)> {
    let mut queue = VecDeque::new();
    // Map of nodes to the `from` block they are reachable from
    let mut origin = HashMap::new();
    let mut visited = HashSet::new();

    for &f in &from {
        origin.insert(f, f);
        visited.insert(f);
        queue.push_back(f);
    }

    while let Some(u) = queue.pop_front() {
        if without.contains(&u) {
            continue;
        }
        for succ in graph[u]
            .terminator
            .as_ref()
            .map(|t| t.successors().collect::<Vec<_>>())
            .unwrap_or_default()
        {
            if without.contains(&succ) || visited.contains(&succ) {
                continue;
            }
            origin.insert(succ, origin[&u]);
            visited.insert(succ);
            queue.push_back(succ);
        }
    }

    to.into_iter()
        .filter_map(|bb| origin.get(&bb).map(|o| (bb, *o)))
        .collect()
}

use clippy_utils::ty::is_type_diagnostic_item;
use rustc_lint::LateContext;
use rustc_middle::{
    mir::{
        BasicBlock, BasicBlocks, Body as MirBody, HasLocalDecls, Local, Operand, Place, Rvalue,
        StatementKind,
    },
    ty::{self as rustc_ty},
};
use rustc_span::{Symbol, source_map::Spanned};

use std::collections::{HashMap, HashSet, VecDeque};

use crate::models::CpiCallsInfo;

#[derive(Debug)]
pub enum AssignmentKind<'tcx> {
    Const,
    FromPlace(Place<'tcx>),
    RefTo(Place<'tcx>),
    Other,
}

#[derive(Debug)]
pub enum Origin {
    Constant,
    Parameter,
    Unknown,
}

/// If these function args are two `Pubkey` references, return the corresponding
/// [`Local`]s.
pub fn args_as_pubkey_locals(
    cx: &LateContext<'_>,
    mir: &MirBody<'_>,
    args: &[Spanned<Operand>],
) -> Option<(Local, Local)> {
    Option::zip(
        pubkey_operand_to_local(cx, mir, &args.get(0)?.node),
        pubkey_operand_to_local(cx, mir, &args.get(1)?.node),
    )
}

/// If this [`Operand`] refers to a [`Local`] that is a `Pubkey`, return it
pub fn pubkey_operand_to_local(
    cx: &LateContext<'_>,
    mir: &MirBody<'_>,
    op: &Operand<'_>,
) -> Option<Local> {
    match op {
        Operand::Copy(place) | Operand::Move(place) => place
            .as_local()
            .filter(|local| is_pubkey_type(cx, mir, &local)),
        Operand::Constant(_) => None,
    }
}

pub fn is_pubkey_type(cx: &LateContext<'_>, mir: &MirBody<'_>, local: &Local) -> bool {
    if let Some(decl) = mir.local_decls().get(*local)
    && let ty = decl.ty.peel_refs()
    && let rustc_ty::Adt(adt_def, _) = ty.kind()
    && let def_path = cx.tcx.def_path_str(adt_def.did())
    // TODO: Add better check for Pubkey type
    && def_path.contains("Pubkey")
    {
        return true;
    }
    return false;
}

pub fn get_local_from_operand<'tcx>(operand: Option<&Spanned<Operand<'tcx>>>) -> Option<Local> {
    operand.and_then(|op| match &op.node {
        Operand::Copy(place) | Operand::Move(place) => place.as_local(),
        Operand::Constant(_) => None,
    })
}

pub fn check_program_id_included_in_conditional_blocks(
    cpi_ctx_local: &Local,
    cmps: &[crate::Cmp],
    assignment_map: &HashMap<Local, Vec<Local>>,
) -> bool {
    let mut cpi_context_references: Vec<Local> = Vec::new();
    for (k, v) in assignment_map {
        if v.contains(cpi_ctx_local) || k == cpi_ctx_local {
            cpi_context_references.push(*k);
            v.iter().for_each(|l| cpi_context_references.push(*l));
        }
    }

    if cmps
        .iter()
        .any(|l| cpi_context_references.contains(&l.lhs) || cpi_context_references.contains(&l.rhs))
    {
        return true;
    }

    false
}

pub fn check_cpi_context_variables_are_same(
    from: &Local,
    to: &Local,
    visited: &mut HashSet<Local>,
    assignment_map: &HashMap<Local, Vec<Local>>,
) -> bool {
    if visited.contains(from) {
        return false;
    }
    visited.insert(*from);
    if to == from {
        return true;
    }
    if let Some(assignment_locals) = assignment_map.get(from) {
        for assignment_local in assignment_locals {
            if check_cpi_context_variables_are_same(assignment_local, to, visited, assignment_map) {
                return true;
            }
        }
        return false;
    }
    return false;
}

pub fn cpi_invocation_is_reachable_from_cpi_context(
    graph: &BasicBlocks,
    from: BasicBlock,
    to: &HashMap<BasicBlock, CpiCallsInfo>,
) -> Option<BasicBlock> {
    let mut queue = VecDeque::new();
    let mut visited = HashSet::new();

    visited.insert(from);
    queue.push_back(from);

    while let Some(u) = queue.pop_front() {
        for succ in graph[u]
            .terminator
            .as_ref()
            .map(|t| t.successors().collect::<Vec<_>>())
            .unwrap_or_default()
        {
            if visited.contains(&succ) {
                continue;
            }
            if to.contains_key(&succ) {
                return Some(succ);
            }
            visited.insert(succ);
            queue.push_back(succ);
        }
    }
    None
}

pub fn build_assign_and_reverse_assignment_map<'tcx>(
    mir: &MirBody<'tcx>,
) -> (
    HashMap<Local, AssignmentKind<'tcx>>,
    HashMap<Local, Vec<Local>>,
) {
    let mut assignment_map = HashMap::new();
    let mut reverse_assignment_map = HashMap::new();

    for (_bb, bbdata) in mir.basic_blocks.iter_enumerated() {
        for statement in &bbdata.statements {
            if let StatementKind::Assign(box (place, rvalue)) = &statement.kind {
                if let Some(dest_local) = place.as_local() {
                    // Build forward map (AssignmentKind)
                    let kind = match rvalue {
                        Rvalue::Use(Operand::Constant(_)) => AssignmentKind::Const,
                        Rvalue::Use(Operand::Copy(src) | Operand::Move(src)) => {
                            AssignmentKind::FromPlace(*src)
                        }
                        Rvalue::Ref(_, _, src_place) => AssignmentKind::RefTo(*src_place),
                        _ => AssignmentKind::Other,
                    };
                    assignment_map.insert(dest_local, kind);

                    // Helper for reverse map
                    let mut record_mapping = |src_place: &Place<'tcx>| {
                        let src_local = src_place.local;
                        reverse_assignment_map
                            .entry(src_local)
                            .or_insert_with(Vec::new)
                            .push(dest_local);
                    };

                    // Build reverse map based on RHS sources
                    match rvalue {
                        Rvalue::Use(Operand::Copy(src) | Operand::Move(src)) => {
                            record_mapping(&src);
                        }
                        Rvalue::Ref(_, _, src) => {
                            record_mapping(&src);
                        }
                        Rvalue::Cast(_, op, _) => {
                            if let Operand::Copy(src) | Operand::Move(src) = op {
                                record_mapping(&src);
                            }
                        }
                        Rvalue::Aggregate(_, operands) => {
                            for operand in operands {
                                if let Operand::Copy(src) | Operand::Move(src) = operand {
                                    record_mapping(&src);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    (assignment_map, reverse_assignment_map)
}

pub fn build_transitive_reverse_map(
    direct_map: &HashMap<Local, Vec<Local>>,
) -> HashMap<Local, Vec<Local>> {
    let mut transitive_map: HashMap<Local, Vec<Local>> = HashMap::new();

    for (&src, dests) in direct_map {
        let mut visited = HashSet::new();
        let mut queue: VecDeque<Local> = VecDeque::from(dests.clone());

        while let Some(next) = queue.pop_front() {
            if visited.insert(next) {
                // Insert in Vec form
                transitive_map.entry(src).or_default().push(next);

                if let Some(next_dests) = direct_map.get(&next) {
                    for &nd in next_dests {
                        queue.push_back(nd);
                    }
                }
            }
        }
    }

    for vec in transitive_map.values_mut() {
        vec.sort();
    }

    transitive_map
}

pub fn origin_of_operand<'tcx>(
    cx: &LateContext<'tcx>,
    mir: &MirBody<'tcx>,
    assignment_map: &HashMap<Local, AssignmentKind<'tcx>>,
    op: &Operand<'tcx>,
) -> Origin {
    match op {
        Operand::Constant(_) => Origin::Constant,
        Operand::Copy(place) | Operand::Move(place) => {
            if let Some(local) = place.as_local() {
                let mut visited = HashSet::new();
                resolve_local_origin(cx, mir, assignment_map, local, &mut visited)
            } else {
                Origin::Unknown
            }
        }
    }
}

pub fn resolve_local_origin<'tcx>(
    cx: &LateContext<'tcx>,
    mir: &MirBody<'tcx>,
    assignment_map: &HashMap<Local, AssignmentKind<'tcx>>,
    local: Local,
    visited: &mut HashSet<Local>,
) -> Origin {
    if visited.contains(&local) {
        return Origin::Unknown;
    }
    visited.insert(local);

    if local.index() < mir.arg_count {
        return Origin::Parameter;
    }
    match assignment_map.get(&local) {
        Some(AssignmentKind::Const) => Origin::Constant,
        Some(AssignmentKind::FromPlace(place)) | Some(AssignmentKind::RefTo(place)) => {
            // if place points to another local
            if let Some(src_local) = place.as_local() {
                let origin = resolve_local_origin(cx, mir, assignment_map, src_local, visited);
                if let Origin::Unknown = origin {
                    return Origin::Unknown;
                }
                if let Origin::Constant | Origin::Parameter = origin {
                    return origin;
                }
                return origin;
            }
            return Origin::Unknown;
        }
        Some(AssignmentKind::Other) | None => Origin::Unknown,
    }
}

pub fn takes_cpi_context(
    cx: &LateContext<'_>,
    mir: &MirBody<'_>,
    args: &[Spanned<Operand>],
) -> bool {
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

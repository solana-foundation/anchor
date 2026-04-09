use crate::config::Program;
use anyhow::{anyhow, Context, Result};
use object::{Object, ObjectSection, ObjectSymbol, SymbolKind};
use rustc_demangle::demangle;
#[allow(deprecated)]
use solana_compute_budget::compute_budget::ComputeBudget;
use solana_sbpf::{
    ebpf,
    elf::Executable,
    program::BuiltinProgram,
    static_analysis::Analysis,
    vm::{Config, ContextObject},
};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::Arc;

/// Contains the folded stack samples that drive SVG flamegraph rendering.
pub struct FlamegraphReport {
    pub program_name: String,
    pub total_cu: u64,
    pub stacks: BTreeMap<Vec<String>, u64>,
}

/// Analyzes a built program ELF and returns a static CU flamegraph summary.
pub fn analyze_program(program: &Program) -> Result<Option<FlamegraphReport>> {
    let elf_path = program.binary_path(false);
    let elf_bytes = fs::read(&elf_path)
        .with_context(|| format!("Failed to read ELF file {}", elf_path.display()))?;
    let executable = match load_executable(&elf_bytes) {
        Ok(executable) => executable,
        Err(err) => {
            eprintln!(
                "Warning: Failed to parse built program ELF for `{}` ({}): {err:#}. Skipping flamegraph generation for this program.",
                program.lib_name,
                elf_path.display()
            );
            return Ok(None);
        }
    };
    let debug_symbol_path = elf_path.with_extension("debug");
    let extra_symbols = load_symbol_map(&debug_symbol_path)
        .or_else(|_| load_symbol_map(&elf_path))
        .unwrap_or_default();
    let analysis = Analysis::from_executable(&executable)
        .map_err(|err| anyhow!("Failed to analyze the built SBPF executable: {err}"))?;
    let entrypoint_pc = executable.get_entrypoint_instruction_offset();
    let walker = InstructionWalker::new(
        program.lib_name.clone(),
        &executable,
        &analysis,
        extra_symbols,
    );

    walker.walk(entrypoint_pc).map(Some)
}

/// Loads an SBPF executable with symbol labels enabled for analysis.
fn load_executable(elf_bytes: &[u8]) -> Result<Executable<NoopContext>> {
    let loader = Arc::new(BuiltinProgram::new_loader(Config {
        enable_symbol_and_section_labels: true,
        ..Config::default()
    }));

    Executable::from_elf(elf_bytes, loader)
        .map_err(|err| anyhow!("Failed to parse built program ELF: {err}"))
}

/// Provides the minimal VM context needed to parse and inspect an executable.
#[derive(Default)]
struct NoopContext;

impl ContextObject for NoopContext {
    fn consume(&mut self, _amount: u64) {}

    fn get_remaining(&self) -> u64 {
        0
    }
}

/// Walks reachable SBPF instructions and folds them into flamegraph stacks.
struct InstructionWalker<'a> {
    program_name: String,
    executable: &'a Executable<NoopContext>,
    instructions: HashMap<usize, ebpf::Insn>,
    function_labels: BTreeMap<usize, String>,
    sbpf_version: solana_sbpf::program::SBPFVersion,
    cost_model: CostModel,
    visited_states: HashSet<WalkerState>,
    folded_stacks: BTreeMap<Vec<String>, u64>,
    total_cu: u64,
}

/// Identifies a unique DFS state by program counter and symbolic call stack.
#[derive(Clone, Eq, Hash, PartialEq)]
struct WalkerState {
    pc: usize,
    frames: Vec<usize>,
}

/// Encapsulates the default Solana compute costs used by the flamegraph walker.
#[allow(deprecated)]
struct CostModel {
    compute_budget: ComputeBudget,
}

impl<'a> InstructionWalker<'a> {
    /// Creates a new walker from disassembled instructions and known symbols.
    fn new(
        program_name: String,
        executable: &'a Executable<NoopContext>,
        analysis: &'a Analysis<'a>,
        extra_symbols: BTreeMap<usize, String>,
    ) -> Self {
        let instructions = analysis
            .instructions
            .iter()
            .cloned()
            .map(|insn| (insn.ptr, insn))
            .collect();
        let function_labels = analysis
            .functions
            .iter()
            .map(|(pc, (_key, name))| (*pc, normalize_symbol(name, *pc)))
            .chain(extra_symbols)
            .collect();

        Self {
            program_name,
            executable,
            instructions,
            function_labels,
            sbpf_version: executable.get_sbpf_version(),
            cost_model: CostModel::new(),
            visited_states: HashSet::new(),
            folded_stacks: BTreeMap::new(),
            total_cu: 0,
        }
    }

    /// Starts the reachability walk from the executable entrypoint.
    fn walk(mut self, entrypoint_pc: usize) -> Result<FlamegraphReport> {
        self.walk_pc(entrypoint_pc, vec![entrypoint_pc]);

        Ok(FlamegraphReport {
            program_name: self.program_name,
            total_cu: self.total_cu,
            stacks: self.folded_stacks,
        })
    }

    /// Recursively explores all control-flow reachable from a given PC.
    fn walk_pc(&mut self, pc: usize, frames: Vec<usize>) {
        let state = WalkerState {
            pc,
            frames: frames.clone(),
        };
        if !self.visited_states.insert(state) {
            return;
        }

        let Some(insn) = self.instructions.get(&pc).cloned() else {
            return;
        };

        let cost = self
            .cost_model
            .instruction_cost(&insn, self.resolve_syscall_name(&insn).as_deref());
        self.record_instruction(&frames, cost);

        match insn.opc {
            ebpf::CALL_IMM => {
                if let Some(target_pc) = self.resolve_call_target(&insn) {
                    if !frames.contains(&target_pc) {
                        let mut callee_frames = frames.clone();
                        callee_frames.push(target_pc);
                        self.walk_pc(target_pc, callee_frames);
                    }
                }
                self.walk_fallthrough(&insn, frames);
            }
            ebpf::CALL_REG => {
                self.walk_fallthrough(&insn, frames);
            }
            ebpf::EXIT if !self.sbpf_version.static_syscalls() => {}
            ebpf::RETURN if self.sbpf_version.static_syscalls() => {}
            ebpf::JA => {
                self.walk_pc(self.jump_target(&insn), frames);
            }
            ebpf::JEQ_IMM
            | ebpf::JGT_IMM
            | ebpf::JGE_IMM
            | ebpf::JLT_IMM
            | ebpf::JLE_IMM
            | ebpf::JSET_IMM
            | ebpf::JNE_IMM
            | ebpf::JSGT_IMM
            | ebpf::JSGE_IMM
            | ebpf::JSLT_IMM
            | ebpf::JSLE_IMM
            | ebpf::JEQ_REG
            | ebpf::JGT_REG
            | ebpf::JGE_REG
            | ebpf::JLT_REG
            | ebpf::JLE_REG
            | ebpf::JSET_REG
            | ebpf::JNE_REG
            | ebpf::JSGT_REG
            | ebpf::JSGE_REG
            | ebpf::JSLT_REG
            | ebpf::JSLE_REG => {
                self.walk_pc(self.jump_target(&insn), frames.clone());
                self.walk_fallthrough(&insn, frames);
            }
            _ => {
                self.walk_fallthrough(&insn, frames);
            }
        }
    }

    /// Continues walking to the next sequential instruction.
    fn walk_fallthrough(&mut self, insn: &ebpf::Insn, frames: Vec<usize>) {
        self.walk_pc(self.next_pc(insn), frames);
    }

    /// Resolves the symbolic name of a syscall or helper invoked by an instruction.
    fn resolve_syscall_name(&self, insn: &ebpf::Insn) -> Option<String> {
        match insn.opc {
            ebpf::CALL_IMM if !self.sbpf_version.static_syscalls() => {
                if self.resolve_call_target(insn).is_some() {
                    return None;
                }

                self.loader_symbol_name(insn.imm as u32)
            }
            ebpf::SYSCALL if self.sbpf_version.static_syscalls() => {
                self.loader_symbol_name(insn.imm as u32)
            }
            _ => None,
        }
    }

    /// Resolves an immediate call target to a local instruction address.
    fn resolve_call_target(&self, insn: &ebpf::Insn) -> Option<usize> {
        let key = self
            .sbpf_version
            .calculate_call_imm_target_pc(insn.ptr, insn.imm);
        self.executable
            .get_function_registry()
            .lookup_by_key(key)
            .map(|(_name, target_pc)| target_pc)
            .filter(|target_pc| self.instructions.contains_key(target_pc))
    }

    /// Resolves a loader-registered helper name for legacy or static syscalls.
    fn loader_symbol_name(&self, key: u32) -> Option<String> {
        self.executable
            .get_loader()
            .get_function_registry()
            .lookup_by_key(key)
            .and_then(|(name, _)| std::str::from_utf8(name).ok())
            .map(|name| normalize_symbol(name, key as usize))
    }

    /// Records compute units against the current symbolic stack.
    fn record_instruction(&mut self, frames: &[usize], cost: u64) {
        let stack = frames
            .iter()
            .map(|pc| self.frame_label(*pc))
            .collect::<Vec<_>>();

        *self.folded_stacks.entry(stack).or_default() += cost;
        self.total_cu += cost;
    }

    /// Maps a function entry PC to the display label used in the flamegraph.
    fn frame_label(&self, pc: usize) -> String {
        self.function_labels
            .get(&pc)
            .cloned()
            .unwrap_or_else(|| format!("function_{pc}"))
    }

    /// Computes the static branch target for a jump instruction.
    fn jump_target(&self, insn: &ebpf::Insn) -> usize {
        (insn.ptr as isize + insn.off as isize + 1) as usize
    }

    /// Computes the next sequential program counter after an instruction.
    fn next_pc(&self, insn: &ebpf::Insn) -> usize {
        insn.ptr + instruction_width(insn)
    }
}

#[allow(deprecated)]
impl CostModel {
    /// Creates a cost model using Solana's current default compute budget values.
    fn new() -> Self {
        Self {
            compute_budget: ComputeBudget::new_with_defaults(false, false),
        }
    }

    /// Returns the static CU charge for a reachable instruction.
    fn instruction_cost(&self, insn: &ebpf::Insn, syscall_name: Option<&str>) -> u64 {
        let base_cost = 1;
        let extra_cost = match insn.opc {
            ebpf::CALL_IMM | ebpf::SYSCALL => syscall_name
                .map(|name| self.syscall_cost(name))
                .unwrap_or_default(),
            _ => 0,
        };

        base_cost + extra_cost
    }

    /// Returns the default CU charge for a known syscall helper.
    fn syscall_cost(&self, name: &str) -> u64 {
        if is_invoke_syscall(name) {
            self.compute_budget.invoke_units
        } else if is_log_64_syscall(name) {
            self.compute_budget.log_64_units
        } else if is_log_pubkey_syscall(name) {
            self.compute_budget.log_pubkey_units
        } else if is_create_program_address_syscall(name) {
            self.compute_budget.create_program_address_units
        } else if is_sysvar_syscall(name) {
            self.compute_budget.sysvar_base_cost
        } else if is_mem_op_syscall(name) {
            self.compute_budget.mem_op_base_cost
        } else if is_get_remaining_compute_units_syscall(name) {
            self.compute_budget.get_remaining_compute_units_cost
        } else {
            self.compute_budget.syscall_base_cost
        }
    }
}

/// Returns the number of instruction slots consumed by a single SBPF opcode.
fn instruction_width(insn: &ebpf::Insn) -> usize {
    if insn.opc == ebpf::LD_DW_IMM {
        2
    } else {
        1
    }
}

/// Demangles and normalizes a raw symbol for flamegraph display.
fn normalize_symbol(name: &str, pc: usize) -> String {
    let trimmed = name.trim_matches(char::from(0));
    let normalized = if trimmed.is_empty() {
        format!("function_{pc}")
    } else {
        demangle(trimmed).to_string()
    };

    strip_rust_hash_suffix(&normalized)
        .replace(';', ":")
        .replace('\n', " ")
}

/// Removes the trailing rustc symbol hash suffix when present.
fn strip_rust_hash_suffix(symbol: &str) -> &str {
    let Some((prefix, suffix)) = symbol.rsplit_once("::h") else {
        return symbol;
    };

    if suffix.len() == 16 && suffix.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        prefix
    } else {
        symbol
    }
}

/// Returns whether a syscall performs a cross-program invocation.
fn is_invoke_syscall(name: &str) -> bool {
    name.starts_with("sol_invoke")
}

/// Returns whether a syscall uses the fixed `log_u64` pricing bucket.
fn is_log_64_syscall(name: &str) -> bool {
    matches!(name, "sol_log_64_" | "sol_log_compute_units_")
}

/// Returns whether a syscall logs a public key.
fn is_log_pubkey_syscall(name: &str) -> bool {
    name == "sol_log_pubkey"
}

/// Returns whether a syscall derives a program address.
fn is_create_program_address_syscall(name: &str) -> bool {
    matches!(
        name,
        "sol_create_program_address" | "sol_try_find_program_address"
    )
}

/// Returns whether a syscall loads a sysvar via the runtime.
fn is_sysvar_syscall(name: &str) -> bool {
    matches!(
        name,
        "sol_get_clock_sysvar"
            | "sol_get_epoch_rewards_sysvar"
            | "sol_get_epoch_schedule_sysvar"
            | "sol_get_fees_sysvar"
            | "sol_get_last_restart_slot"
            | "sol_get_rent_sysvar"
            | "sol_get_sysvar"
    )
}

/// Returns whether a syscall is a memory helper with a shared base cost.
fn is_mem_op_syscall(name: &str) -> bool {
    matches!(
        name,
        "sol_memcmp_" | "sol_memcpy_" | "sol_memmove_" | "sol_memset_"
    )
}

/// Returns whether a syscall reads the remaining compute units.
fn is_get_remaining_compute_units_syscall(name: &str) -> bool {
    name == "sol_get_remaining_compute_units"
}

/// Loads text symbols from an ELF file and maps them to SBPF PCs.
fn load_symbol_map(path: &Path) -> Result<BTreeMap<usize, String>> {
    let bytes =
        fs::read(path).with_context(|| format!("Failed to read symbol file {}", path.display()))?;
    let file = object::File::parse(&*bytes)
        .map_err(|err| anyhow!("Failed to parse symbol file {}: {err}", path.display()))?;
    let text_section = file
        .sections()
        .find(|section| section.name().ok() == Some(".text"))
        .or_else(|| {
            file.sections()
                .find(|section| section.kind() == object::SectionKind::Text)
        })
        .ok_or_else(|| anyhow!("Missing .text section in {}", path.display()))?;
    let text_address = text_section.address();
    let text_end = text_address.saturating_add(text_section.size());

    let mut symbols = BTreeMap::new();
    for symbol in file.symbols().chain(file.dynamic_symbols()) {
        if symbol.kind() != SymbolKind::Text || symbol.address() == 0 {
            continue;
        }
        let Ok(name) = symbol.name() else {
            continue;
        };
        if name.is_empty() {
            continue;
        }

        let address = symbol.address();
        if address < text_address || address >= text_end {
            continue;
        }
        let relative = address - text_address;
        if relative % ebpf::INSN_SIZE as u64 != 0 {
            continue;
        }

        let pc = (relative / ebpf::INSN_SIZE as u64) as usize;
        symbols
            .entry(pc)
            .or_insert_with(|| normalize_symbol(name, pc));
    }

    Ok(symbols)
}

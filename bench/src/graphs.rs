use {
    anchor_bench::history::{BenchmarkHistory, BenchmarkResult},
    anyhow::{Context, Result},
    plotters::prelude::*,
    std::{collections::BTreeSet, fs, path::Path},
};

/// Directory name used to store generated benchmark charts.
pub const GRAPHS_DIR: &str = "graphs";

/// Renders benchmark charts for each program and instruction into SVG files.
pub fn render_graphs(bench_dir: &Path, history: &BenchmarkHistory) -> Result<()> {
    let graphs_dir = bench_dir.join(GRAPHS_DIR);
    fs::create_dir_all(&graphs_dir)
        .with_context(|| format!("failed to create {}", graphs_dir.display()))?;

    let timeline = ordered_results(history);
    if timeline.is_empty() {
        return Ok(());
    }

    let program_names = timeline
        .iter()
        .flat_map(|result| result.programs.keys().cloned())
        .collect::<BTreeSet<_>>();

    for program_name in program_names {
        render_program_graphs(&graphs_dir, history, &timeline, &program_name)?;
    }

    Ok(())
}

/// Returns benchmark results ordered from the oldest commit to the newest.
fn ordered_results(history: &BenchmarkHistory) -> Vec<&BenchmarkResult> {
    history.results.iter().rev().collect()
}

/// Renders all charts for a single benchmarked program.
fn render_program_graphs(
    graphs_dir: &Path,
    history: &BenchmarkHistory,
    timeline: &[&BenchmarkResult],
    program_name: &str,
) -> Result<()> {
    let program_dir = graphs_dir.join(sanitize_filename(program_name));
    fs::create_dir_all(&program_dir)
        .with_context(|| format!("failed to create {}", program_dir.display()))?;

    let labels = timeline
        .iter()
        .map(|result| display_commit(&result.commit))
        .collect::<Vec<_>>();
    let baseline_names = history
        .baseline_programs
        .get(program_name)
        .cloned()
        .unwrap_or_default();

    let binary_points = timeline
        .iter()
        .enumerate()
        .filter_map(|(index, result)| {
            result
                .programs
                .get(program_name)
                .map(|program| (index as i32, program.binary_size_bytes))
        })
        .collect::<Vec<_>>();

    let binary_baselines = baseline_names
        .iter()
        .filter_map(|baseline_name| {
            history
                .baseline
                .get(baseline_name)
                .map(|program| (baseline_name.clone(), program.binary_size_bytes))
        })
        .collect::<Vec<_>>();

    draw_metric_chart(
        &program_dir.join("binary_size.svg"),
        &format!("{program_name}: binary size"),
        "bytes",
        &labels,
        &binary_points,
        &binary_baselines,
    )?;

    let instruction_names = timeline
        .iter()
        .filter_map(|result| result.programs.get(program_name))
        .flat_map(|program| program.compute_units.keys().cloned())
        .collect::<BTreeSet<_>>();

    for instruction_name in instruction_names {
        let points = timeline
            .iter()
            .enumerate()
            .filter_map(|(index, result)| {
                result
                    .programs
                    .get(program_name)
                    .and_then(|program| program.compute_units.get(&instruction_name))
                    .copied()
                    .map(|compute_units| (index as i32, compute_units))
            })
            .collect::<Vec<_>>();

        let baselines = baseline_names
            .iter()
            .filter_map(|baseline_name| {
                history
                    .baseline
                    .get(baseline_name)
                    .and_then(|program| program.compute_units.get(&instruction_name))
                    .copied()
                    .map(|compute_units| (baseline_name.clone(), compute_units))
            })
            .collect::<Vec<_>>();

        draw_metric_chart(
            &program_dir.join(format!(
                "compute_units_{}.svg",
                sanitize_filename(&instruction_name)
            )),
            &format!("{program_name}: compute units for {instruction_name}"),
            "compute units",
            &labels,
            &points,
            &baselines,
        )?;
    }

    Ok(())
}

/// Draws a single benchmark metric chart with historical data and horizontal baselines.
fn draw_metric_chart(
    output_path: &Path,
    title: &str,
    y_label: &str,
    labels: &[String],
    benchmark_points: &[(i32, u64)],
    baselines: &[(String, u64)],
) -> Result<()> {
    if benchmark_points.is_empty() {
        return Ok(());
    }

    let root = SVGBackend::new(output_path, (960, 540)).into_drawing_area();
    root.fill(&WHITE)
        .with_context(|| format!("failed to initialize {}", output_path.display()))?;

    let x_count = labels.len().max(2) as i32;
    let last_x = x_count.saturating_sub(1);
    let max_value = benchmark_points
        .iter()
        .map(|(_, value)| *value)
        .chain(baselines.iter().map(|(_, value)| *value))
        .max()
        .unwrap_or(0);
    let y_max = padded_upper_bound(max_value);

    let mut chart = ChartBuilder::on(&root)
        .caption(title, ("sans-serif", 28))
        .margin(20)
        .x_label_area_size(48)
        .y_label_area_size(64)
        .build_cartesian_2d(0..x_count, 0u64..y_max)
        .with_context(|| format!("failed to build chart for {}", output_path.display()))?;

    chart
        .configure_mesh()
        .x_desc("commit")
        .y_desc(y_label)
        .x_labels(labels.len().max(1))
        .x_label_formatter(&|value| {
            labels
                .get((*value).clamp(0, last_x) as usize)
                .cloned()
                .unwrap_or_default()
        })
        .draw()
        .with_context(|| format!("failed to draw mesh for {}", output_path.display()))?;

    chart
        .draw_series(LineSeries::new(benchmark_points.iter().copied(), &BLUE))
        .with_context(|| format!("failed to draw series for {}", output_path.display()))?
        .label("benchmark")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 24, y)], BLUE));

    chart
        .draw_series(
            benchmark_points
                .iter()
                .copied()
                .map(|point| Circle::new(point, 4, BLUE.filled())),
        )
        .with_context(|| format!("failed to draw points for {}", output_path.display()))?;

    for (index, (baseline_name, baseline_value)) in baselines.iter().enumerate() {
        let color = Palette99::pick(index);
        let horizontal = vec![(0, *baseline_value), (last_x, *baseline_value)];
        chart
            .draw_series(LineSeries::new(horizontal, &color))
            .with_context(|| format!("failed to draw baseline for {}", output_path.display()))?
            .label(baseline_name.as_str())
            .legend(move |(x, y)| PathElement::new(vec![(x, y), (x + 24, y)], &color));
    }

    chart
        .configure_series_labels()
        .background_style(WHITE.mix(0.85))
        .border_style(BLACK)
        .draw()
        .with_context(|| format!("failed to draw legend for {}", output_path.display()))?;

    root.present()
        .with_context(|| format!("failed to write {}", output_path.display()))
}

/// Returns a readable label for a commit on the chart x-axis.
fn display_commit(commit: &str) -> String {
    if commit == "current" {
        commit.to_owned()
    } else {
        commit.chars().take(7).collect()
    }
}

/// Returns a filesystem-safe chart filename fragment.
fn sanitize_filename(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect()
}

/// Adds a small headroom buffer to the chart upper bound.
fn padded_upper_bound(max_value: u64) -> u64 {
    let padding = (max_value / 10).max(1);
    max_value.saturating_add(padding)
}

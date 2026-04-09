use crate::flamegraph::walker::FlamegraphReport;
use serde::Serialize;
use std::collections::BTreeMap;

const FRAME_HEIGHT: f64 = 22.0;
const FRAME_GAP: f64 = 1.0;
const SIDE_PADDING: f64 = 8.0;
const TOP_PADDING: f64 = 54.0;
const BOTTOM_PADDING: f64 = 12.0;
const SVG_WIDTH: f64 = 1600.0;
const RESET_BUTTON_WIDTH: f64 = 78.0;
const RESET_BUTTON_HEIGHT: f64 = 28.0;

/// Renders an interactive SVG flamegraph from folded stack samples.
pub fn render(report: &FlamegraphReport) -> String {
    let mut root = FlamegraphNode::new(report.program_name.clone());
    for (stack, count) in &report.stacks {
        root.insert(stack, *count);
    }

    let tree_json = serde_json::to_string(&root.to_render_node())
        .expect("Flamegraph tree should serialize")
        .replace("</", "<\\/");
    let max_depth = max_depth(&root.to_render_node());
    let svg_height =
        TOP_PADDING + BOTTOM_PADDING + ((max_depth as f64) + 1.0) * (FRAME_HEIGHT + FRAME_GAP);

    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="100%" height="100%" viewBox="0 0 {width} {height}">
  <style>
    .title {{ font: 700 24px monospace; fill: #1f1f1f; }}
    .subtitle {{ font: 14px monospace; fill: #4b4b4b; }}
    .button-label {{ font: 12px monospace; fill: #222; dominant-baseline: middle; text-anchor: middle; }}
    .button-rect {{ fill: #f3f3f3; stroke: #b8b8b8; stroke-width: 1; }}
  </style>
  <rect width="{width}" height="{height}" fill="#ffffff" />
  <text x="{side_padding}" y="30" class="title">{title}</text>
  <text id="subtitle" x="{side_padding}" y="50" class="subtitle">Total Reachable CU: {total_cu}</text>
  <g id="reset-button" transform="translate({reset_x}, 18)">
    <rect class="button-rect" width="{reset_button_width}" height="{reset_button_height}" rx="5" />
    <text x="{reset_button_text_x}" y="{reset_button_text_y}" class="button-label">Reset</text>
  </g>
  <g id="frames"></g>
  <script><![CDATA[
const tree = {tree_json};
const SVG_NS = "http://www.w3.org/2000/svg";
const FRAME_HEIGHT = {frame_height};
const FRAME_GAP = {frame_gap};
const SIDE_PADDING = {side_padding};
const TOP_PADDING = {top_padding};
const BOTTOM_PADDING = {bottom_padding};
const SVG_WIDTH = {width};
const SVG_HEIGHT = {height};
const INNER_WIDTH = SVG_WIDTH - SIDE_PADDING * 2;
const PROGRAM_NAME = {program_name_json};
const TOTAL_CU = {total_cu};

let focusPath = [];

function getNode(path) {{
  let node = tree;
  for (const index of path) {{
    node = node.children[index];
  }}
  return node;
}}

function maxDepth(node) {{
  if (!node.children.length) {{
    return 0;
  }}
  let depth = 0;
  for (const child of node.children) {{
    depth = Math.max(depth, 1 + maxDepth(child));
  }}
  return depth;
}}

function focusName() {{
  return focusPath.length ? getNode(focusPath).name : PROGRAM_NAME;
}}

function updateHeader() {{
  const subtitle = document.getElementById("subtitle");
  const resetButton = document.getElementById("reset-button");
  subtitle.textContent = focusPath.length
    ? `Focused: ${{focusName()}} | Total CU: ${{TOTAL_CU}}`
    : `Total CU: ${{TOTAL_CU}}`;
  resetButton.style.opacity = focusPath.length ? "1" : "0.55";
}}

function resetZoom(event) {{
  if (event) {{
    event.stopPropagation();
  }}
  focusPath = [];
  renderFrames();
}}

function zoomTo(path, event) {{
  if (event) {{
    event.stopPropagation();
  }}
  focusPath = path.slice();
  renderFrames();
}}

function truncateLabel(label, width) {{
  const maxChars = Math.floor(width / 7);
  if (maxChars < 4 || label.length <= maxChars) {{
    return label;
  }}
  return label.slice(0, maxChars - 3) + "...";
}}

function hashLabel(label) {{
  let hash = 2166136261 >>> 0;
  for (let index = 0; index < label.length; index += 1) {{
    hash ^= label.charCodeAt(index);
    hash = Math.imul(hash, 16777619) >>> 0;
  }}
  return hash >>> 0;
}}

function frameColor(label) {{
  const t = (hashLabel(label) % 10000) / 10000;
  const red = 255;
  const green = Math.round(235 + (52 - 235) * t);
  const blue = Math.round(130 + (0 - 130) * t);
  return {{ red, green, blue }};
}}

function colorToCss(color) {{
  return `rgb(${{color.red}}, ${{color.green}}, ${{color.blue}})`;
}}

function labelColor(color) {{
  const luminance = (0.299 * color.red + 0.587 * color.green + 0.114 * color.blue) / 255;
  return luminance < 0.52 ? "#fffdf8" : "#2b1600";
}}

function createSvgElement(tagName, attributes = {{}}) {{
  const element = document.createElementNS(SVG_NS, tagName);
  for (const [name, value] of Object.entries(attributes)) {{
    element.setAttribute(name, String(value));
  }}
  return element;
}}

function renderNode(parent, node, depth, x, width, subtreeDepth, path) {{
  if (!node.value || width <= 0.5) {{
    return;
  }}

  const y = TOP_PADDING + (subtreeDepth - depth) * (FRAME_HEIGHT + FRAME_GAP);
  const fillColor = frameColor(node.name);
  const label = `${{node.name}} (${{node.value}})`;
  const group = createSvgElement("g");
  const title = createSvgElement("title");
  title.textContent = label;
  group.appendChild(title);

  const rect = createSvgElement("rect", {{
    x,
    y,
    width,
    height: FRAME_HEIGHT,
    fill: colorToCss(fillColor),
    stroke: "#ffffff",
    "stroke-width": 1,
  }});
  group.appendChild(rect);

  if (path.length || node.children.length) {{
    group.style.cursor = "pointer";
    group.addEventListener("click", (event) => zoomTo(path, event));
  }}

  if (width > 48) {{
    const text = createSvgElement("text", {{
      x: x + 6,
      y: y + FRAME_HEIGHT / 2,
      "dominant-baseline": "middle",
      "font-family": "monospace",
      "font-size": 12,
      fill: labelColor(fillColor),
    }});
    text.textContent = truncateLabel(label, width);
    group.appendChild(text);
  }}

  parent.appendChild(group);

  let childX = x;
  for (let index = 0; index < node.children.length; index += 1) {{
    const child = node.children[index];
    const childWidth = width * (child.value / node.value);
    renderNode(parent, child, depth + 1, childX, childWidth, subtreeDepth, path.concat(index));
    childX += childWidth;
  }}
}}

function renderFrames() {{
  const frames = document.getElementById("frames");
  while (frames.firstChild) {{
    frames.removeChild(frames.firstChild);
  }}

  const focus = getNode(focusPath);
  const subtreeDepth = maxDepth(focus);
  renderNode(frames, focus, 0, SIDE_PADDING, INNER_WIDTH, subtreeDepth, focusPath.slice());
  updateHeader();
}}

document.getElementById("reset-button").addEventListener("click", resetZoom);
if (document.readyState === "loading") {{
  document.addEventListener("DOMContentLoaded", renderFrames, {{ once: true }});
}} else {{
  renderFrames();
}}
  ]]></script>
</svg>
"##,
        width = SVG_WIDTH,
        height = svg_height,
        side_padding = SIDE_PADDING,
        title = escape_xml(&format!("{} flamegraph", report.program_name)),
        total_cu = report.total_cu,
        tree_json = tree_json,
        frame_height = FRAME_HEIGHT,
        frame_gap = FRAME_GAP,
        top_padding = TOP_PADDING,
        bottom_padding = BOTTOM_PADDING,
        reset_x = SVG_WIDTH - SIDE_PADDING - RESET_BUTTON_WIDTH,
        reset_button_width = RESET_BUTTON_WIDTH,
        reset_button_height = RESET_BUTTON_HEIGHT,
        reset_button_text_x = RESET_BUTTON_WIDTH / 2.0,
        reset_button_text_y = RESET_BUTTON_HEIGHT / 2.0 + 1.0,
        program_name_json =
            serde_json::to_string(&report.program_name).expect("Program name should serialize"),
    )
}

/// Computes the maximum stack depth in the rendered flamegraph tree.
fn max_depth(node: &RenderNode) -> usize {
    node.children
        .iter()
        .map(|child| 1 + max_depth(child))
        .max()
        .unwrap_or(0)
}

/// Stores the intermediate tree used to aggregate folded stack samples.
#[derive(Default)]
struct FlamegraphNode {
    name: String,
    value: u64,
    children: BTreeMap<String, FlamegraphNode>,
}

impl FlamegraphNode {
    /// Creates an empty aggregate node with the given display name.
    fn new(name: String) -> Self {
        Self {
            name,
            value: 0,
            children: BTreeMap::new(),
        }
    }

    /// Inserts one folded stack and its sample count into the aggregate tree.
    fn insert(&mut self, stack: &[String], count: u64) {
        self.value += count;
        if let Some((head, tail)) = stack.split_first() {
            self.children
                .entry(head.clone())
                .or_insert_with(|| FlamegraphNode::new(head.clone()))
                .insert(tail, count);
        }
    }

    /// Converts the aggregate tree into a JSON-serializable render tree.
    fn to_render_node(&self) -> RenderNode {
        RenderNode {
            name: self.name.clone(),
            value: self.value,
            children: self
                .children
                .values()
                .map(FlamegraphNode::to_render_node)
                .collect(),
        }
    }
}

/// Represents a flamegraph node in the JSON payload embedded into the SVG.
#[derive(Serialize)]
struct RenderNode {
    name: String,
    value: u64,
    children: Vec<RenderNode>,
}

/// Escapes XML-sensitive characters before embedding text into the SVG.
fn escape_xml(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

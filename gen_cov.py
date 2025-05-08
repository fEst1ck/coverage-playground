# Updated generator that writes times.js and loads it from function.html
import os
import json
import argparse
import shutil
import urllib.parse
from pathlib import Path
from jinja2 import Template

INDEX_HTML_TEMPLATE = """
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Coverage Report</title>
    <link rel="stylesheet" href="css/style.css">
</head>
<body>
<h1>Coverage Summary</h1>

<label for="time-slider">Timestamp:</label>
<input type="range" min="0" max="{{ max_idx }}" value="0" id="time-slider">
<span id="time-label">{{ times[0] }}</span>

<table>
    <thead>
        <tr>
            <th>Function</th>
            <th># Blocks</th>
            <th># Edges</th>
            <th># Executions</th>
        </tr>
    </thead>
    <tbody id="table-body"></tbody>
</table>

<script>
const snapshots = {{ snapshots_json | safe }};
const times = {{ times | safe }};

function loadSnapshot(idx) {
    const tbody = document.getElementById('table-body');
    tbody.innerHTML = '';

    const snapshot = snapshots[idx];
    if (!snapshot || snapshot.length === 0) {
        const tr = document.createElement('tr');
        const td = document.createElement('td');
        td.colSpan = 4;
        td.textContent = "No functions in this snapshot";
        tr.appendChild(td);
        tbody.appendChild(tr);
        return;
    }

    for (const fn of snapshot) {
        if (!fn || !fn.name) continue;

        const tr = document.createElement('tr');

        const tdName = document.createElement('td');
        const a = document.createElement('a');
        a.href = `function.html?name=${encodeURIComponent(fn.name)}&t=${times[idx]}`;
        a.textContent = fn.name;
        tdName.appendChild(a);

        const tdBlocks = document.createElement('td');
        tdBlocks.textContent = fn.num_blocks;

        const tdEdges = document.createElement('td');
        tdEdges.textContent = fn.num_edges;

        const tdExecs = document.createElement('td');
        tdExecs.textContent = fn.execs;

        tr.appendChild(tdName);
        tr.appendChild(tdBlocks);
        tr.appendChild(tdEdges);
        tr.appendChild(tdExecs);
        tbody.appendChild(tr);
    }
}

document.getElementById('time-slider').addEventListener('input', function () {
    const idx = parseInt(this.value);
    document.getElementById('time-label').textContent = times[idx];
    loadSnapshot(idx);
});

window.addEventListener('DOMContentLoaded', () => {
    loadSnapshot(0);
});
</script>
</body>
</html>
"""

FUNCTION_HTML_TEMPLATE = """
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Function Coverage</title>
    <script src="https://unpkg.com/cytoscape/dist/cytoscape.min.js"></script>
    <script src="https://unpkg.com/dagre/dist/dagre.min.js"></script>
    <script src="https://unpkg.com/cytoscape-dagre/cytoscape-dagre.js"></script>
    <script src="times.js"></script>
    <link rel="stylesheet" href="css/style.css">
</head>
<body>
<h1 id="title">Function Coverage</h1>
<div>
  <label for="time-slider">Timestamp:</label>
  <input type="range" id="time-slider" />
  <span id="time-label"></span>
</div>
<div id="cy" style="width: 100%; height: 85vh;"></div>

<script>
function getParam(name) {
    const params = new URLSearchParams(window.location.search);
    return params.get(name);
}
const name = getParam("name");
let t = parseInt(getParam("t") || "0");

const slider = document.getElementById("time-slider");
const label = document.getElementById("time-label");
slider.min = 0;
slider.max = times.length - 1;
let currentIndex = times.indexOf(t);
if (currentIndex === -1) currentIndex = 0;
slider.value = currentIndex;
label.textContent = times[currentIndex];

function loadGraph(name, t) {
    const safeName = encodeURIComponent(name);
    fetch("graphs/" + t + "/" + safeName + ".json")
        .then(response => response.json())
        .then(data => {
            document.getElementById("title").innerText = name + " @ t=" + t;
            cytoscape({
                container: document.getElementById('cy'),
                elements: data,
                layout: {
                    name: 'dagre',
                    rankDir: 'TB',
                    nodeSep: 70,
                    edgeSep: 30,
                    rankSep: 100
                },
                style: [
                    {
                        selector: 'node',
                        style: {
                            'label': 'data(label)',
                            'background-color': '#0074D9',
                            'color': '#fff',
                            'text-valign': 'center',
                            'text-halign': 'center',
                            'text-wrap': 'wrap',
                            'text-max-width': 80,
                            'font-size': '10px',
                            'padding': '6px',
                            'shape': 'roundrectangle',
                            'width': 'label',
                            'height': 'label'
                        }
                    },
                    {
                        selector: 'edge',
                        style: {
                            'width': 2,
                            'line-color': '#ccc',
                            'target-arrow-shape': 'triangle',
                            'target-arrow-color': '#ccc',
                            'curve-style': 'bezier'
                        }
                    }
                ]
            });
        });
}

slider.addEventListener("input", () => {
    const idx = parseInt(slider.value);
    const newTime = times[idx];
    label.textContent = newTime;
    loadGraph(name, newTime);
});

window.addEventListener("DOMContentLoaded", () => {
    loadGraph(name, times[currentIndex]);
});
</script>
</body>
</html>
"""

STYLE_CSS = """
body {
    font-family: sans-serif;
    margin: 20px;
    background: #f4f4f4;
}
h1 {
    text-align: center;
}
table {
    border-collapse: collapse;
    width: 100%;
    background: white;
}
th, td {
    border: 1px solid #999;
    padding: 8px;
    text-align: left;
}
th {
    background: #ddd;
}
a {
    color: #0074D9;
    text-decoration: none;
}
"""

def safe_filename(name: str) -> str:
    return urllib.parse.quote(name, safe="")

def generate_time_series_report(input_dir: str, output_dir: str):
    input_path = Path(input_dir)
    output_path = Path(output_dir)
    graph_dir = output_path / "graphs"
    css_dir = output_path / "css"

    if output_path.exists():
        shutil.rmtree(output_path)
    output_path.mkdir()
    graph_dir.mkdir()
    css_dir.mkdir()

    snapshots = []
    times = []

    all_files = sorted(
        input_path.glob("fun_coverage_*.json"),
        key=lambda f: int(f.stem.split("_")[-1])
    )

    for file in all_files:
        time = int(file.stem.split("_")[-1])
        with open(file) as f:
            data = json.load(f)
        snapshot_summary = []
        time_dir = graph_dir / str(time)
        time_dir.mkdir(parents=True, exist_ok=True)
        for fn in data:
            name = fn["name"]
            block_exec_map = {bid: count for bid, count in fn["unique_blocks"]}
            covered_blocks = set(block_exec_map.keys())

            nodes = [{"data": {"id": str(bid), "label": f"Block {bid}\nExecs: {block_exec_map[bid]}"}} for bid in covered_blocks]
            edges = [{"data": {"source": str(src), "target": str(dst)}} for src, dst in fn["unique_edges"] if src in covered_blocks and dst in covered_blocks]

            safe_name = safe_filename(name)
            (time_dir / f"{safe_name}.json").write_text(json.dumps(nodes + edges, indent=2))

            snapshot_summary.append({
                "name": name,
                "num_blocks": len(block_exec_map),
                "num_edges": len(edges),
                "execs": fn["nums_executed"]
            })

        snapshots.append(snapshot_summary)
        times.append(time)

    (output_path / "index.html").write_text(Template(INDEX_HTML_TEMPLATE).render(
        snapshots_json=json.dumps(snapshots, ensure_ascii=False),
        times=times,
        max_idx=len(times) - 1
    ))
    (output_path / "function.html").write_text(FUNCTION_HTML_TEMPLATE)
    (css_dir / "style.css").write_text(STYLE_CSS)
    (output_path / "times.js").write_text("const times = " + json.dumps(times) + ";")
    print(f"✅ Time-series report generated at: {output_path.resolve()}")

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Generate time-series HTML coverage report")
    parser.add_argument("input_dir", help="Directory containing fun_coverage_*.json files")
    parser.add_argument("output", help="Directory to write the report to")
    args = parser.parse_args()
    generate_time_series_report(args.input_dir, args.output)

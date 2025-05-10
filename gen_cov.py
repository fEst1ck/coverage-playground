# Updated generator that writes times.js and loads it from function.html
import os
import json
import argparse
import shutil
import urllib.parse
from pathlib import Path
from jinja2 import Template
import hashlib
import itertools
import sys

INDEX_HTML_TEMPLATE1 = """
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Coverage Report</title>
    <link rel="stylesheet" href="css/style.css">
    <link rel="stylesheet" type="text/css" href="https://cdn.datatables.net/1.13.7/css/jquery.dataTables.css">
    <script type="text/javascript" src="https://code.jquery.com/jquery-3.7.0.min.js"></script>
    <script type="text/javascript" src="https://cdn.datatables.net/1.13.7/js/jquery.dataTables.js"></script>
</head>
<body>
<h1>Coverage Summary</h1>

<label for="time-slider">Timestamp:</label>
<input type="range" min="0" max="{{ max_idx }}" value="0" id="time-slider">
<span id="time-label">{{ times[0] }}</span>

<div style="overflow-x:auto;">
  <table id="coverage-table">
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
</div>

<script>
const snapshots = {{ snapshots_json | safe }};
const times = {{ times | safe }};
let dataTable;
let currentPage = 0;
let currentLength = 25;

function loadSnapshot(idx) {
    const snapshot = snapshots[idx];
    if (!snapshot || snapshot.length === 0) {
        if (dataTable) {
            dataTable.clear();
            dataTable.row.add(['No functions in this snapshot', '', '', '']).draw(false);
        }
        return;
    }

    // Clear existing data
    if (dataTable) {
        dataTable.clear();
    } else {
        // Initialize DataTable if it doesn't exist
        dataTable = $('#coverage-table').DataTable({
            order: [[0, 'asc']], // Sort by function name by default
            pageLength: 25,
            lengthMenu: [[10, 25, 50, -1], [10, 25, 50, "All"]],
            columnDefs: [
                { targets: 0, type: 'string' },
                { targets: [1, 2, 3], type: 'num' }
            ]
        });
    }

    // Add new data
    snapshot.forEach(fn => {
        if (!fn || !fn.name) return;
        const nameLink = `<a href="function.html?name=${encodeURIComponent(fn.name)}&t=${times[idx]}">${fn.name}</a>`;
        dataTable.row.add([
            nameLink,
            fn.num_blocks,
            fn.num_edges,
            fn.execs
        ]);
    });

    // Draw and preserve current page
    dataTable.draw(false);
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

FUNCTION_HTML_TEMPLATE1 = """
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
    const graphDir = "graphs/" + t + "/";
    const nameMapUrl = graphDir + "name_map.json";
    fetch(nameMapUrl)
        .then(resp => resp.json())
        .then(nameMap => {
            // Find the hash for this function name
            let hash = null;
            for (const [h, original] of Object.entries(nameMap)) {
                if (original === name) {
                    hash = h;
                    break;
                }
            }
            if (!hash) {
                alert("Function not found in mapping!");
                throw new Error("Function not found in mapping");
            }
            return fetch(graphDir + hash + ".json");
        })
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
                        selector: 'node[execs = 0]',
                        style: {
                            'background-color': '#cccccc', // light gray
                            'color': '#333',
                            'border-color': '#999'
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

/* Make the table horizontally scrollable if needed */
table {
    display: block;
    overflow-x: auto;
    white-space: nowrap;
    width: 100%;
    max-width: 100vw;
    background: white;
}

/* Prevent table from stretching too wide */
th, td {
    max-width: 220px;
    min-width: 80px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

/* Responsive: shrink font size for big tables */
@media (max-width: 900px) {
    th, td {
        font-size: 12px;
        min-width: 60px;
        max-width: 120px;
    }
}
"""

def safe_filename(name: str) -> str:
    return urllib.parse.quote(name, safe="")

def hash_name(name: str) -> str:
    return hashlib.sha1(name.encode('utf-8')).hexdigest()

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
        name_map = {}
        for fn in data:
            name = fn["name"]
            h = hash_name(name)
            block_exec_map = {bid: count for bid, count in fn["unique_blocks"]}
            covered_blocks = set(block_exec_map.keys())

            nodes = [{
                "data": {
                    "id": str(bid),
                    "label": f"Block {bid}\nExecs: {block_exec_map[bid]}",
                    "execs": int(block_exec_map[bid])
                }
            } for bid in covered_blocks]
            edges = [{"data": {"source": str(src), "target": str(dst)}} for src, dst in fn["unique_edges"] if src in covered_blocks and dst in covered_blocks]

            (time_dir / f"{h}.json").write_text(json.dumps(nodes + edges, indent=2))
            name_map[h] = name

            snapshot_summary.append({
                "name": name,
                "num_blocks": sum(1 for count in block_exec_map.values() if count > 0),
                "num_edges": len(edges),
                "execs": fn["nums_executed"]
            })
        # Write the mapping file for this snapshot
        (time_dir / "name_map.json").write_text(json.dumps(name_map, indent=2, ensure_ascii=False))

        snapshots.append(snapshot_summary)
        times.append(time)

    (output_path / "index.html").write_text(Template(INDEX_HTML_TEMPLATE1).render(
        snapshots_json=json.dumps(snapshots, ensure_ascii=False),
        times=times,
        max_idx=len(times) - 1
    ))
    (output_path / "function.html").write_text(FUNCTION_HTML_TEMPLATE1)
    (css_dir / "style.css").write_text(STYLE_CSS)
    (output_path / "times.js").write_text("const times = " + json.dumps(times) + ";")
    print(f"✅ Time-series report generated at: {output_path.resolve()}")

def generate_comparison_report(input_dirs: list[str], output_dir: str):
    input_paths = [Path(input_dir) for input_dir in input_dirs]
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

    files1 = sorted(
        input_paths[0].glob("fun_coverage_*.json"),
        key=lambda f: int(f.stem.split("_")[-1])
    )

    files2 = sorted(
        input_paths[1].glob("fun_coverage_*.json"),
        key=lambda f: int(f.stem.split("_")[-1])
    )

    for file1, file2 in zip(files1, files2):
        time1 = int(file1.stem.split("_")[-1])
        time2 = int(file2.stem.split("_")[-1])
        if time1 != time2:
            print(f"Error: Timestamp mismatch between files: {time1} != {time2}", file=sys.stderr)
        with open(file1) as f1:
            data1 = json.load(f1)
        with open(file2) as f2:
            data2 = json.load(f2)

        # convert data1 and data2 to dicts of name -> fn
        data1 = {fn["name"]: fn for fn in data1}
        data2 = {fn["name"]: fn for fn in data2}

        snapshot_summary = []
        time_dir = graph_dir / str(time1)
        time_dir.mkdir(parents=True, exist_ok=True)
        name_map = {}
        names = set(itertools.chain(data1.keys(), data2.keys()))
        for name in names:
            h = hash_name(name)
            fn1 = data1.get(name)
            fn2 = data2.get(name)
            if fn1 is None:
                blocks2 = {bid: count for bid, count in fn2["unique_blocks"]}
                blocks1 = {bid: 0 for bid, _count in fn2["unique_blocks"]}
            elif fn2 is None:
                blocks1 = {bid: count for bid, count in fn1["unique_blocks"]}
                blocks2 = {bid: 0 for bid, _count in fn1["unique_blocks"]}
            else:
                blocks1 = {bid: count for bid, count in fn1["unique_blocks"]}
                blocks2 = {bid: count for bid, count in fn2["unique_blocks"]}
            bids = set(blocks1.keys()) | set(blocks2.keys())
            block_exec_map = dict()
            for bid in bids:
                count1 = blocks1.get(bid, 0)
                count2 = blocks2.get(bid, 0)
                block_exec_map[bid] = (count1, count2)

            def node_color(bid):
                execs0, execs1 = block_exec_map[bid]
                if execs0 == 0 and execs1 == 0:
                    return "#808080"
                elif execs0 == 0:
                    return "#2ECC40"
                elif execs1 == 0:
                    return "#FF4136"
                else:
                    return "#0000FF"
            nodes = [{
                "data": {
                    "id": str(bid),
                    "label": f"Block {bid}\nExecs: {block_exec_map[bid][0]} / {block_exec_map[bid][1]}",
                    "execs0": int(block_exec_map[bid][0]),
                    "execs1": int(block_exec_map[bid][1]),
                    "color": node_color(bid)
                }
            } for bid in bids]
            edges1 = { (src, dst) for src, dst in fn1["unique_edges"] if src in bids and dst in bids } if fn1 else set()
            edges2 = { (src, dst) for src, dst in fn2["unique_edges"] if src in bids and dst in bids } if fn2 else set()
            edges_1_only = edges1 - edges2
            edges_2_only = edges2 - edges1
            edges_both = edges1 & edges2
            edges1_only = [{"data": {"source": str(src), "target": str(dst), "color": "#FF4136"}} for src, dst in edges_1_only]
            edges2_only = [{"data": {"source": str(src), "target": str(dst), "color": "#2ECC40"}} for src, dst in edges_2_only]
            edges_both = [
                {"data": {"source": str(src), "target": str(dst), "color": "#0074D9"}}
                for src, dst in edges_both
            ]

            (time_dir / f"{h}.json").write_text(json.dumps(nodes + edges1_only + edges2_only + edges_both, indent=2))
            name_map[h] = name

            snapshot_summary.append({
                "name": name,
                "num_blocks0": sum(1 for count in block_exec_map.values() if count[0] > 0),
                "num_blocks1": sum(1 for count in block_exec_map.values() if count[1] > 0),
                "num_edges0": len(edges1),
                "num_edges1": len(edges2),
                "execs0": fn1["nums_executed"] if fn1 else 0,
                "execs1": fn2["nums_executed"] if fn2 else 0
            })
        # Write the mapping file for this snapshot
        (time_dir / "name_map.json").write_text(json.dumps(name_map, indent=2, ensure_ascii=False))

        snapshots.append(snapshot_summary)
        times.append(time1)

    (output_path / "index.html").write_text(Template(INDEX_HTML_TEMPLATE2).render(
        snapshots_json=json.dumps(snapshots, ensure_ascii=False),
        times=times,
        max_idx=len(times) - 1
    ))
    (output_path / "function.html").write_text(FUNCTION_HTML_TEMPLATE2)
    (css_dir / "style.css").write_text(STYLE_CSS)
    (output_path / "times.js").write_text("const times = " + json.dumps(times) + ";")
    print(f"✅ Time-series report generated at: {output_path.resolve()}")

INDEX_HTML_TEMPLATE2 = """
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Coverage Report (Comparison)</title>
    <link rel="stylesheet" href="css/style.css">
</head>
<body>
<h1>Coverage Summary (Comparison)</h1>

<label for="time-slider">Timestamp:</label>
<input type="range" min="0" max="{{ max_idx }}" value="0" id="time-slider">
<span id="time-label">{{ times[0] }}</span>

<div style="overflow-x:auto;">
  <table id="coverage-table">
    <thead>
        <tr>
            <th>Function</th>
            <th># Blocks (F1)</th>
            <th># Blocks (F2)</th>
            <th># Edges (F1)</th>
            <th># Edges (F2)</th>
            <th># Executions (F1)</th>
            <th># Executions (F2)</th>
        </tr>
    </thead>
    <tbody id="table-body"></tbody>
  </table>
</div>

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
        td.colSpan = 7;
        td.textContent = "No functions in this snapshot";
        tr.appendChild(td);
        tbody.appendChild(tr);
        return;
    }

    for (const fn of snapshot) {
        if (!fn || !fn.name) continue;

        const tr = document.createElement('tr');
        const a = document.createElement('a');
        a.href = `function.html?name=${encodeURIComponent(fn.name)}&t=${times[idx]}`;
        a.textContent = fn.name;
        a.title = fn.name;
        const tdName = document.createElement('td');
        tdName.appendChild(a);

        function valOrDash(val) { return val === null || val === undefined ? '—' : val; }

        const tdBlocks1 = document.createElement('td');
        tdBlocks1.textContent = valOrDash(fn.num_blocks0);

        const tdBlocks2 = document.createElement('td');
        tdBlocks2.textContent = valOrDash(fn.num_blocks1);

        const tdEdges1 = document.createElement('td');
        tdEdges1.textContent = valOrDash(fn.num_edges0);

        const tdEdges2 = document.createElement('td');
        tdEdges2.textContent = valOrDash(fn.num_edges1);

        const tdExecs1 = document.createElement('td');
        tdExecs1.textContent = valOrDash(fn.execs0);

        const tdExecs2 = document.createElement('td');
        tdExecs2.textContent = valOrDash(fn.execs1);

        tr.appendChild(tdName);
        tr.appendChild(tdBlocks1);
        tr.appendChild(tdBlocks2);
        tr.appendChild(tdEdges1);
        tr.appendChild(tdEdges2);
        tr.appendChild(tdExecs1);
        tr.appendChild(tdExecs2);
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

FUNCTION_HTML_TEMPLATE2 = """
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Function Coverage (Comparison)</title>
    <script src="https://unpkg.com/cytoscape/dist/cytoscape.min.js"></script>
    <script src="https://unpkg.com/dagre/dist/dagre.min.js"></script>
    <script src="https://unpkg.com/cytoscape-dagre/cytoscape-dagre.js"></script>
    <script src="times.js"></script>
    <link rel="stylesheet" href="css/style.css">
</head>
<body>
<h1 id="title">Function Coverage (Comparison)</h1>
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
    const graphDir = "graphs/" + t + "/";
    const nameMapUrl = graphDir + "name_map.json";
    fetch(nameMapUrl)
        .then(resp => resp.json())
        .then(nameMap => {
            // Find the hash for this function name
            let hash = null;
            for (const [h, original] of Object.entries(nameMap)) {
                if (original === name) {
                    hash = h;
                    break;
                }
            }
            if (!hash) {
                alert("Function not found in mapping!");
                throw new Error("Function not found in mapping");
            }
            return fetch(graphDir + hash + ".json");
        })
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
                            'background-color': 'data(color)',
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
                            'line-color': 'data(color)',
                            'target-arrow-shape': 'triangle',
                            'target-arrow-color': 'data(color)',
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

CALL_GRAPH_HTML_TEMPLATE = """
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Call Graph Comparison</title>
    <script src="https://unpkg.com/cytoscape/dist/cytoscape.min.js"></script>
    <script src="https://unpkg.com/dagre/dist/dagre.min.js"></script>
    <script src="https://unpkg.com/cytoscape-dagre/cytoscape-dagre.js"></script>
    <script src="times.js"></script>
    <link rel="stylesheet" href="css/style.css">
</head>
<body>
<h1 id="title">Call Graph Comparison</h1>
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
let t = parseInt(getParam("t") || "0");

const slider = document.getElementById("time-slider");
const label = document.getElementById("time-label");
slider.min = 0;
slider.max = times.length - 1;
let currentIndex = times.indexOf(t);
if (currentIndex === -1) currentIndex = 0;
slider.value = currentIndex;
label.textContent = times[currentIndex];

function loadGraph(t) {
    const graphDir = "graphs/" + t + "/";
    fetch(graphDir + "call_graph.json")
        .then(response => response.json())
        .then(data => {
            document.getElementById("title").innerText = "Call Graph Comparison @ t=" + t;
            cytoscape({
                container: document.getElementById('cy'),
                elements: data,
                layout: {
                    name: 'dagre',
                    rankDir: 'LR',
                    nodeSep: 100,
                    edgeSep: 50,
                    rankSep: 150
                },
                style: [
                    {
                        selector: 'node',
                        style: {
                            'label': 'data(label)',
                            'background-color': 'data(color)',
                            'color': '#fff',
                            'text-valign': 'center',
                            'text-halign': 'center',
                            'text-wrap': 'wrap',
                            'text-max-width': 120,
                            'font-size': '12px',
                            'padding': '8px',
                            'shape': 'roundrectangle',
                            'width': 'label',
                            'height': 'label'
                        }
                    },
                    {
                        selector: 'edge',
                        style: {
                            'width': 2,
                            'line-color': 'data(color)',
                            'target-arrow-shape': 'triangle',
                            'target-arrow-color': 'data(color)',
                            'curve-style': 'bezier',
                            'label': 'data(label)',
                            'font-size': '10px',
                            'text-rotation': 'autorotate'
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
    loadGraph(newTime);
});

window.addEventListener("DOMContentLoaded", () => {
    loadGraph(times[currentIndex]);
});
</script>
</body>
</html>
"""

def generate_call_graph_report(input_dirs: list[str], output_dir: str):
    input_paths = [Path(input_dir) for input_dir in input_dirs]
    output_path = Path(output_dir)
    graph_dir = output_path / "graphs"
    css_dir = output_path / "css"

    if output_path.exists():
        shutil.rmtree(output_path)
    output_path.mkdir()
    graph_dir.mkdir()
    css_dir.mkdir()

    times = []

    files1 = sorted(
        input_paths[0].glob("fun_coverage_*.json"),
        key=lambda f: int(f.stem.split("_")[-1])
    )

    files2 = sorted(
        input_paths[1].glob("fun_coverage_*.json"),
        key=lambda f: int(f.stem.split("_")[-1])
    )

    for file1, file2 in zip(files1, files2):
        time1 = int(file1.stem.split("_")[-1])
        time2 = int(file2.stem.split("_")[-1])
        if time1 != time2:
            print(f"Error: Timestamp mismatch between files: {time1} != {time2}", file=sys.stderr)
        with open(file1) as f1:
            data1 = json.load(f1)
        with open(file2) as f2:
            data2 = json.load(f2)

        # Convert data1 and data2 to dicts of name -> fn
        data1 = {fn["name"]: fn for fn in data1}
        data2 = {fn["name"]: fn for fn in data2}

        time_dir = graph_dir / str(time1)
        time_dir.mkdir(parents=True, exist_ok=True)

        # Create nodes for all functions
        nodes = []
        edges = []
        all_function_names = set(itertools.chain(data1.keys(), data2.keys()))
        all_function_ids = set(fn["id"] for fn in data1) | set(fn["id"] for fn in data2)

        for name in all_function_names:
            fn1 = data1.get(name)
            fn2 = data2.get(name)
            
            # Determine node color based on execution counts
            if fn1 is None:
                execs1 = 0
                execs2 = fn2["nums_executed"]
                color = "#2ECC40"  # Green - only in fuzzer 2
                id = fn2["id"]
            elif fn2 is None:
                execs1 = fn1["nums_executed"]
                execs2 = 0
                color = "#FF4136"  # Red - only in fuzzer 1
                id = fn1["id"]
            else:
                execs1 = fn1["nums_executed"]
                execs2 = fn2["nums_executed"]
                id = fn1["id"]
                if fn1["id"] != fn2["id"]:
                    print(f"Error: Function ID mismatch between files: {fn1['id']} != {fn2['id']}", file=sys.stderr)
                if execs1 == 0 and execs2 == 0:
                    color = "#808080"  # Gray - not executed in either
                elif execs1 == 0:
                    color = "#2ECC40"  # Green - only executed in fuzzer 2
                elif execs2 == 0:
                    color = "#FF4136"  # Red - only executed in fuzzer 1
                else:
                    color = "#0074D9"  # Blue - executed in both

            nodes.append({
                "data": {
                    "id": id,
                    "label": f"{name}\\nExecs: {execs1} / {execs2}",
                    "color": color
                }
            })

            existing_edges = set()
            # Add edges for function calls
            if fn1:
                for callee in fn1["calls"]:
                    edges.append({
                        "data": {
                            "source": fn1["id"],
                            "target": callee,
                            "color": "#FF4136",  # Red for fuzzer 1
                            "label": "F1"
                        }
                    })
                    existing_edges.add((fn1["id"], callee))

            if fn2:
                for callee in fn2["calls"]:
                    if (fn2["id"], callee) in existing_edges:
                        edges.append({
                            "data": {
                                "source": fn2["id"],
                                "target": callee,
                                "color": "#0074D9",  # Blue for both
                                "label": "F2"
                            }
                        })
                    else:
                        edges.append({
                            "data": {
                                "source": fn2["id"],
                                "target": callee,
                                "color": "#2ECC40",  # Green for fuzzer 2
                                "label": "F2"
                            }
                        })

        # Write the call graph for this timestamp
        (time_dir / "call_graph.json").write_text(json.dumps(nodes + edges, indent=2))
        times.append(time1)

    (output_path / "call_graph.html").write_text(CALL_GRAPH_HTML_TEMPLATE)
    (css_dir / "style.css").write_text(STYLE_CSS)
    (output_path / "times.js").write_text("const times = " + json.dumps(times) + ";")
    print(f"✅ Call graph comparison report generated at: {output_path.resolve()}")

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Generate time-series HTML coverage report")
    parser.add_argument("input_dirs", nargs='+', help="One or two directories containing fun_coverage_*.json files")
    parser.add_argument("output", help="Directory to write the report to")
    parser.add_argument("--call-graph", action="store_true", help="Generate call graph comparison report")
    args = parser.parse_args()
    if len(args.input_dirs) == 1:
        generate_time_series_report(args.input_dirs[0], args.output)
    elif args.call_graph:
        generate_call_graph_report(args.input_dirs, args.output)
    else:
        generate_comparison_report(args.input_dirs, args.output)

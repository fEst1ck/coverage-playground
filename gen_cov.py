import os
import json
import argparse
import shutil
import urllib.parse
from pathlib import Path
from jinja2 import Template

# HTML templates (index, function view, style)
INDEX_HTML_TEMPLATE = """<!DOCTYPE html>
<html><head><meta charset="utf-8">
<title>Coverage Report</title>
<link rel="stylesheet" href="css/style.css"></head>
<body>
<h1>Coverage Summary</h1>
<table><thead><tr><th>Function</th><th># Blocks</th><th># Edges</th><th># Executions</th></tr></thead><tbody>
{% for f in functions %}
<tr>
<td><a href="function.html?name={{ f.name }}">{{ f.name }}</a></td>
<td>{{ f.num_blocks }}</td>
<td>{{ f.num_edges }}</td>
<td>{{ f.execs }}</td>
</tr>
{% endfor %}
</tbody></table></body></html>
"""

FUNCTION_HTML_TEMPLATE = """<!DOCTYPE html>
<html><head><meta charset="utf-8"><title>Function Coverage</title>
<script src="https://unpkg.com/cytoscape/dist/cytoscape.min.js"></script>
<script src="https://unpkg.com/dagre/dist/dagre.min.js"></script>
<script src="https://unpkg.com/cytoscape-dagre/cytoscape-dagre.js"></script>
<link rel="stylesheet" href="css/style.css"></head>
<body>
<h1 id="title">Function Coverage</h1>
<div id="cy" style="width: 100%; height: 90vh;"></div>
<script>
function getParam(name) {
    const params = new URLSearchParams(window.location.search);
    return params.get(name);
}
const name = getParam("name");
document.getElementById("title").innerText = name;
const safeName = encodeURIComponent(name);
fetch("graphs/" + safeName + ".json")
.then(response => response.json())
.then(data => {
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
</script>
</body></html>
"""

STYLE_CSS = """body {
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

def generate_html_report(snapshot_path: str, output_dir: str):
    output_path = Path(output_dir)
    graph_dir = output_path / "graphs"
    css_dir = output_path / "css"

    if output_path.exists():
        shutil.rmtree(output_path)
    output_path.mkdir()
    graph_dir.mkdir()
    css_dir.mkdir()

    with open(snapshot_path) as f:
        data = json.load(f)

    data.sort(key=lambda x: x["name"])

    summary = []
    for fn in data:
        name = fn["name"]
        block_exec_map = {bid: count for bid, count in fn["unique_blocks"]}
        covered_blocks = set(block_exec_map.keys())

        nodes = [
            {
                "data": {
                    "id": str(bid),
                    "label": f"Block {bid}\nExecs: {block_exec_map[bid]}"
                }
            }
            for bid in covered_blocks
        ]

        edges = [
            {
                "data": {
                    "source": str(src),
                    "target": str(dst)
                }
            }
            for src, dst in fn["unique_edges"]
            if src in covered_blocks and dst in covered_blocks
        ]

        safe_name = safe_filename(name)
        (graph_dir / f"{safe_name}.json").write_text(json.dumps(nodes + edges, indent=2))

        summary.append({
            "name": name,
            "num_blocks": len(block_exec_map),
            "num_edges": len(edges),
            "execs": fn["nums_executed"]
        })

    (output_path / "index.html").write_text(Template(INDEX_HTML_TEMPLATE).render(functions=summary))
    (output_path / "function.html").write_text(FUNCTION_HTML_TEMPLATE)
    (css_dir / "style.css").write_text(STYLE_CSS)

    print(f"✅ Report generated at: {output_path.resolve()}")

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Generate HTML coverage report from snapshot")
    parser.add_argument("snapshot", help="Path to JSON snapshot file")
    parser.add_argument("output", help="Output directory for report")
    args = parser.parse_args()

    generate_html_report(args.snapshot, args.output)

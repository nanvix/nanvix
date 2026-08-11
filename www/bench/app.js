// @ts-check
"use strict";

// --- Decoding ---

/** Decode the ?d= query param into structured PR data. */
function decodePayload() {
  const raw = new URLSearchParams(window.location.search).get("d");
  if (!raw) return null;
  try {
    // base64url -> standard base64
    let b64 = raw.replace(/-/g, "+").replace(/_/g, "/");
    // Ensure padding is present (defensive)
    const pad = (4 - (b64.length % 4)) % 4;
    b64 += "=".repeat(pad);
    const json = atob(b64);
    return JSON.parse(json);
  } catch (e) {
    console.error("Failed to decode payload:", e);
    return null;
  }
}

/** Decode an inner base64 CSV blob into rows. */
function decodeCsv(b64) {
  const text = atob(b64);
  const lines = text.trim().split("\n");
  const header = lines[0].split(",");
  const rows = lines.slice(1).map((l) => l.split(","));
  return { header, rows };
}

// --- Baseline fetching ---

/** Map bench key to baseline CSV path (relative to site root). */
function baselinePath(bench, runner, machine, arch) {
  const slug = bench.replace(/-/g, "_") + "_standalone";
  return `data/${runner}/nanvix_bench_${slug}_${machine}_${arch}.csv`;
}

async function fetchBaseline(bench, runner, machine, arch) {
  const path = baselinePath(bench, runner, machine, arch);
  try {
    const resp = await fetch(path);
    if (!resp.ok) return null;
    const text = await resp.text();
    const lines = text.trim().split("\n");
    const header = lines[0].split(",");
    const rows = lines.slice(1).map((l) => l.split(","));
    return { header, rows };
  } catch {
    return null;
  }
}

// --- Chart rendering ---

const SHORT_SHA = 7;
const MAX_BASELINE_COMMITS = 50;

let chartInstances = [];

function destroyCharts() {
  for (const c of chartInstances) c.destroy();
  chartInstances = [];
  document.getElementById("charts").innerHTML = "";
}

/** Create a chart container with optional title, append to #charts, return canvas. */
function createCanvas(title) {
  const container = document.createElement("div");
  container.className = "chart-container";
  if (title) {
    const h = document.createElement("h2");
    h.textContent = title;
    container.appendChild(h);
  }
  const canvas = document.createElement("canvas");
  container.appendChild(canvas);
  document.getElementById("charts").appendChild(container);
  return canvas;
}

/**
 * Render a standard benchmark (commit,p50,p95,p99).
 * Single stacked bar chart: history, with the PR commit highlighted as an
 * extra bar when one is given. Pass prRow=null to render pure history with
 * no highlighted bar.
 */
function renderStandard(baseline, prRow, prCommit) {
  destroyCharts();
  const canvas = createCanvas(null);

  const rows = baseline.rows.slice(-MAX_BASELINE_COMMITS);
  const commits = rows.map((r) => r[0].slice(0, SHORT_SHA));
  const labels = prRow ? [...commits, prCommit.slice(0, SHORT_SHA)] : commits;

  const p50Data = rows.map((r) => +r[1]);
  const p95Delta = rows.map((r) => +r[2] - +r[1]);
  const p99Delta = rows.map((r) => +r[3] - +r[2]);

  if (prRow) {
    p50Data.push(+prRow[1]);
    p95Delta.push(+prRow[2] - +prRow[1]);
    p99Delta.push(+prRow[3] - +prRow[2]);
  }

  const prIdx = prRow ? labels.length - 1 : -1;
  const bgColor = (base, highlight) => labels.map((_, i) => i === prIdx ? highlight : base);

  const chart = new Chart(canvas, {
    type: "bar",
    data: {
      labels,
      datasets: [
        { label: "p50", data: p50Data, backgroundColor: bgColor("#00adb5", "#00ffd5"), borderWidth: 0 },
        { label: "p95 \u2212 p50", data: p95Delta, backgroundColor: bgColor("#e2b93d", "#ffe066"), borderWidth: 0 },
        { label: "p99 \u2212 p95", data: p99Delta, backgroundColor: bgColor("#e94560", "#ff6b81"), borderWidth: 0 },
      ],
    },
    options: {
      responsive: true,
      plugins: {
        legend: { labels: { color: "#e0e0e0", font: { size: 11 } } },
        tooltip: {
          callbacks: {
            afterBody(items) {
              const idx = items[0].dataIndex;
              const total = p50Data[idx] + p95Delta[idx] + p99Delta[idx];
              return `p99 total: ${total.toLocaleString()} \u03bcs`;
            },
          },
        },
      },
      scales: {
        x: { stacked: true, ticks: { color: "#888", font: { size: 9 }, maxRotation: 60 }, grid: { color: "#0f3460" } },
        y: { stacked: true, ticks: { color: "#888" }, grid: { color: "#0f3460" }, title: { display: true, text: "Latency (\u03bcs)", color: "#888" } },
      },
    },
  });
  chartInstances.push(chart);
}

/**
 * Render a sized benchmark (commit,size,p50,p95,p99).
 * One chart per size bucket. Each chart shows baseline history, with the PR
 * commit's stacked bar highlighted when prRows is non-empty. Pass an empty
 * prRows array to render pure history for every size found in the baseline.
 */
function renderSized(baseline, prRows, prCommit) {
  destroyCharts();

  const hasPr = prRows.length > 0;
  const sizes = hasPr
    ? [...new Set(prRows.map((r) => r[1]))]
    : [...new Set(baseline.rows.map((r) => r[1]))];

  // Group baseline rows by size
  const baseBySize = {};
  for (const row of baseline.rows) {
    const size = row[1];
    if (!baseBySize[size]) baseBySize[size] = [];
    baseBySize[size].push(row);
  }

  // PR values by size
  const prBySize = {};
  for (const row of prRows) {
    prBySize[row[1]] = { p50: +row[2], p95: +row[3], p99: +row[4] };
  }

  for (const size of sizes) {
    const canvas = createCanvas(size);

    // Baseline history for this size (last N commits)
    const sizeRows = (baseBySize[size] || []).slice(-MAX_BASELINE_COMMITS);
    const commits = sizeRows.map((r) => r[0].slice(0, SHORT_SHA));
    const labels = hasPr ? [...commits, prCommit.slice(0, SHORT_SHA)] : commits;

    const p50Data = sizeRows.map((r) => +r[2]);
    const p95Delta = sizeRows.map((r) => +r[3] - +r[2]);
    const p99Delta = sizeRows.map((r) => +r[4] - +r[3]);

    if (hasPr) {
      const pr = prBySize[size];
      if (pr) {
        p50Data.push(pr.p50);
        p95Delta.push(pr.p95 - pr.p50);
        p99Delta.push(pr.p99 - pr.p95);
      } else {
        p50Data.push(null);
        p95Delta.push(null);
        p99Delta.push(null);
      }
    }

    const prIdx = hasPr ? labels.length - 1 : -1;
    const bgColor = (base, highlight) => labels.map((_, i) => i === prIdx ? highlight : base);

    const chart = new Chart(canvas, {
      type: "bar",
      data: {
        labels,
        datasets: [
          { label: "p50", data: p50Data, backgroundColor: bgColor("#00adb5", "#00ffd5"), borderWidth: 0 },
          { label: "p95 \u2212 p50", data: p95Delta, backgroundColor: bgColor("#e2b93d", "#ffe066"), borderWidth: 0 },
          { label: "p99 \u2212 p95", data: p99Delta, backgroundColor: bgColor("#e94560", "#ff6b81"), borderWidth: 0 },
        ],
      },
      options: {
        responsive: true,
        plugins: {
          legend: { labels: { color: "#e0e0e0", font: { size: 11 } } },
          tooltip: {
            callbacks: {
              afterBody(items) {
                const idx = items[0].dataIndex;
                const total = p50Data[idx] + p95Delta[idx] + p99Delta[idx];
                return `p99 total: ${total.toLocaleString()} \u03bcs`;
              },
            },
          },
        },
        scales: {
          x: { stacked: true, ticks: { color: "#888", font: { size: 9 }, maxRotation: 60 }, grid: { color: "#0f3460" } },
          y: { stacked: true, ticks: { color: "#888" }, grid: { color: "#0f3460" }, title: { display: true, text: "Latency (\u03bcs)", color: "#888" } },
        },
      },
    });
    chartInstances.push(chart);
  }
}

// --- Main ---

const SIZED_BENCHMARKS = new Set([
  "warm-start-gateway",
  "warm-start-vmm",
  "warm-start-socket",
]);

const VFS_BENCHMARKS = new Set(["vfs-bench"]);

// Standard benchmark suite, used to build a default history-only payload
// when the URL has no ?d= param (i.e. what CI runs, on
// linux-baremetal/microvm/X64).
const DEFAULT_BENCHMARKS = [
  "boot-time",
  "cold-start",
  "cold-start-uvm",
  "snapshot-restore",
  "vfs-bench",
  "warm-start-gateway",
  "warm-start-vmm",
  "warm-start-socket",
];

/** Detect the runner-label matching the platform this browser is running on
 * (mirrors benchmark.py's DEFAULT_RUNNER_LABEL). Used to pick a sensible
 * default history dir when the URL has no ?d= param. */
function detectPlatformRunnerLabel() {
  const platform =
    (navigator.userAgentData && navigator.userAgentData.platform) ||
    navigator.platform ||
    navigator.userAgent ||
    "";
  return /win/i.test(platform) ? "windows-baremetal" : "linux-baremetal";
}

/** Build a history-only payload (no PR/highlighted commit) for the default
 * runner/machine/arch, listing every standard benchmark. Used as a fallback
 * when the URL has no ?d= param. */
function defaultHistoryPayload() {
  const b = {};
  for (const bench of DEFAULT_BENCHMARKS) b[bench] = true;
  return { r: detectPlatformRunnerLabel(), m: "microvm", a: "X64", h: true, b };
}

/**
 * Render a VFS benchmark (commit,section,operation,samples,p50,p95,p99).
 * One chart per section. X-axis = operations, grouped stacked bars: dev vs PR.
 * Pass an empty prRows array to render only the latest baseline commit, with
 * no separate PR series.
 */
function renderVfs(baseline, prRows, prCommit) {
  destroyCharts();

  const hasPr = prRows.length > 0;

  // Group PR rows by section
  const prBySec = {};
  for (const row of prRows) {
    const sec = row[1];
    if (!prBySec[sec]) prBySec[sec] = [];
    prBySec[sec].push(row);
  }

  // Group baseline by section, take latest commit only
  const baseBySec = {};
  if (baseline.rows.length > 0) {
    const latestCommit = baseline.rows[baseline.rows.length - 1][0];
    for (const row of baseline.rows) {
      if (row[0] !== latestCommit) continue;
      const sec = row[1];
      if (!baseBySec[sec]) baseBySec[sec] = [];
      baseBySec[sec].push(row);
    }
  }

  const sections = hasPr
    ? [...new Set(prRows.map((r) => r[1]))]
    : Object.keys(baseBySec);

  for (const sec of sections) {
    const canvas = createCanvas(sec);
    const prOps = prBySec[sec] || [];
    const baseOps = baseBySec[sec] || [];

    // Operations as x-axis labels (from PR data, or baseline when there's no PR).
    const ops = (hasPr ? prOps : baseOps).map((r) => r[2]);

    // Index baseline by operation
    const baseByOp = {};
    for (const row of baseOps) {
      baseByOp[row[2]] = { p50: +row[4], p95: +row[5], p99: +row[6] };
    }

    const prByOp = {};
    for (const row of prOps) {
      prByOp[row[2]] = { p50: +row[4], p95: +row[5], p99: +row[6] };
    }

    // Stacked deltas
    const devP50 = ops.map((o) => baseByOp[o]?.p50 ?? null);
    const devP95d = ops.map((o) => baseByOp[o] ? baseByOp[o].p95 - baseByOp[o].p50 : null);
    const devP99d = ops.map((o) => baseByOp[o] ? baseByOp[o].p99 - baseByOp[o].p95 : null);

    let devLabels;
    let devColors;
    if (hasPr) {
      devLabels = ["dev p50", "dev p95\u2212p50", "dev p99\u2212p95"];
      devColors = ["#00adb580", "#e2b93d80", "#e9456080"];
    } else {
      devLabels = ["p50", "p95 \u2212 p50", "p99 \u2212 p95"];
      devColors = ["#00adb5", "#e2b93d", "#e94560"];
    }

    const datasets = [
      { label: devLabels[0], data: devP50, backgroundColor: devColors[0], stack: "dev", borderWidth: 0 },
      { label: devLabels[1], data: devP95d, backgroundColor: devColors[1], stack: "dev", borderWidth: 0 },
      { label: devLabels[2], data: devP99d, backgroundColor: devColors[2], stack: "dev", borderWidth: 0 },
    ];

    if (hasPr) {
      const prP50 = ops.map((o) => prByOp[o]?.p50 ?? null);
      const prP95d = ops.map((o) => prByOp[o] ? prByOp[o].p95 - prByOp[o].p50 : null);
      const prP99d = ops.map((o) => prByOp[o] ? prByOp[o].p99 - prByOp[o].p95 : null);
      datasets.push(
        { label: "PR p50", data: prP50, backgroundColor: "#00ffd5", stack: "pr", borderWidth: 0 },
        { label: "PR p95\u2212p50", data: prP95d, backgroundColor: "#ffe066", stack: "pr", borderWidth: 0 },
        { label: "PR p99\u2212p95", data: prP99d, backgroundColor: "#ff6b81", stack: "pr", borderWidth: 0 }
      );
    }

    const chart = new Chart(canvas, {
      type: "bar",
      data: {
        labels: ops,
        datasets,
      },
      options: {
        responsive: true,
        plugins: {
          legend: { labels: { color: "#e0e0e0", font: { size: 11 } } },
        },
        scales: {
          x: { stacked: true, ticks: { color: "#888" }, grid: { color: "#0f3460" } },
          y: { stacked: true, ticks: { color: "#888" }, grid: { color: "#0f3460" }, title: { display: true, text: "Latency (\u03bcs)", color: "#888" } },
        },
      },
    });
    chartInstances.push(chart);
  }
}

async function renderBench(benchKey, payload) {
  const status = document.getElementById("status");
  const historyOnly = !!payload.h;

  const baseline = await fetchBaseline(
    benchKey,
    payload.r || "linux-baremetal",
    payload.m || "microvm",
    payload.a || "X64"
  );

  if (historyOnly) {
    if (!baseline) {
      status.textContent = `No history data for ${benchKey}`;
      destroyCharts();
      return;
    }
    if (VFS_BENCHMARKS.has(benchKey)) {
      renderVfs(baseline, [], payload.c);
    } else if (SIZED_BENCHMARKS.has(benchKey)) {
      renderSized(baseline, [], payload.c);
    } else {
      renderStandard(baseline, null, payload.c);
    }
    status.textContent = "";
    return;
  }

  const csvBlob = payload.b[benchKey];
  if (!csvBlob) {
    status.textContent = `No data for ${benchKey}`;
    destroyCharts();
    return;
  }

  const prData = decodeCsv(csvBlob);

  if (VFS_BENCHMARKS.has(benchKey)) {
    const prRows = prData.rows.filter((r) => r[0] === payload.c);
    renderVfs(
      baseline || { header: prData.header, rows: [] },
      prRows,
      payload.c
    );
  } else if (SIZED_BENCHMARKS.has(benchKey)) {
    const prRows = prData.rows.filter((r) => r[0] === payload.c);
    renderSized(
      baseline || { header: prData.header, rows: [] },
      prRows,
      payload.c
    );
  } else {
    const prRow = prData.rows.find((r) => r[0] === payload.c) || prData.rows[0];
    renderStandard(
      baseline || { header: prData.header, rows: [] },
      prRow,
      payload.c
    );
  }

  status.textContent = "";
}

function init() {
  const status = document.getElementById("status");
  const select = document.getElementById("bench-select");
  const commitLabel = document.getElementById("commit-label");
  const runnerLabel = document.getElementById("runner-label");

  const payload = decodePayload() || defaultHistoryPayload();

  commitLabel.textContent = payload.h
    ? "history only"
    : `commit ${payload.c.slice(0, SHORT_SHA)}`;
  if (payload.r) runnerLabel.textContent = payload.r;

  const benchKeys = Object.keys(payload.b);
  if (benchKeys.length === 0) {
    status.textContent = "Payload contains no benchmark data.";
    return;
  }

  select.disabled = false;
  select.innerHTML = "";
  for (const key of benchKeys) {
    const opt = document.createElement("option");
    opt.value = key;
    opt.textContent = key;
    select.appendChild(opt);
  }

  select.addEventListener("change", () => renderBench(select.value, payload));
  renderBench(benchKeys[0], payload);
}

document.addEventListener("DOMContentLoaded", init);

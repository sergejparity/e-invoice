async function pickFolder() {
  try {
    console.log("Calling pick_folder command...");
    const dir = await invoke("pick_folder");
    console.log("Command returned:", dir);
    return dir || null;
  } catch (e) {
    console.error("Folder picker error:", e);
    alert("Folder picker error: " + e + "\n\nPlease enter path manually.");
    const manual = prompt("Enter folder path with XML invoices:");
    return manual || null;
  }
}

const state = { dir: null, files: [], results: [] };

function renderFiles() {
  const tbody = document.getElementById("list");
  tbody.innerHTML = "";
  for (const f of state.files) {
    const r = state.results.find((x) => x.path === f.path);
    const tr = document.createElement("tr");
    tr.innerHTML = `<td>${f.path}</td><td>${f.size_bytes}</td><td>${r ? (r.valid ? '<span class="ok">OK</span>' : '<span class="err">NO</span>') : ''}</td><td>${r && r.errors ? r.errors.join("; ") : ''}</td>`;
    tbody.appendChild(tr);
  }
}

function renderJobs(jobs) {
  const tbody = document.getElementById("jobs");
  tbody.innerHTML = "";
  for (const j of jobs) {
    const tr = document.createElement("tr");
    const updated = j.updated_at ? new Date(j.updated_at).toLocaleString() : "";
    tr.innerHTML = `<td>${j.job_id}</td><td>${j.state}</td><td>${updated}</td><td>${j.transmission_id || ''}</td><td>${j.last_error || ''}</td>`;
    tbody.appendChild(tr);
  }
}

async function invoke(cmd, args) {
  // Use the global __TAURI_INVOKE__ injected by Tauri
  if (typeof window.__TAURI_INVOKE__ === "function") {
    return await window.__TAURI_INVOKE__(cmd, args);
  }
  // Fallback to __TAURI__ API if available
  if (window.__TAURI__ && window.__TAURI__.tauri && window.__TAURI__.tauri.invoke) {
    return await window.__TAURI__.tauri.invoke(cmd, args);
  }
  throw new Error("Tauri invoke not available");
}

async function refreshJobs() {
  try {
    const jobs = await invoke("list_status");
    renderJobs(jobs);
  } catch (e) {
    console.error(e);
  }
}

function updateDebugStatus(msg) {
  const elem = document.getElementById("debug-status");
  if (elem) {
    elem.textContent = msg;
    elem.style.background = "#d4edda";
  }
  console.log("[DEBUG]", msg);
}

async function main() {
  updateDebugStatus("Script loaded. Checking Tauri API...");
  console.log("App initializing...");
  console.log("Tauri API available:", !!window.__TAURI__);
  console.log("window.__TAURI__ =", window.__TAURI__);
  
  if (!window.__TAURI__) {
    updateDebugStatus("ERROR: Tauri API not available!");
    alert("Tauri API not loaded. This app must run in Tauri.");
    return;
  }
  
  updateDebugStatus("Ready! Tauri API loaded successfully.");

  document.getElementById("pick").onclick = async () => {
    updateDebugStatus("Opening folder picker...");
    console.log("Pick button clicked");
    const d = await pickFolder();
    if (d) {
      state.dir = d;
      document.getElementById("folder").textContent = d;
      updateDebugStatus("Folder selected: " + d);
      console.log("Selected folder:", d);
    } else {
      updateDebugStatus("No folder selected");
    }
  };

  document.getElementById("scan").onclick = async () => {
    if (!state.dir) return alert("Pick a folder first");
    console.log("Scanning folder:", state.dir);
    state.files = await invoke("scan_folder", { dir: state.dir });
    state.results = [];
    renderFiles();
    console.log("Found files:", state.files.length);
  };

  document.getElementById("validate").onclick = async () => {
    if (state.files.length === 0) return alert("Scan first");
    console.log("Validating", state.files.length, "files");
    const paths = state.files.map((f) => f.path);
    state.results = await invoke("validate_invoices", { paths });
    renderFiles();
    console.log("Validation complete");
  };

  document.getElementById("send").onclick = async () => {
    if (state.results.length === 0 || state.results.some((r) => !r.valid)) {
      return alert("Validate and ensure all invoices are valid before sending");
    }
    const paths = state.files.map((f) => f.path);
    try {
      const resp = await invoke("enqueue_send", {
        req: {
          paths,
          sender: "LV:YOUR-SENDER-ID",
          receiver: "LV:RECEIVER-ID",
          profile: "peppol-bis-3",
        },
      });
      if (resp && resp.job_ids) {
        alert(`Enqueued ${resp.job_ids.length} invoices`);
      }
      await refreshJobs();
    } catch (e) {
      console.error(e);
      alert(`Failed to enqueue: ${e}`);
    }
  };

  await refreshJobs();
  setInterval(refreshJobs, 2000);
  console.log("App initialized successfully");
}

// Wait for DOM and run
if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", main);
} else {
  main();
}

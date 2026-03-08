<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount, onDestroy, tick } from "svelte";

  type Protocol = "tcp" | "udp";
  type RuleStatus = "stopped" | "running" | { error: string };

  interface RuleStats {
    bytes_up: number;
    bytes_down: number;
    connections: number;
  }

  interface ForwardRule {
    id: string;
    name: string;
    protocol: Protocol;
    listen_addr: string;
    target_addr: string;
    enabled: boolean;
    status: RuleStatus;
    stats: RuleStats;
  }

  let rules = $state<ForwardRule[]>([]);
  let showAddModal = $state(false);
  let editingRule = $state<ForwardRule | null>(null);
  let loading = $state<Record<string, boolean>>({});
  let addErrorMsg = $state("");
  let editErrorMsg = $state("");
  let logs = $state<string[]>([]);
  let logsEl = $state<HTMLDivElement | null>(null);
  let refreshInterval: number;
  let logInterval: number;

  let addForm = $state({
    name: "",
    protocol: "tcp" as Protocol,
    listen_addr: "",
    target_addr: "",
  });

  let editForm = $state({
    name: "",
    protocol: "tcp" as Protocol,
    listen_addr: "",
    target_addr: "",
  });

  async function loadRules() {
    try {
      rules = await invoke<ForwardRule[]>("list_rules");
    } catch (e) {
      console.error("Failed to load rules:", e);
    }
  }

  async function loadLogs() {
    try {
      const newLogs = await invoke<string[]>("get_logs", { limit: 300 });
      logs = newLogs;
      await tick();
      if (logsEl) logsEl.scrollTop = logsEl.scrollHeight;
    } catch {
      // ignore
    }
  }

  async function addRule() {
    if (!addForm.listen_addr || !addForm.target_addr) {
      addErrorMsg = "Listen address and target address are required";
      return;
    }
    const addrPattern = /^.+:\d+$/;
    if (!addrPattern.test(addForm.listen_addr)) {
      addErrorMsg = "Listen address must be in format host:port";
      return;
    }
    if (!addrPattern.test(addForm.target_addr)) {
      addErrorMsg = "Target address must be in format host:port";
      return;
    }
    try {
      await invoke("add_rule", { req: addForm });
      showAddModal = false;
      addForm = { name: "", protocol: "tcp", listen_addr: "", target_addr: "" };
      addErrorMsg = "";
      await loadRules();
    } catch (e) {
      addErrorMsg = String(e);
    }
  }

  function openEdit(rule: ForwardRule) {
    editingRule = rule;
    editForm = {
      name: rule.name,
      protocol: rule.protocol,
      listen_addr: rule.listen_addr,
      target_addr: rule.target_addr,
    };
    editErrorMsg = "";
  }

  async function saveEdit() {
    if (!editingRule) return;
    if (!editForm.listen_addr || !editForm.target_addr) {
      editErrorMsg = "Listen address and target address are required";
      return;
    }
    const addrPattern = /^.+:\d+$/;
    if (!addrPattern.test(editForm.listen_addr)) {
      editErrorMsg = "Listen address must be in format host:port";
      return;
    }
    if (!addrPattern.test(editForm.target_addr)) {
      editErrorMsg = "Target address must be in format host:port";
      return;
    }
    try {
      await invoke("update_rule", {
        req: { id: editingRule.id, ...editForm },
      });
      editingRule = null;
      await loadRules();
    } catch (e) {
      editErrorMsg = String(e);
    }
  }

  function cancelEdit() {
    editingRule = null;
    editErrorMsg = "";
  }

  async function toggleRule(rule: ForwardRule) {
    loading[rule.id] = true;
    try {
      if (rule.enabled) {
        await invoke("stop_rule", { id: rule.id });
      } else {
        await invoke("start_rule", { id: rule.id });
      }
      await loadRules();
    } catch (e) {
      alert(String(e));
    } finally {
      loading[rule.id] = false;
    }
  }

  async function removeRule(rule: ForwardRule) {
    if (rule.enabled) {
      alert("Stop the rule before removing it");
      return;
    }
    loading[rule.id] = true;
    try {
      await invoke("remove_rule", { id: rule.id });
      await loadRules();
    } catch (e) {
      alert(String(e));
    } finally {
      loading[rule.id] = false;
    }
  }

  async function exportRules() {
    try {
      const json = await invoke<string>("export_rules");
      const blob = new Blob([json], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = "portopener-rules.json";
      a.click();
      URL.revokeObjectURL(url);
    } catch (e) {
      alert(String(e));
    }
  }

  async function importRules() {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = ".json";
    input.onchange = async () => {
      const file = input.files?.[0];
      if (!file) return;
      try {
        const text = await file.text();
        const count = await invoke<number>("import_rules", { json: text });
        await loadRules();
        alert(`Imported ${count} rule(s)`);
      } catch (e) {
        alert(String(e));
      }
    };
    input.click();
  }

  function formatBytes(n: number): string {
    if (n < 1024) return `${n} B`;
    if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
    if (n < 1024 * 1024 * 1024) return `${(n / 1024 / 1024).toFixed(1)} MB`;
    return `${(n / 1024 / 1024 / 1024).toFixed(2)} GB`;
  }

  function statusLabel(status: RuleStatus): string {
    if (status === "running") return "Running";
    if (status === "stopped") return "Stopped";
    if (typeof status === "object" && "error" in status) return `Error: ${status.error}`;
    return "Unknown";
  }

  function statusClass(status: RuleStatus): string {
    if (status === "running") return "status-running";
    if (typeof status === "object" && "error" in status) return "status-error";
    return "status-stopped";
  }

  // Add modal: only ESC closes it (not edit modal)
  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape" && showAddModal) {
      showAddModal = false;
      addErrorMsg = "";
    }
  }

  onMount(() => {
    loadRules();
    loadLogs();
    refreshInterval = setInterval(loadRules, 2000);
    logInterval = setInterval(loadLogs, 2000);
    window.addEventListener("keydown", handleKeydown);
    // Disable context menu
    window.addEventListener("contextmenu", (e) => e.preventDefault());
  });

  onDestroy(() => {
    if (refreshInterval) clearInterval(refreshInterval);
    if (logInterval) clearInterval(logInterval);
    window.removeEventListener("keydown", handleKeydown);
  });
</script>

<div class="app">
  <header>
    <h1>Port Forwarder</h1>
    <div class="header-actions">
      <button class="btn-secondary" onclick={importRules}>Import</button>
      <button class="btn-secondary" onclick={exportRules}>Export</button>
      <button class="btn-primary" onclick={() => (showAddModal = true)}>+ Add Rule</button>
    </div>
  </header>

  <div class="table-wrap">
    {#if rules.length === 0}
      <div class="empty">No forwarding rules yet. Click "Add Rule" to get started.</div>
    {:else}
      <table>
        <thead>
          <tr>
            <th>Name</th>
            <th>Protocol</th>
            <th>Listen</th>
            <th>Target</th>
            <th>Status</th>
            <th>Conns</th>
            <th>Up / Down</th>
            <th>Actions</th>
          </tr>
        </thead>
        <tbody>
          {#each rules as rule (rule.id)}
            <tr ondblclick={() => openEdit(rule)} title="Double-click to edit">
              <td>{rule.name || "—"}</td>
              <td><span class="badge badge-{rule.protocol}">{rule.protocol.toUpperCase()}</span></td>
              <td class="mono">{rule.listen_addr}</td>
              <td class="mono">{rule.target_addr}</td>
              <td><span class="status {statusClass(rule.status)}">{statusLabel(rule.status)}</span></td>
              <td class="mono dim">{rule.enabled ? rule.stats.connections : "—"}</td>
              <td class="mono dim">
                {#if rule.enabled || rule.stats.bytes_up > 0}
                  {formatBytes(rule.stats.bytes_up)} / {formatBytes(rule.stats.bytes_down)}
                {:else}
                  —
                {/if}
              </td>
              <td class="actions">
                <button
                  class="btn-sm {rule.enabled ? 'btn-stop' : 'btn-start'}"
                  disabled={loading[rule.id]}
                  onclick={(e) => { e.stopPropagation(); toggleRule(rule); }}
                >
                  {loading[rule.id] ? "..." : rule.enabled ? "Stop" : "Start"}
                </button>
                <button
                  class="btn-sm btn-danger"
                  disabled={loading[rule.id] || rule.enabled}
                  onclick={(e) => { e.stopPropagation(); removeRule(rule); }}
                >
                  Delete
                </button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </div>

  <div class="log-panel">
    <div class="log-panel-header">
      <span>Logs</span>
      <button class="btn-sm btn-secondary" onclick={() => { logs = []; }}>Clear</button>
    </div>
    <div class="log-body" bind:this={logsEl}>
      {#if logs.length === 0}
        <span class="dim">No logs yet.</span>
      {:else}
        {#each logs as line}
          <div class="log-line">{line}</div>
        {/each}
      {/if}
    </div>
  </div>
</div>

<!-- Add Rule Modal -->
{#if showAddModal}
  <div class="overlay" onclick={() => { showAddModal = false; addErrorMsg = ""; }} role="presentation">
    <div class="modal" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()} role="dialog" aria-modal="true" aria-label="Add forwarding rule" tabindex="-1">
      <h2>Add Forwarding Rule</h2>

      {#if addErrorMsg}
        <div class="error-banner">{addErrorMsg}</div>
      {/if}

      <label>
        Name (optional)
        <input bind:value={addForm.name} placeholder="e.g. My SSH Tunnel" onkeydown={(e) => e.key === "Enter" && addRule()} />
      </label>
      <label>
        Protocol
        <select bind:value={addForm.protocol}>
          <option value="tcp">TCP</option>
          <option value="udp">UDP</option>
        </select>
      </label>
      <label>
        Listen Address
        <input bind:value={addForm.listen_addr} placeholder="0.0.0.0:8080" onkeydown={(e) => e.key === "Enter" && addRule()} />
      </label>
      <label>
        Target Address
        <input bind:value={addForm.target_addr} placeholder="192.168.1.1:80" onkeydown={(e) => e.key === "Enter" && addRule()} />
      </label>

      <div class="modal-actions">
        <button class="btn-secondary" onclick={() => { showAddModal = false; addErrorMsg = ""; }}>Cancel</button>
        <button class="btn-primary" onclick={addRule}>Add</button>
      </div>
    </div>
  </div>
{/if}

<!-- Edit Rule Modal — only Save/Cancel can close it -->
{#if editingRule}
  <div class="overlay" role="presentation">
    <div class="modal" role="dialog" aria-modal="true" aria-label="Edit forwarding rule" tabindex="-1">
      <h2>Edit Rule</h2>

      {#if editErrorMsg}
        <div class="error-banner">{editErrorMsg}</div>
      {/if}

      <label>
        Name (optional)
        <input bind:value={editForm.name} placeholder="e.g. My SSH Tunnel" />
      </label>
      <label>
        Protocol
        <select bind:value={editForm.protocol} disabled={editingRule.enabled}>
          <option value="tcp">TCP</option>
          <option value="udp">UDP</option>
        </select>
      </label>
      <label>
        Listen Address
        <input bind:value={editForm.listen_addr} placeholder="0.0.0.0:8080" disabled={editingRule.enabled} />
      </label>
      <label>
        Target Address
        <input bind:value={editForm.target_addr} placeholder="192.168.1.1:80" disabled={editingRule.enabled} />
      </label>

      {#if editingRule.enabled}
        <div class="info-banner">Stop the rule to edit addresses and protocol.</div>
      {/if}

      <div class="modal-actions">
        <button class="btn-secondary" onclick={cancelEdit}>Cancel</button>
        <button class="btn-primary" onclick={saveEdit}>Save</button>
      </div>
    </div>
  </div>
{/if}

<style>
  :global(*, *::before, *::after) { box-sizing: border-box; }
  :global(body) {
    margin: 0;
    font-family: Inter, system-ui, sans-serif;
    font-size: 14px;
    background: #0f1117;
    color: #e2e8f0;
    user-select: none;
  }

  .app {
    max-width: 1100px;
    margin: 0 auto;
    padding: 24px 24px 0;
    display: flex;
    flex-direction: column;
    height: 100vh;
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 16px;
    gap: 12px;
    flex-shrink: 0;
  }

  h1 {
    margin: 0;
    font-size: 20px;
    font-weight: 600;
    color: #f1f5f9;
    white-space: nowrap;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .table-wrap {
    background: #1a1d27;
    border-radius: 10px;
    border: 1px solid #2d3148;
    overflow: hidden;
    flex-shrink: 0;
  }

  .empty {
    padding: 48px;
    text-align: center;
    color: #64748b;
  }

  table {
    width: 100%;
    border-collapse: collapse;
  }

  th {
    padding: 12px 16px;
    text-align: left;
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: #64748b;
    border-bottom: 1px solid #2d3148;
  }

  td {
    padding: 12px 16px;
    border-bottom: 1px solid #1e2235;
    color: #cbd5e1;
  }

  tr:last-child td { border-bottom: none; }
  tr:hover td { background: #1e2235; cursor: pointer; }

  .mono { font-family: 'JetBrains Mono', 'Fira Code', monospace; font-size: 13px; }
  .dim { color: #64748b; }

  .badge {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.05em;
  }
  .badge-tcp { background: #1e3a5f; color: #60a5fa; }
  .badge-udp { background: #2d1f4e; color: #a78bfa; }

  .status {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
  }
  .status::before {
    content: '';
    display: inline-block;
    width: 7px;
    height: 7px;
    border-radius: 50%;
  }
  .status-running { color: #4ade80; }
  .status-running::before { background: #4ade80; box-shadow: 0 0 6px #4ade80; }
  .status-stopped { color: #64748b; }
  .status-stopped::before { background: #64748b; }
  .status-error { color: #f87171; }
  .status-error::before { background: #f87171; }

  .actions { display: flex; gap: 8px; }

  button { cursor: pointer; border: none; border-radius: 6px; font-size: 13px; font-weight: 500; transition: opacity 0.15s; }
  button:disabled { opacity: 0.4; cursor: not-allowed; }

  .btn-primary { padding: 8px 16px; background: #3b82f6; color: #fff; }
  .btn-primary:hover:not(:disabled) { background: #2563eb; }

  .btn-secondary { padding: 8px 16px; background: #2d3148; color: #cbd5e1; }
  .btn-secondary:hover:not(:disabled) { background: #374162; }

  .btn-sm { padding: 5px 12px; font-size: 12px; }
  .btn-start { background: #166534; color: #4ade80; }
  .btn-start:hover:not(:disabled) { background: #15803d; }
  .btn-stop { background: #7c3aed; color: #c4b5fd; }
  .btn-stop:hover:not(:disabled) { background: #6d28d9; }
  .btn-danger { background: #1f1f1f; color: #f87171; border: 1px solid #3f1f1f; }
  .btn-danger:hover:not(:disabled) { background: #3f1f1f; }

  /* Log panel */
  .log-panel {
    margin-top: 16px;
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    background: #1a1d27;
    border: 1px solid #2d3148;
    border-radius: 10px;
    overflow: hidden;
    margin-bottom: 24px;
  }

  .log-panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 16px;
    border-bottom: 1px solid #2d3148;
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: #64748b;
    flex-shrink: 0;
  }

  .log-body {
    flex: 1;
    overflow-y: auto;
    padding: 8px 12px;
    font-family: 'JetBrains Mono', 'Fira Code', monospace;
    font-size: 12px;
    color: #94a3b8;
  }

  .log-line {
    padding: 2px 0;
    border-bottom: 1px solid #1a1d27;
    white-space: pre-wrap;
    word-break: break-all;
    line-height: 1.6;
  }

  /* Modals */
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0,0,0,0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }

  .modal {
    background: #1a1d27;
    border: 1px solid #2d3148;
    border-radius: 12px;
    padding: 28px;
    width: 420px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .modal h2 { margin: 0; font-size: 16px; font-weight: 600; color: #f1f5f9; }

  .modal label {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 13px;
    color: #94a3b8;
  }

  .modal input, .modal select {
    padding: 8px 12px;
    background: #0f1117;
    border: 1px solid #2d3148;
    border-radius: 6px;
    color: #e2e8f0;
    font-size: 14px;
    outline: none;
  }
  .modal input:focus, .modal select:focus { border-color: #3b82f6; }
  .modal input:disabled, .modal select:disabled { opacity: 0.4; cursor: not-allowed; }

  .modal-actions { display: flex; justify-content: flex-end; gap: 10px; margin-top: 4px; }

  .error-banner {
    background: #3f1f1f;
    border: 1px solid #7f1d1d;
    color: #fca5a5;
    padding: 10px 14px;
    border-radius: 6px;
    font-size: 13px;
  }

  .info-banner {
    background: #1e2d3d;
    border: 1px solid #1e3a5f;
    color: #93c5fd;
    padding: 10px 14px;
    border-radius: 6px;
    font-size: 13px;
  }
</style>

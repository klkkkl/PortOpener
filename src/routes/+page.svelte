<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount, onDestroy } from "svelte";

  type Protocol = "tcp" | "udp";
  type RuleStatus = "stopped" | "running" | { error: string };

  interface ForwardRule {
    id: string;
    name: string;
    protocol: Protocol;
    listen_addr: string;
    target_addr: string;
    enabled: boolean;
    status: RuleStatus;
  }

  let rules = $state<ForwardRule[]>([]);
  let showModal = $state(false);
  let loading = $state<Record<string, boolean>>({});
  let errorMsg = $state("");
  let refreshInterval: number;

  let form = $state({
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

  async function addRule() {
    if (!form.listen_addr || !form.target_addr) {
      errorMsg = "Listen address and target address are required";
      return;
    }

    // Validate address format (host:port)
    const addrPattern = /^.+:\d+$/;
    if (!addrPattern.test(form.listen_addr)) {
      errorMsg = "Listen address must be in format host:port (e.g., 0.0.0.0:8080)";
      return;
    }
    if (!addrPattern.test(form.target_addr)) {
      errorMsg = "Target address must be in format host:port (e.g., 192.168.1.1:80)";
      return;
    }

    try {
      await invoke("add_rule", { req: form });
      showModal = false;
      form = { name: "", protocol: "tcp", listen_addr: "", target_addr: "" };
      errorMsg = "";
      await loadRules();
    } catch (e) {
      errorMsg = String(e);
    }
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

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape" && showModal) {
      showModal = false;
      errorMsg = "";
    }
  }

  onMount(() => {
    loadRules();
    // Auto-refresh every 2 seconds to keep UI in sync
    refreshInterval = setInterval(loadRules, 2000);

    // Add global keyboard listener
    window.addEventListener("keydown", handleKeydown);
  });

  onDestroy(() => {
    if (refreshInterval) clearInterval(refreshInterval);
    window.removeEventListener("keydown", handleKeydown);
  });
</script>

<div class="app">
  <header>
    <h1>Port Forwarder</h1>
    <button class="btn-primary" onclick={() => (showModal = true)}>+ Add Rule</button>
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
            <th>Actions</th>
          </tr>
        </thead>
        <tbody>
          {#each rules as rule (rule.id)}
            <tr>
              <td>{rule.name || "—"}</td>
              <td><span class="badge badge-{rule.protocol}">{rule.protocol.toUpperCase()}</span></td>
              <td class="mono">{rule.listen_addr}</td>
              <td class="mono">{rule.target_addr}</td>
              <td><span class="status {statusClass(rule.status)}">{statusLabel(rule.status)}</span></td>
              <td class="actions">
                <button
                  class="btn-sm {rule.enabled ? 'btn-stop' : 'btn-start'}"
                  disabled={loading[rule.id]}
                  onclick={() => toggleRule(rule)}
                >
                  {loading[rule.id] ? "..." : rule.enabled ? "Stop" : "Start"}
                </button>
                <button
                  class="btn-sm btn-danger"
                  disabled={loading[rule.id] || rule.enabled}
                  onclick={() => removeRule(rule)}
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
</div>

{#if showModal}
  <div class="overlay" onclick={() => (showModal = false)} role="presentation">
    <div class="modal" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()} role="dialog" aria-modal="true" aria-label="Add forwarding rule" tabindex="-1">
      <h2>Add Forwarding Rule</h2>

      {#if errorMsg}
        <div class="error-banner">{errorMsg}</div>
      {/if}

      <label>
        Name (optional)
        <input bind:value={form.name} placeholder="e.g. My SSH Tunnel" onkeydown={(e) => e.key === "Enter" && addRule()} />
      </label>

      <label>
        Protocol
        <select bind:value={form.protocol}>
          <option value="tcp">TCP</option>
          <option value="udp">UDP</option>
        </select>
      </label>

      <label>
        Listen Address
        <input bind:value={form.listen_addr} placeholder="0.0.0.0:8080" onkeydown={(e) => e.key === "Enter" && addRule()} />
      </label>

      <label>
        Target Address
        <input bind:value={form.target_addr} placeholder="192.168.1.1:80" onkeydown={(e) => e.key === "Enter" && addRule()} />
      </label>

      <div class="modal-actions">
        <button class="btn-secondary" onclick={() => { showModal = false; errorMsg = ""; }}>Cancel</button>
        <button class="btn-primary" onclick={addRule}>Add</button>
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
  }

  .app {
    max-width: 900px;
    margin: 0 auto;
    padding: 32px 24px;
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 24px;
  }

  h1 {
    margin: 0;
    font-size: 20px;
    font-weight: 600;
    color: #f1f5f9;
  }

  .table-wrap {
    background: #1a1d27;
    border-radius: 10px;
    border: 1px solid #2d3148;
    overflow: hidden;
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
  tr:hover td { background: #1e2235; }

  .mono { font-family: 'JetBrains Mono', 'Fira Code', monospace; font-size: 13px; }

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

  /* Modal */
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

  .modal-actions { display: flex; justify-content: flex-end; gap: 10px; margin-top: 4px; }

  .error-banner {
    background: #3f1f1f;
    border: 1px solid #7f1d1d;
    color: #fca5a5;
    padding: 10px 14px;
    border-radius: 6px;
    font-size: 13px;
  }
</style>

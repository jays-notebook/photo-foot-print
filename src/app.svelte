<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  interface SummaryDto {
    path: string;
    file_name: string;
    has_gps: boolean;
    capture_time: string | null;
  }

  // Discriminated union mirroring src-tauri's WireError.
  type WireError =
    | { kind: "io"; detail: string }
    | { kind: "exif"; detail: string }
    | { kind: "path_traversal"; detail: string };

  type Row =
    | { kind: "ok"; path: string; summary: SummaryDto }
    | { kind: "err"; path: string; error: WireError | string };

  let loading = $state(true);
  let rows: Row[] = $state([]);
  let bootstrapError: string | null = $state(null);

  async function loadFixtures() {
    try {
      const fixtures = await invoke<string[]>("list_fixtures");
      const collected: Row[] = [];
      for (const path of fixtures) {
        try {
          const summary = await invoke<SummaryDto>("read_exif_summary", { path });
          collected.push({ kind: "ok", path, summary });
        } catch (err) {
          collected.push({
            kind: "err",
            path,
            error:
              typeof err === "object" && err !== null
                ? (err as WireError)
                : String(err),
          });
        }
      }
      rows = collected;
    } catch (err) {
      bootstrapError = `Failed to list fixtures: ${JSON.stringify(err)}`;
    } finally {
      loading = false;
    }
  }

  // Fire-and-forget on mount. Svelte 5 runes: top-level await is allowed in
  // <script> but loadFixtures() returns a promise we deliberately do not block on.
  loadFixtures();
</script>

<main>
  <h1>photo-foot-print — Phase 1 demo</h1>

  <p class="subtitle">
    EXIF summaries read from <code>tests/fixtures/{'{'}sony,canon,nikon{'}'}/sample.jpg</code>
    via the <code>read_exif_summary</code> IPC command.
  </p>

  {#if loading}
    <p>Loading fixture summaries…</p>
  {:else if bootstrapError}
    <p class="error">{bootstrapError}</p>
  {:else if rows.length === 0}
    <p class="empty-state">
      No fixtures supplied yet. See <code>docs/FIXTURES.md</code> for the
      <strong>[needs_user_action]</strong> checklist.
    </p>
  {:else}
    <ul>
      {#each rows as row}
        {#if row.kind === "ok"}
          <li>
            <strong>{row.summary.file_name}</strong>
            — {row.summary.has_gps ? "has GPS" : "missing GPS"}
            {#if row.summary.capture_time}
              — {row.summary.capture_time}
            {:else}
              — no DateTimeOriginal
            {/if}
          </li>
        {:else}
          <li class="error">
            <strong>{row.path}</strong>: ERROR
            {typeof row.error === "string"
              ? row.error
              : `${row.error.kind}: ${row.error.detail}`}
          </li>
        {/if}
      {/each}
    </ul>
  {/if}
</main>

<style>
  .subtitle {
    color: #666;
    font-size: 0.9rem;
  }
  .empty-state {
    padding: 1rem;
    background: #fff8e1;
    border-radius: 4px;
  }
  ul {
    list-style: none;
    padding: 0;
  }
  li {
    padding: 0.5rem 0;
    border-bottom: 1px solid #eee;
  }
  .error {
    color: #b00020;
  }
  code {
    background: #f4f4f4;
    padding: 0.1rem 0.3rem;
    border-radius: 3px;
    font-size: 0.9em;
  }
</style>

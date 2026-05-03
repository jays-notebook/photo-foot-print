<script lang="ts">
  import type { FolderFooter } from "../ipc";

  interface Props {
    footer: FolderFooter;
    loading: boolean;
  }
  let { footer, loading }: Props = $props();

  let parts = $derived.by(() => {
    if (loading) return ["Loading..."];
    const segments = [`Total ${footer.total_jpegs} JPEGs`];
    if (footer.non_image_hidden > 0) {
      segments.push(`${footer.non_image_hidden} non-image hidden`);
    }
    if (footer.read_failed > 0) {
      segments.push(`${footer.read_failed} read failed`);
    }
    return segments;
  });
</script>

<footer aria-label="Folder summary">
  {parts.join(" · ")}
</footer>

<style>
  footer {
    height: var(--footer-h);
    background: var(--bg-secondary);
    border-top: 1px solid var(--border);
    color: var(--text-muted);
    font-size: 14px;
    font-variant-numeric: tabular-nums;
    display: flex;
    align-items: center;
    padding: 0 var(--sp-md);
    flex-shrink: 0;
  }
</style>

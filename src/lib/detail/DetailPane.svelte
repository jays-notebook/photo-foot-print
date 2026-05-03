<script lang="ts">
  import type { PhotoSummary } from "../ipc";

  // Stub — full implementation lands in Task 2.
  interface Props {
    summary: PhotoSummary | null;
  }
  let { summary }: Props = $props();
  void summary;
</script>

<div>DetailPane (stub)</div>

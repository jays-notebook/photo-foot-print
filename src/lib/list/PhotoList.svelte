<script lang="ts">
  import type { PhotoSummary, FolderFooter } from "../ipc";

  // Stub — full implementation lands in Task 2.
  interface Props {
    items: PhotoSummary[];
    footer: FolderFooter;
    selectedId: string | null;
    thumbReady: Record<string, number>;
    loading: boolean;
    onSelect: (id: string) => void;
  }
  let { items, footer, selectedId, thumbReady, loading, onSelect }: Props = $props();
  void items;
  void footer;
  void selectedId;
  void thumbReady;
  void loading;
  void onSelect;
</script>

<div>PhotoList (stub)</div>

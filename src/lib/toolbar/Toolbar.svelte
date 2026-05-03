<script lang="ts">
  // Stub — full implementation lands in Task 2.
  interface Props {
    folderPath: string | null;
    loading: boolean;
    onOpen: (path: string) => Promise<void>;
    onRefresh: () => Promise<void> | void;
  }
  let { folderPath, loading, onOpen, onRefresh }: Props = $props();
  // Reference props so TypeScript sees them as used.
  void folderPath;
  void loading;
  void onOpen;
  void onRefresh;
</script>

<header>Toolbar (stub)</header>

<style>
  header {
    grid-area: toolbar;
    height: var(--toolbar-h);
  }
</style>

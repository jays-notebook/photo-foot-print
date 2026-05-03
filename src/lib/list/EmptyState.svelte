<script lang="ts">
  // Stub — full implementation lands in Task 2.
  interface Props {
    note?: string | null;
    heading?: string;
    body?: string;
  }
  let {
    note = null,
    heading = "Open a folder to start.",
    body = "Pick a folder containing scanner JPEGs.",
  }: Props = $props();
  void note;
  void heading;
  void body;
</script>

<div>EmptyState (stub)</div>

<script lang="ts">
  interface Props {
    ondismiss: () => void;
  }
  let { ondismiss }: Props = $props();
</script>

<div class="banner" role="alert">
  <span class="msg">
    Cannot write to this folder. Thumbnails won't be cached. The folder may be on a read-only volume or you don't have write permission.
  </span>
  <button type="button" class="dismiss" onclick={ondismiss} aria-label="Dismiss"
    >×</button
  >
</div>

<style>
  .banner {
    height: var(--banner-h);
    background: var(--bg-secondary);
    border-left: 4px solid var(--destructive);
    color: var(--text-default);
    font-size: 14px;
    display: flex;
    align-items: center;
    padding: 0 var(--sp-md);
    gap: var(--sp-sm);
    flex-shrink: 0;
  }
  .msg {
    flex: 1;
  }
  .dismiss {
    background: transparent;
    border: none;
    color: var(--text-muted);
    font-size: 18px;
    line-height: 1;
    cursor: pointer;
    width: 24px;
    height: 24px;
  }
  .dismiss:hover {
    color: var(--text-default);
  }
</style>

<script>
  let { percent = 0, size = 48, strokeWidth = 4, color = 'text-brand-500', label = '' } = $props();

  const radius = $derived((size - strokeWidth) / 2);
  const circumference = $derived(2 * Math.PI * radius);
  const offset = $derived(circumference - (percent / 100) * circumference);
</script>

<div
  class="relative inline-flex items-center justify-center"
  style="width: {size}px; height: {size}px;"
>
  <svg width={size} height={size} class="transform -rotate-90">
    <!-- Background ring -->
    <circle
      cx={size / 2}
      cy={size / 2}
      r={radius}
      fill="none"
      stroke="currentColor"
      stroke-width={strokeWidth}
      class="text-gray-200"
    />
    <!-- Progress ring -->
    <circle
      cx={size / 2}
      cy={size / 2}
      r={radius}
      fill="none"
      stroke="currentColor"
      stroke-width={strokeWidth}
      stroke-linecap="round"
      stroke-dasharray={circumference}
      stroke-dashoffset={offset}
      class="{color} transition-all duration-500"
    />
  </svg>
  <div class="absolute inset-0 flex flex-col items-center justify-center">
    <span class="text-xs font-bold leading-none font-mono tabular-nums">{percent}%</span>
    {#if label}
      <span class="text-[8px] text-gray-400 leading-none mt-0.5">{label}</span>
    {/if}
  </div>
</div>

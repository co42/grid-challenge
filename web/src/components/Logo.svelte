<script>
  let { size = 24, class: className = '' } = $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 32 32"
  fill="none"
  xmlns="http://www.w3.org/2000/svg"
  class={className}
>
  <!-- Solid background -->
  <rect width="32" height="32" rx="7" fill="currentColor" />

  <!-- Grid lines -->
  <line x1="11" y1="0" x2="11" y2="32" stroke="white" stroke-width="0.75" opacity="0.2" />
  <line x1="21" y1="0" x2="21" y2="32" stroke="white" stroke-width="0.75" opacity="0.2" />
  <line x1="0" y1="11" x2="32" y2="11" stroke="white" stroke-width="0.75" opacity="0.2" />
  <line x1="0" y1="21" x2="32" y2="21" stroke="white" stroke-width="0.75" opacity="0.2" />

  <!-- Visited cells -->
  <rect x="0" y="11" width="11" height="10" fill="white" opacity="0.12" rx="1" />
  <rect x="11" y="0" width="10" height="11" fill="white" opacity="0.12" rx="1" />
  <rect x="21" y="11" width="11" height="10" fill="white" opacity="0.08" rx="1" />
  <rect x="11" y="21" width="10" height="11" fill="white" opacity="0.08" rx="1" />

  <!-- Trail path -->
  <path
    d="M4 28 C7 21, 11 17, 15 15 S21 10, 28 4"
    stroke="white"
    stroke-width="3"
    stroke-linecap="round"
    fill="none"
    opacity="0.5"
  />

  <!-- Runner -->
  <circle cx="16.5" cy="10" r="2.5" fill="white" />
  <line
    x1="16.5"
    y1="12.5"
    x2="16.5"
    y2="18.5"
    stroke="white"
    stroke-width="2.2"
    stroke-linecap="round"
  />
  <line
    x1="16.5"
    y1="18.5"
    x2="20"
    y2="23"
    stroke="white"
    stroke-width="2"
    stroke-linecap="round"
  />
  <line
    x1="16.5"
    y1="18.5"
    x2="13"
    y2="22.5"
    stroke="white"
    stroke-width="2"
    stroke-linecap="round"
  />
  <line x1="16.5" y1="14" x2="13" y2="17" stroke="white" stroke-width="2" stroke-linecap="round" />
  <line
    x1="16.5"
    y1="14"
    x2="20"
    y2="16.5"
    stroke="white"
    stroke-width="2"
    stroke-linecap="round"
  />
</svg>

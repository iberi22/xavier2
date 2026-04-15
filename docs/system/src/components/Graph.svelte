<script>
  import { onMount } from 'svelte';

  // Sample graph data - in production this would come from API
  const nodes = [
    { id: 'src', label: 'SRC', type: 'module', x: 150, y: 150 },
    { id: 'architecture', label: 'Architecture', type: 'core', x: 150, y: 50 },
    { id: 'project-admin', label: 'Project Admin', type: 'admin', x: 50, y: 250 },
    { id: 'features', label: 'Features', type: 'tracking', x: 250, y: 250 },
    { id: 'agent-index', label: 'Agent Index', type: 'agents', x: 50, y: 50 },
    { id: 'cli-config', label: 'CLI Config', type: 'config', x: 250, y: 50 },
  ];

  const edges = [
    { from: 'src', to: 'architecture' },
    { from: 'src', to: 'agent-index' },
    { from: 'architecture', to: 'features' },
    { from: 'project-admin', to: 'features' },
    { from: 'agent-index', to: 'cli-config' },
  ];

  const colors = {
    module: '#e94560',
    core: '#0f3460',
    admin: '#16213e',
    tracking: '#533483',
    agents: '#e94560',
    config: '#0f3460',
  };

  let canvas;
  let selectedNode = null;

  onMount(() => {
    const ctx = canvas.getContext('2d');
    draw();
  });

  function draw() {
    const ctx = canvas.getContext('2d');
    ctx.clearRect(0, 0, canvas.width, canvas.height);

    // Draw edges
    edges.forEach(edge => {
      const from = nodes.find(n => n.id === edge.from);
      const to = nodes.find(n => n.id === edge.to);
      if (from && to) {
        ctx.beginPath();
        ctx.moveTo(from.x, from.y);
        ctx.lineTo(to.x, to.y);
        ctx.strokeStyle = '#0f3460';
        ctx.lineWidth = 1;
        ctx.stroke();
      }
    });

    // Draw nodes
    nodes.forEach(node => {
      ctx.beginPath();
      ctx.arc(node.x, node.y, 20, 0, Math.PI * 2);
      ctx.fillStyle = colors[node.type] || '#e94560';
      ctx.fill();

      // Label
      ctx.fillStyle = '#eee';
      ctx.font = '10px sans-serif';
      ctx.textAlign = 'center';
      ctx.fillText(node.label, node.x, node.y + 35);
    });
  }

  function handleClick(event) {
    const rect = canvas.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;

    // Find clicked node
    const clicked = nodes.find(n => {
      const dx = n.x - x;
      const dy = n.y - y;
      return Math.sqrt(dx*dx + dy*dy) < 20;
    });

    selectedNode = clicked;
    if (clicked) {
      window.location.href = `/doc/${clicked.id}`;
    }
  }
</script>

<div class="graph-container">
  <h3 class="graph-title">Knowledge Graph</h3>
  <canvas 
    bind:this={canvas}
    width="280"
    height="280"
    on:click={handleClick}
    class="graph-canvas"
  ></canvas>
  <p class="hint">Click a node to navigate</p>
</div>

<style>
  .graph-container {
    text-align: center;
  }

  .graph-title {
    font-size: 0.9rem;
    color: #e94560;
    margin-bottom: 1rem;
  }

  .graph-canvas {
    background: #1a1a2e;
    border-radius: 8px;
    cursor: pointer;
  }

  .hint {
    font-size: 0.75rem;
    color: #666;
    margin-top: 0.5rem;
  }
</style>

<script>
  const sections = [
    {
      title: 'Getting Started',
      items: [
        { title: 'Introduction', path: '/doc/intro' },
        { title: 'Quick Start', path: '/doc/quickstart' },
      ]
    },
    {
      title: 'Core',
      items: [
        { title: 'Architecture', path: '/doc/architecture' },
        { title: 'SRC Reference', path: '/doc/src' },
      ]
    },
    {
      title: 'Administration',
      items: [
        { title: 'Project Admin', path: '/doc/project-admin' },
        { title: 'Features', path: '/doc/features' },
      ]
    },
    {
      title: 'Agents',
      items: [
        { title: 'Agent Index', path: '/doc/agent-index' },
        { title: 'CLI Config', path: '/doc/cli-config' },
      ]
    }
  ];
</script>

<nav class="sidebar">
  {#each sections as section}
    <div class="section">
      <h3 class="section-title">{section.title}</h3>
      <ul class="nav-list">
        {#each section.items as item}
          <li>
            <a href={item.path}>{item.title}</a>
          </li>
        {/each}
      </ul>
    </div>
  {/each}
</nav>

<style>
  .sidebar {
    font-size: 0.9rem;
  }

  .section {
    margin-bottom: 1.5rem;
  }

  .section-title {
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: #e94560;
    margin-bottom: 0.5rem;
  }

  .nav-list {
    list-style: none;
  }

  .nav-list li {
    margin-bottom: 0.25rem;
  }

  .nav-list a {
    color: #aaa;
    text-decoration: none;
    display: block;
    padding: 0.4rem 0.5rem;
    border-radius: 4px;
    transition: background 0.2s;
  }

  .nav-list a:hover {
    color: #fff;
    background: #0f3460;
  }
</style>

<script>
  let searchQuery = '';
  let results = [];

  const docs = [
    { title: 'SRC - Source Code Reference', path: '/doc/src', tags: ['#documentation', '#source'] },
    { title: 'Architecture', path: '/doc/architecture', tags: ['#architecture', '#core'] },
    { title: 'Project Admin', path: '/doc/project-admin', tags: ['#admin', '#projects'] },
    { title: 'Agent Index', path: '/doc/agent-index', tags: ['#agents', '#skills'] },
    { title: 'Features', path: '/doc/features', tags: ['#features', '#tracking'] },
  ];

  function handleSearch() {
    if (!searchQuery.trim()) {
      results = [];
      return;
    }
    const q = searchQuery.toLowerCase();
    results = docs.filter(d =>
      d.title.toLowerCase().includes(q) ||
      d.tags.some(t => t.toLowerCase().includes(q))
    );
  }
</script>

<div class="search-container">
  <input
    type="text"
    bind:value={searchQuery}
    on:input={handleSearch}
    placeholder="Search docs..."
    class="search-input"
  />
  {#if results.length > 0}
    <ul class="results">
      {#each results as result}
        <li>
          <a href={result.path}>{result.title}</a>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .search-container {
    margin-bottom: 1.5rem;
  }

  .search-input {
    width: 100%;
    padding: 0.75rem 1rem;
    border: 1px solid #0f3460;
    border-radius: 8px;
    background: #1a1a2e;
    color: #eee;
    font-size: 0.9rem;
  }

  .search-input:focus {
    outline: none;
    border-color: #e94560;
  }

  .results {
    list-style: none;
    margin-top: 0.5rem;
    background: #1a1a2e;
    border-radius: 8px;
    overflow: hidden;
  }

  .results li a {
    display: block;
    padding: 0.75rem 1rem;
    color: #eee;
    text-decoration: none;
    border-bottom: 1px solid #0f3460;
  }

  .results li a:hover {
    background: #0f3460;
  }
</style>

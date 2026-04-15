/**
 * Xavier2 Client - Simple REST client for Xavier2 memory system
 *
 * Usage:
 * const xavier2 = new Xavier2Client('http://localhost:8003', 'dev-token');
 * await xavier2.add({ content: '...', metadata: {...} });
 * const results = await xavier2.search({ query: '...' });
 */

class Xavier2Client {
  constructor(baseUrl = 'http://localhost:8003', token = 'dev-token') {
    this.baseUrl = baseUrl;
    this.token = token;
  }

  async _request(method, endpoint, body = null) {
    const options = {
      method,
      headers: {
        'Content-Type': 'application/json',
        'X-Xavier2-Token': this.token
      }
    };
    if (body) options.body = JSON.stringify(body);

    const response = await fetch(`${this.baseUrl}${endpoint}`, options);
    if (!response.ok) {
      const error = await response.text();
      throw new Error(`Xavier2 API error ${response.status}: ${error}`);
    }
    return response.json();
  }

  /** Health check */
  async health() {
    return this._request('GET', '/health');
  }

  /** Search memories */
  async search({ query, limit = 5, workspace_id = 'default' }) {
    return this._request('POST', '/memory/search', { query, limit, workspace_id });
  }

  /** Add memory */
  async add({ content, metadata = {} }) {
    return this._request('POST', '/memory/add', { content, metadata });
  }

  /** Get memory stats */
  async stats() {
    return this._request('GET', '/memory/stats');
  }

  /** Apply decay to old memories */
  async decay(dryRun = false) {
    return this._request('POST', '/memory/decay', { dry_run: dryRun });
  }

  /** Consolidate/merge duplicates */
  async consolidate(dryRun = false) {
    return this._request('POST', '/memory/consolidate', { dry_run: dryRun });
  }

  /** Evict low quality memories */
  async evict({ threshold = 0.2, dryRun = false }) {
    return this._request('DELETE', `/memory/evict?threshold=${threshold}&dry_run=${dryRun}`);
  }

  /** Get low quality memories */
  async quality(threshold = 0.3) {
    return this._request('GET', `/memory/quality?threshold=${threshold}`);
  }
}

module.exports = { Xavier2Client };

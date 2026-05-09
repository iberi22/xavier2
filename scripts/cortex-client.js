/**
 * Xavier Client - Simple REST client for Xavier memory system
 *
 * Usage:
 * const xavier = new XavierClient('http://localhost:8003', process.env.XAVIER_TOKEN);
 * await xavier.add({ content: '...', metadata: {...} });
 * const results = await xavier.search({ query: '...' });
 */

function getRequiredXavierToken() {
  const token = process.env.XAVIER_TOKEN || process.env.XAVIER_API_KEY || process.env.XAVIER_TOKEN;
  if (!token) {
    throw new Error('Missing Xavier token. Set XAVIER_TOKEN, XAVIER_API_KEY, or XAVIER_TOKEN.');
  }
  return token;
}

class XavierClient {
  constructor(baseUrl = 'http://localhost:8003', token = getRequiredXavierToken()) {
    this.baseUrl = baseUrl;
    this.token = token;
  }

  async _request(method, endpoint, body = null) {
    const options = {
      method,
      headers: {
        'Content-Type': 'application/json',
        'X-Xavier-Token': this.token
      }
    };
    if (body) options.body = JSON.stringify(body);

    const response = await fetch(`${this.baseUrl}${endpoint}`, options);
    if (!response.ok) {
      const error = await response.text();
      throw new Error(`Xavier API error ${response.status}: ${error}`);
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

module.exports = { XavierClient };

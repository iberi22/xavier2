/**
 * Vault Reader - Reads markdown files from vault folder
 * Supports frontmatter, internal links, and graph relationships
 */

import fs from 'fs';
import path from 'path';
import matter from 'gray-matter';

export interface DocMetadata {
  title: string;
  type?: string;
  tags?: string[];
  related?: string[];
  sources?: string[];
  version?: string;
  last_updated?: string;
}

export interface Doc {
  slug: string;
  path: string;
  metadata: DocMetadata;
  content: string;
  links: string[];  // Internal links [[doc-name]]
}

export interface GraphNode {
  id: string;
  label: string;
  type: string;
  tags: string[];
}

export interface GraphEdge {
  from: string;
  to: string;
}

/**
 * Read all markdown files from vault directory
 */
export function readVault(vaultPath: string): Doc[] {
  const docs: Doc[] = [];
  
  function walkDir(dir: string) {
    const files = fs.readdirSync(dir);
    
    for (const file of files) {
      const fullPath = path.join(dir, file);
      const stat = fs.statSync(fullPath);
      
      if (stat.isDirectory()) {
        walkDir(fullPath);
      } else if (file.endsWith('.md') || file.endsWith('.markdown')) {
        const doc = readDoc(fullPath, vaultPath);
        if (doc) docs.push(doc);
      }
    }
  }
  
  if (fs.existsSync(vaultPath)) {
    walkDir(vaultPath);
  }
  
  return docs;
}

/**
 * Read a single document
 */
function readDoc(filePath: string, basePath: string): Doc | null {
  try {
    const content = fs.readFileSync(filePath, 'utf-8');
    const { data, content: body } = matter(content);
    
    const slug = path.relative(basePath, filePath)
      .replace(/\.md$/, '')
      .replace(/\\/g, '/');
    
    // Extract internal links [[doc-name]]
    const links = extractInternalLinks(body);
    
    return {
      slug,
      path: filePath,
      metadata: {
        title: data.title || slug,
        type: data.type,
        tags: data.tags || [],
        related: data.related || [],
        sources: data.sources || [],
        version: data.version,
        last_updated: data.last_updated,
      },
      content: body,
      links,
    };
  } catch (error) {
    console.error(`Error reading ${filePath}:`, error);
    return null;
  }
}

/**
 * Extract [[internal links]] from content
 */
function extractInternalLinks(content: string): string[] {
  const regex = /\[\[([^\]|]+)(?:\|[^\]]+)?\]\]/g;
  const links: string[] = [];
  let match;
  
  while ((match = regex.exec(content)) !== null) {
    links.push(match[1]);
  }
  
  return links;
}

/**
 * Build graph from documents
 */
export function buildGraph(docs: Doc[]): { nodes: GraphNode[]; edges: GraphEdge[] } {
  const nodes: GraphNode[] = docs.map(doc => ({
    id: doc.slug,
    label: doc.metadata.title,
    type: doc.metadata.type || 'doc',
    tags: doc.metadata.tags || [],
  }));
  
  const edges: GraphEdge[] = [];
  
  // Add edges from internal links
  docs.forEach(doc => {
    doc.links.forEach(link => {
      // Find target doc
      const target = docs.find(d => 
        d.slug === link || 
        d.metadata.title.toLowerCase() === link.toLowerCase()
      );
      
      if (target) {
        edges.push({ from: doc.slug, to: target.slug });
      }
    });
  });
  
  // Add edges from related field
  docs.forEach(doc => {
    (doc.metadata.related || []).forEach(rel => {
      const target = docs.find(d => 
        d.slug === rel || 
        d.metadata.title.toLowerCase() === rel.toLowerCase()
      );
      
      if (target) {
        edges.push({ from: doc.slug, to: target.slug });
      }
    });
  });
  
  return { nodes, edges };
}

/**
 * Search documents
 */
export function searchDocs(docs: Doc[], query: string): Doc[] {
  const q = query.toLowerCase();
  
  return docs.filter(doc => 
    doc.metadata.title.toLowerCase().includes(q) ||
    doc.content.toLowerCase().includes(q) ||
    (doc.metadata.tags || []).some(t => t.toLowerCase().includes(q))
  );
}

export default {
  readVault,
  buildGraph,
  searchDocs,
};

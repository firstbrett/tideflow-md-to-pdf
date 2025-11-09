import type { DocumentKind } from '../types';

const markdownExtensions = new Set(['md', 'markdown', 'qmd']);
const latexExtensions = new Set(['tex', 'latex']);

function extensionOf(path: string | null | undefined): string {
  if (!path) return '';
  const match = path.split('.').pop();
  return match ? match.toLowerCase() : '';
}

export function detectDocumentKind(filePath: string | null): DocumentKind {
  const ext = extensionOf(filePath);
  if (latexExtensions.has(ext)) {
    return 'latex';
  }
  return 'markdown';
}

export function isMarkdownFile(path: string | null | undefined): boolean {
  return markdownExtensions.has(extensionOf(path));
}

export function isLatexFile(path: string | null | undefined): boolean {
  return latexExtensions.has(extensionOf(path));
}

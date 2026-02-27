/**
 * Format a byte count as a human-readable string (e.g. "1.2 MB").
 */
export function formatBytes(bytes: number): string {
  if (!isFinite(bytes) || bytes < 0) return '0 B';
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

/**
 * Extract the hostname from a URL string for display purposes.
 * Returns the raw string on parse failure.
 */
export function urlDomain(url: string): string {
  try {
    return new URL(url).hostname;
  } catch {
    return url;
  }
}

export const formatFileSize = (bytes?: number): string => {
  if (!bytes) return 'Unknown size'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let size = bytes
  let unitIndex = 0
  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024
    unitIndex++
  }
  return `${size.toFixed(2)} ${units[unitIndex]}`
}

// Extract filename from path, or return as-is if not a path
// Determine if a string looks like a local file path
export const isLocalPath = (str: string): boolean => {
  if (!str) return false
  // Check for absolute paths or typical relative path starts
  // On Windows checking for \ or Drive: is safer
  return (
    str.startsWith('/') ||
    str.startsWith('./') ||
    str.startsWith('../') ||
    str.includes('\\') ||
    /^[a-zA-Z]:\\/.test(str)
  )
}

// Extract filename from path for display, but keep HF IDs (user/repo) as is
export const getDisplayValue = (pathOrName: string): string => {
  if (!pathOrName) return ''
  // Only strip path if it actually looks like a filesystem path
  if (isLocalPath(pathOrName)) {
    // Extract just the filename
    const parts = pathOrName.split(/[/\\]/)
    return parts[parts.length - 1] || pathOrName
  }
  return pathOrName
}

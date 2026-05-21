/**
 * Heuristics for mapping Notion titles and properties to GKS IDs and types.
 */

/**
 * Converts a Notion title into a screaming-kebab-case slug.
 */
export function slugify(text: string): string {
  return text
    .toUpperCase()
    .trim()
    .replace(/[^\w\s-]/g, '')
    .replace(/[\s_]+/g, '-')
}

/**
 * Infers the GKS atom type from Notion database properties.
 */
export function inferGksType(properties: Record<string, any>): string {
  // Check for an explicit "GKS_TYPE" property
  const explicit = properties['GKS_TYPE']?.select?.name || properties['Type']?.select?.name
  if (explicit) return explicit.toUpperCase()

  // Heuristic based on status or other keywords
  const title = properties['Name']?.title?.[0]?.plain_text || ''
  if (title.includes('ADR') || title.includes('Decision')) return 'ADR'
  if (title.includes('Spec')) return 'SPEC'
  if (title.includes('Plan') || title.includes('Blueprint')) return 'BLUEPRINT'

  return 'CONCEPT' // Default fallback
}

/**
 * Generates a full GKS ID.
 */
export function generateGksId(type: string, title: string): string {
  const slug = slugify(title)
  return `${type.toUpperCase()}--${slug}`
}

import { Client } from '@notionhq/client'
import { NotionToMarkdown } from 'notion-to-md'

export interface NotionClientOptions {
  token: string
}

/**
 * Technical wrapper for the Notion API.
 */
export class MspNotionClient {
  private client: Client
  private n2m: NotionToMarkdown

  constructor(opts: NotionClientOptions) {
    this.client = new Client({ auth: opts.token })
    this.n2m = new NotionToMarkdown({ notionClient: this.client })
  }

  /**
   * Fetches all pages from a specific database.
   */
  async fetchDatabasePages(databaseId: string) {
    const response = await (this.client.databases as any).query({
      database_id: databaseId,
    })
    return response.results
  }

  /**
   * Fetches and converts a Notion page to Markdown.
   */
  async pageToMarkdown(pageId: string): Promise<string> {
    const mdblocks = await this.n2m.pageToMarkdown(pageId)
    const mdString = this.n2m.toMarkdownString(mdblocks)
    return mdString.parent
  }

  /**
   * Extracts properties from a Notion page.
   */
  async getPageProperties(pageId: string): Promise<Record<string, any>> {
    const page = await this.client.pages.retrieve({ page_id: pageId }) as any
    return page.properties || {}
  }
}

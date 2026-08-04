import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest'
import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
import {
  clearToolsCache,
  fetchAvailableTools,
  getToolIcon,
  getToolIconFromMetadata,
  getToolIconWithMetadata
} from './toolIcons.ts'
import type { ToolInfo } from '@types'

vi.mock('@axios/axiosBackendInstance.ts', () => ({
  axiosBackendInstance: {
    get: vi.fn()
  }
}))

const mockedAxios = axiosBackendInstance as unknown as {
  get: ReturnType<typeof vi.fn>
}

const tool = (over: Partial<ToolInfo> & { name: string }): ToolInfo => ({
  id: over.name,
  tool_type: 'utility',
  description: '',
  category: 'utility',
  icon: '',
  ...over
})

beforeEach(() => {
  clearToolsCache()
  mockedAxios.get.mockReset()
  vi.spyOn(console, 'error').mockImplementation(() => {})
})

afterEach(() => {
  vi.restoreAllMocks()
})

describe('fetchAvailableTools', () => {
  test('fetches the tool list from agent/tools', async () => {
    const tools = [tool({ name: 'calculator', icon: 'calculator' })]
    mockedAxios.get.mockResolvedValue({ data: tools })

    await expect(fetchAvailableTools()).resolves.toEqual(tools)
    expect(mockedAxios.get).toHaveBeenCalledWith('agent/tools')
  })

  test('serves later calls from the cache without re-requesting', async () => {
    const tools = [tool({ name: 'calculator' })]
    mockedAxios.get.mockResolvedValue({ data: tools })

    const first = await fetchAvailableTools()
    const second = await fetchAvailableTools()

    expect(mockedAxios.get).toHaveBeenCalledTimes(1)
    expect(second).toBe(first)
  })

  test('de-duplicates concurrent calls onto a single request', async () => {
    let resolveGet: (_value: { data: ToolInfo[] }) => void = () => {}
    mockedAxios.get.mockReturnValue(
      new Promise((resolve) => {
        resolveGet = resolve
      })
    )

    const a = fetchAvailableTools()
    const b = fetchAvailableTools()
    resolveGet({ data: [tool({ name: 'calculator' })] })

    expect(await a).toEqual(await b)
    expect(mockedAxios.get).toHaveBeenCalledTimes(1)
  })

  test('returns an empty list and logs when the request fails', async () => {
    mockedAxios.get.mockRejectedValue(new Error('network down'))

    await expect(fetchAvailableTools()).resolves.toEqual([])
    expect(console.error).toHaveBeenCalledWith(
      'Failed to fetch available tools:',
      expect.any(Error)
    )
  })

  test('retries after a failure because nothing was cached', async () => {
    mockedAxios.get.mockRejectedValueOnce(new Error('network down'))
    expect(await fetchAvailableTools()).toEqual([])

    const tools = [tool({ name: 'calculator' })]
    mockedAxios.get.mockResolvedValueOnce({ data: tools })
    expect(await fetchAvailableTools()).toEqual(tools)
    expect(mockedAxios.get).toHaveBeenCalledTimes(2)
  })

  test('clearToolsCache forces a new request', async () => {
    mockedAxios.get.mockResolvedValue({ data: [tool({ name: 'a' })] })
    await fetchAvailableTools()
    clearToolsCache()
    await fetchAvailableTools()
    expect(mockedAxios.get).toHaveBeenCalledTimes(2)
  })
})

describe('getToolIconFromMetadata', () => {
  test('returns the wrench for a missing tool name without hitting the backend', async () => {
    await expect(getToolIconFromMetadata(undefined)).resolves.toBe('wrench')
    await expect(getToolIconFromMetadata('')).resolves.toBe('wrench')
    expect(mockedAxios.get).not.toHaveBeenCalled()
  })

  test('uses the icon of the tool whose name matches exactly, case-insensitively', async () => {
    mockedAxios.get.mockResolvedValue({
      data: [
        tool({ name: 'other', tool_type: 'zzz', icon: 'nope' }),
        tool({ name: 'calculator', tool_type: 'zzz', icon: 'calculator' })
      ]
    })
    await expect(getToolIconFromMetadata('Calculator')).resolves.toBe(
      'calculator'
    )
  })

  test('matches on the tool type with the underscore turned into a space', async () => {
    mockedAxios.get.mockResolvedValue({
      data: [tool({ name: 'brave', tool_type: 'web_search', icon: 'magnify' })]
    })
    await expect(getToolIconFromMetadata('web search')).resolves.toBe('magnify')
  })

  test('matches when the requested name contains the tool name', async () => {
    mockedAxios.get.mockResolvedValue({
      data: [tool({ name: 'weather', tool_type: 'utility', icon: 'weather' })]
    })
    await expect(getToolIconFromMetadata('get_weather_forecast')).resolves.toBe(
      'weather'
    )
  })

  test('matches when the requested name contains the compacted tool type', async () => {
    mockedAxios.get.mockResolvedValue({
      data: [tool({ name: 'zzz', tool_type: 'file_read', icon: 'file-eye' })]
    })
    await expect(getToolIconFromMetadata('do_fileread_now')).resolves.toBe(
      'file-eye'
    )
  })

  test('falls back to pattern matching when the matched tool has no icon', async () => {
    mockedAxios.get.mockResolvedValue({
      data: [tool({ name: 'chromadb_query', tool_type: 'zzz', icon: '' })]
    })
    await expect(getToolIconFromMetadata('chromadb_query')).resolves.toBe(
      'database'
    )
  })

  test('falls back to pattern matching when no tool matches', async () => {
    mockedAxios.get.mockResolvedValue({
      data: [tool({ name: 'zzz', tool_type: 'yyy', icon: 'nope' })]
    })
    await expect(getToolIconFromMetadata('send_email')).resolves.toBe('send')
  })

  test('falls back to pattern matching and logs when the metadata lookup throws', async () => {
    mockedAxios.get.mockImplementation(() => {
      throw new Error('client blew up')
    })

    await expect(getToolIconFromMetadata('delete_row')).resolves.toBe('delete')
    expect(console.error).toHaveBeenCalledWith(
      '⚠️ Could not fetch tool metadata, using pattern matching:',
      expect.any(Error)
    )
  })

  test('getToolIconWithMetadata is the same implementation', async () => {
    expect(getToolIconWithMetadata).toBe(getToolIconFromMetadata)
    mockedAxios.get.mockResolvedValue({
      data: [tool({ name: 'calculator', tool_type: 'zzz', icon: 'plus' })]
    })
    await expect(getToolIconWithMetadata('calculator')).resolves.toBe('plus')
  })
})

describe('getToolIcon', () => {
  test('returns the wrench for a missing name', () => {
    expect(getToolIcon(undefined)).toBe('wrench')
    expect(getToolIcon('')).toBe('wrench')
  })

  test.each([
    ['check_website_status', 'web'],
    ['website_check', 'web'],
    ['WEBSITE', 'web'],
    ['financial_report', 'currency-usd'],
    ['sql_query', 'currency-usd'],
    ['crypto_price', 'bitcoin'],
    ['bitcoin_price', 'bitcoin'],
    ['chromadb_add_documents', 'database'],
    ['chroma_list', 'database'],
    ['database_dump', 'database'],
    ['pageindex_lookup', 'file-tree'],
    ['search_docs', 'magnify'],
    ['query_collection', 'magnify'],
    ['read_document', 'file-document'],
    ['list_files', 'file-document'],
    ['write_report', 'file-edit'],
    ['save_note', 'file-edit'],
    ['delete_row', 'delete'],
    ['remove_item', 'delete'],
    ['update_record', 'pencil'],
    ['edit_row', 'pencil'],
    ['create_ticket', 'plus-circle'],
    ['add_row', 'plus-circle'],
    ['new_branch', 'plus-circle'],
    ['send_email', 'send'],
    ['post_message', 'send'],
    ['get_user', 'download'],
    ['fetch_price', 'download'],
    ['retrieve_doc', 'download'],
    ['book_flight', 'book-open-variant'],
    ['calculator', 'wrench']
  ])('maps %s to the %s icon', (name, icon) => {
    expect(getToolIcon(name)).toBe(icon)
  })

  test('earlier patterns win over later ones', () => {
    // 'sql_query' contains 'query' but the financial rule is checked first
    expect(getToolIcon('sql_query')).toBe('currency-usd')
    // 'chromadb_query' contains 'query' but the database rule is checked first
    expect(getToolIcon('chromadb_query')).toBe('database')
    // 'get_file' contains 'get' but the file rule is checked first
    expect(getToolIcon('get_file')).toBe('file-document')
  })
})

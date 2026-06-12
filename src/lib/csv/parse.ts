import Papa from 'papaparse'

export interface ParsedCsv {
  headers: string[]
  rows: Record<string, string>[]
}

/** Parse CSV text into headers + objects keyed by header. */
export function parseCsv(text: string): ParsedCsv {
  const result = Papa.parse<Record<string, string>>(text.trim(), {
    header: true,
    skipEmptyLines: true,
  })
  const headers = result.meta.fields ?? []
  return { headers, rows: result.data }
}

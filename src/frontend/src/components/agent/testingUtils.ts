import { read, utils } from 'xlsx'
import { z } from 'zod'

export const parseQuestionsFromFile = (file: File): Promise<string[]> => {
  return new Promise((resolve, reject) => {
    const reader = new FileReader()

    reader.onload = (e) => {
      try {
        const data = new Uint8Array(e.target?.result as ArrayBuffer)
        const workbook = read(data, { type: 'array' })
        const sheetName = workbook.SheetNames[0]
        const worksheet = workbook.Sheets[sheetName]
        const jsonData = utils.sheet_to_json(worksheet)

        // Validate generic structure (array of objects)
        // Validate generic structure (array of objects)
        const SheetSchema = z.array(z.any())

        const parsedData = SheetSchema.parse(jsonData)

        const questions: string[] = []

        for (const row of parsedData) {
          // Look for 'questions', 'measure', 'text', 'content' (case insensitive)
          const keys = Object.keys(row)
          const targetKey = keys.find(k => 
            ['questions', 'question', 'text', 'content'].includes(k.toLowerCase())
          )

          if (targetKey) {
            const val = row[targetKey]
            if (typeof val === 'string' && val.trim()) {
              questions.push(val.trim())
            } else if (typeof val === 'number') {
               questions.push(val.toString())
            }
          }
        }

        if (questions.length === 0) {
            // If no header matched, maybe it's a headless CSV/Sheet?
            reject(new Error('No valid questions found. Ensure column header is "questions", "question", or "text".'))
            return
        }

        resolve(questions)
      } catch (err) {
        reject(err)
      }
    }

    reader.onerror = (err) => reject(err)
    reader.readAsArrayBuffer(file)
  })
}

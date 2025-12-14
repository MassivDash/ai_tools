import { z } from 'zod'

// PDF file validation - accepts File object
export const PdfToMarkdownRequestSchema = z.object({
  file: z.instanceof(File, { message: 'Please select a PDF file' }).refine(
    (file) => file.type === 'application/pdf' || file.name.endsWith('.pdf'),
    { message: 'File must be a PDF' }
  )
})


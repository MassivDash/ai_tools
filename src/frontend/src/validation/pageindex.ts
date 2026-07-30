import { z } from 'zod'

// File type validation - PDF only
const SUPPORTED_FILE_TYPES = ['application/pdf'] as const

const SUPPORTED_EXTENSIONS = ['.pdf'] as const

export const validatePdfFileType = (file: File): boolean => {
  // Check MIME type
  if (SUPPORTED_FILE_TYPES.includes(file.type as any)) {
    return true
  }
  // Check file extension as fallback
  const fileName = file.name.toLowerCase()
  return SUPPORTED_EXTENSIONS.some((ext) => fileName.endsWith(ext))
}

export const PdfFileValidationSchema = z
  .instanceof(File)
  .refine(validatePdfFileType, {
    message: 'File type not supported. Only PDF files are allowed.'
  })

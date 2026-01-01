import { z } from 'zod';

export const SDConfigSchema = z.object({
  // Standard
  diffusion_model: z.string().min(1, "Diffusion model is required"),
  vae: z.string().optional(), // Can be empty
  llm: z.string().optional(),
  models_path: z.string().min(1, "Models path is required"),
  output_path: z.string().min(1, "Output path is required"),
  
  height: z.number().int().min(64).max(4096).default(1024),
  width: z.number().int().min(64).max(4096).default(1024),
  steps: z.number().int().min(1).max(150).nullable().optional(),
  cfg_scale: z.number().min(1.0).max(30.0).default(7.0),
  
  // Advanced Generation
  seed: z.number().int().nullable().optional(), // -1 or null usually means random, backend handles it
  batch_count: z.number().int().min(1).max(100).nullable().optional(),
  guidance: z.number().nullable().optional(),
  strength: z.number().min(0.0).max(1.0).nullable().optional(),
  sampling_method: z.string().nullable().optional(),
  scheduler: z.string().nullable().optional(),
  
  // Flags
  diffusion_fa: z.boolean().default(false),
  verbose: z.boolean().default(true),
  color: z.boolean().default(true),
  offload_to_cpu: z.boolean().default(false),
  
  // Advanced Models
  clip_l: z.string().nullable().optional(),
  clip_g: z.string().nullable().optional(),
  t5xxl: z.string().nullable().optional(),
  control_net: z.string().nullable().optional(),
  lora_model_dir: z.string().nullable().optional(),
  
  // System
  threads: z.number().int().min(-1).default(-1),
  rng: z.string().nullable().optional(),
});

export const GenerationSchema = z.object({
  prompt: z.string().min(1, "Prompt cannot be empty"),
  negative_prompt: z.string().optional(),
});

export type SDConfig = z.infer<typeof SDConfigSchema>;
export type GenerationRequest = z.infer<typeof GenerationSchema>;

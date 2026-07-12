/**
 * Tailwind class merger. clsx for conditional joining, tailwind-merge for dedup
 * of conflicting utility classes (e.g. "p-2 p-4" → "p-4").
 */

import { clsx, type ClassValue } from 'clsx';
import { twMerge } from 'tailwind-merge';

export function cn(...inputs: ClassValue[]): string {
  return twMerge(clsx(inputs));
}
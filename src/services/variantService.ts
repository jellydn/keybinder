/**
 * Variant detection service for skhd
 *
 * This service provides:
 * - Detecting which skhd variant is installed (original or skhd.zig)
 * - Getting variant metadata (binary path, plist label, detection source)
 */

import { invoke } from '@tauri-apps/api/core';
import type { DetectedVariant } from '../types';

/**
 * Detect which skhd variant is installed and active
 *
 * Detection order:
 * 1. Check running launchd jobs (com.jackielii.skhd first, then com.koekeishiya.skhd)
 * 2. Check Homebrew formulae (brew list skhd-zig, then brew list skhd)
 * 3. Check PATH for skhd binary and fingerprint version output
 * 4. Check .app bundle at /Applications/skhd.app/Contents/MacOS/skhd
 * 5. Return none if not detected
 *
 * @returns {Promise<DetectedVariant>} Detection result with variant info
 *
 * @example
 * ```typescript
 * const detected = await detectSkhdVariant();
 * if (detected.variant === 'zig') {
 *   console.log('skhd.zig detected at:', detected.binary_path);
 * } else if (detected.variant === 'original') {
 *   console.log('Original skhd detected');
 * } else {
 *   console.log('No skhd variant found');
 * }
 * ```
 */
export async function detectSkhdVariant(): Promise<DetectedVariant> {
  return invoke('detect_skhd_variant');
}

/**
 * Check if a skhd variant is detected
 *
 * @param detected - The detection result
 * @returns {boolean} True if a variant was detected
 *
 * @example
 * ```typescript
 * const detected = await detectSkhdVariant();
 * if (isVariantDetected(detected)) {
 *   console.log('Found:', detected.variant);
 * }
 * ```
 */
export function isVariantDetected(detected: DetectedVariant): boolean {
  return detected.variant !== null;
}

/**
 * Get a human-readable description of the detected variant
 *
 * @param detected - The detection result
 * @returns {string} Human-readable description
 *
 * @example
 * ```typescript
 * const detected = await detectSkhdVariant();
 * console.log(getVariantDescription(detected)); // "skhd.zig" or "skhd (original)" or "Not detected"
 * ```
 */
export function getVariantDescription(detected: DetectedVariant): string {
  switch (detected.variant) {
    case 'original':
      return 'skhd (original)';
    case 'zig':
      return 'skhd.zig';
    default:
      return 'Not detected';
  }
}

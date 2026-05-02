/**
 * Settings service for managing app preferences
 *
 * This service provides:
 * - Getting and setting the skhd variant preference
 * - Computing the effective variant based on settings and detection
 * - Loading and saving app settings
 */

import { invoke } from "@tauri-apps/api/core";
import type {
	EffectiveVariantResponse,
	Settings,
	SkhdVariantSetting,
} from "../types";

/**
 * Get the current skhd variant setting
 *
 * Returns the user's preference: 'auto', 'original', or 'zig'
 * Default is 'auto' which automatically detects the installed variant
 *
 * @returns {Promise<SkhdVariantSetting>} The current variant setting
 *
 * @example
 * ```typescript
 * const setting = await getSkhdVariantSetting();
 * console.log(setting); // "auto" | "original" | "zig"
 * ```
 */
export async function getSkhdVariantSetting(): Promise<SkhdVariantSetting> {
	return invoke("get_skhd_variant_setting");
}

/**
 * Set the skhd variant preference
 *
 * @param {SkhdVariantSetting} value - One of: 'auto', 'original', 'zig'
 * @returns {Promise<void>}
 *
 * @example
 * ```typescript
 * // Let the app auto-detect
 * await setSkhdVariantSetting('auto');
 *
 * // Force use of original skhd
 * await setSkhdVariantSetting('original');
 *
 * // Force use of skhd.zig
 * await setSkhdVariantSetting('zig');
 * ```
 */
export async function setSkhdVariantSetting(
	value: SkhdVariantSetting,
): Promise<void> {
	return invoke("set_skhd_variant_setting", { value });
}

/**
 * Get the effective variant based on settings and detection
 *
 * This computes the actual variant to use:
 * - If setting is 'auto': detects installed variant
 * - If setting is a specific variant: returns that variant with a warning if not installed
 *
 * @returns {Promise<EffectiveVariantResponse>} The effective variant with metadata
 *
 * @example
 * ```typescript
 * const effective = await getEffectiveVariant();
 * console.log(effective.variant); // "original" | "zig"
 * console.log(effective.is_auto_detected); // true | false
 * if (effective.warning) {
 *   console.warn(effective.warning); // e.g., "skhd.zig is selected but not detected"
 * }
 * ```
 */
export async function getEffectiveVariant(): Promise<EffectiveVariantResponse> {
	return invoke("get_effective_variant");
}

/**
 * Get all app settings
 *
 * @returns {Promise<Settings>} The complete settings object
 *
 * @example
 * ```typescript
 * const settings = await getSettings();
 * console.log(settings.skhd_variant); // "auto" | "original" | "zig"
 * ```
 */
export async function getSettings(): Promise<Settings> {
	return invoke("get_settings");
}

/**
 * Check if the effective variant was auto-detected
 *
 * Helper function for UI components that want to show detection status
 *
 * @param {EffectiveVariantResponse} effective - The effective variant response
 * @returns {boolean} True if the variant was auto-detected
 *
 * @example
 * ```typescript
 * const effective = await getEffectiveVariant();
 * if (isAutoDetected(effective)) {
 *   console.log('Auto-detected:', effective.variant);
 * }
 * ```
 */
export function isAutoDetected(effective: EffectiveVariantResponse): boolean {
	return effective.is_auto_detected;
}

/**
 * Get a warning message if the chosen variant is not installed
 *
 * Helper function for UI components that want to show install guidance
 *
 * @param {EffectiveVariantResponse} effective - The effective variant response
 * @returns {string | null} Warning message or null if no warning
 *
 * @example
 * ```typescript
 * const effective = await getEffectiveVariant();
 * const warning = getVariantWarning(effective);
 * if (warning) {
 *   showInstallDialog(warning);
 * }
 * ```
 */
export function getVariantWarning(
	effective: EffectiveVariantResponse,
): string | null {
	return effective.warning;
}

/**
 * Check if the chosen variant setting matches what's actually installed
 *
 * Useful for showing "change setting" prompts in the UI
 *
 * @param {EffectiveVariantResponse} effective - The effective variant response
 * @returns {boolean} True if the setting matches installed variant
 *
 * @example
 * ```typescript
 * const effective = await getEffectiveVariant();
 * if (!isVariantInstalled(effective)) {
 *   promptToInstall(effective.variant);
 * }
 * ```
 */
export function isVariantInstalled(
	effective: EffectiveVariantResponse,
): boolean {
	return effective.warning === null;
}

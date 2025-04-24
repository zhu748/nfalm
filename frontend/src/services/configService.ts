// frontend/src/services/configService.ts
import { getConfig, saveConfig } from "../api";
import { ConfigData } from "../types/config.types";

/**
 * Service for managing configuration operations
 */
export const configService = {
  /**
   * Fetches config data from the API
   */
  async fetchConfig(): Promise<ConfigData> {
    const data = await getConfig();
    return data;
  },

  /**
   * Saves config data to the API
   * @param configData - The configuration data to save
   */
  async saveConfig(configData: ConfigData): Promise<Response> {
    const response = await saveConfig(configData);
    return response;
  },

  /**
   * Handle form field changes for config
   * @param config - Current config state
   * @param e - Form event
   */
  handleConfigChange(
    config: ConfigData,
    e: React.ChangeEvent<
      HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement
    >,
  ): ConfigData {
    const { name, value, type } = e.target;

    // Handle checkboxes
    if (type === "checkbox") {
      const checked = (e.target as HTMLInputElement).checked;
      return {
        ...config,
        [name]: checked,
      };
    }

    // Handle numbers
    if (type === "number") {
      return {
        ...config,
        [name]: value === "" ? 0 : Number(value),
      };
    }

    // Handle empty strings for nullable fields
    if (
      ["proxy", "rproxy", "custom_h", "custom_a", "padtxt_file"].includes(
        name,
      ) &&
      value === ""
    ) {
      return {
        ...config,
        [name]: null,
      };
    }

    // Handle regular text inputs
    return {
      ...config,
      [name]: value,
    };
  },

  /**
   * Check if password was changed
   * @param originalPassword - Original password
   * @param currentPassword - Current password
   */
  isPasswordChanged(
    originalPassword: string,
    currentPassword: string,
  ): boolean {
    return originalPassword !== currentPassword;
  },

  /**
   * Check if admin password was changed
   * @param originalAdminPassword - Original admin password
   * @param currentAdminPassword - Current admin password
   */
  isAdminPasswordChanged(
    originalAdminPassword: string,
    currentAdminPassword: string,
  ): boolean {
    return originalAdminPassword !== currentAdminPassword;
  },
};

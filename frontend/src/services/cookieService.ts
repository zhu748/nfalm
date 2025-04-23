import { getCookieStatus, deleteCookie } from "../api";
import { CookieStatusInfo } from "../types/cookie.types";

// Default empty state for cookie status
const emptyCookieStatus: CookieStatusInfo = {
  valid: [],
  dispatched: [],
  exhausted: [],
  invalid: [],
};

/**
 * Service for managing cookie operations
 */
export const cookieService = {
  /**
   * Fetches cookie status from the API
   */
  async fetchCookieStatus(): Promise<CookieStatusInfo> {
    const data = await getCookieStatus();
    return {
      valid: Array.isArray(data?.valid) ? data.valid : [],
      dispatched: Array.isArray(data?.dispatched) ? data.dispatched : [],
      exhausted: Array.isArray(data?.exhausted) ? data.exhausted : [],
      invalid: Array.isArray(data?.invalid) ? data.invalid : [],
    };
  },

  /**
   * Deletes a cookie
   * @param cookie - The cookie to delete
   */
  async deleteCookie(cookie: string): Promise<Response> {
    const response = await deleteCookie(cookie);
    if (!response.ok) {
      if (response.status === 401) {
        throw new Error("Authentication failed");
      }

      const errorData = await response.json().catch(() => ({}));
      throw new Error(errorData.error || `Error ${response.status}`);
    }
    return response;
  },

  /**
   * Parse and extract reason text from cookie reason object
   * @param reason - The reason object from the API
   * @param t - Translation function
   */
  getReasonText(reason: any, t: Function): string {
    if (!reason) return t("cookieStatus.status.reasons.unknown");
    if (typeof reason === "string") return reason;

    try {
      if ("NonPro" in reason)
        return t("cookieStatus.status.reasons.freAccount");
      if ("Disabled" in reason)
        return t("cookieStatus.status.reasons.disabled");
      if ("Banned" in reason) return t("cookieStatus.status.reasons.banned");
      if ("Null" in reason) return t("cookieStatus.status.reasons.invalid");
      if ("Restricted" in reason && typeof reason.Restricted === "number")
        return t("cookieStatus.status.reasons.restricted", {
          time: new Date(reason.Restricted * 1000).toLocaleString(),
        });
      if (
        "TooManyRequest" in reason &&
        typeof reason.TooManyRequest === "number"
      )
        return t("cookieStatus.status.reasons.rateLimited", {
          time: new Date(reason.TooManyRequest * 1000).toLocaleString(),
        });
    } catch (e) {
      console.error("Error parsing reason:", e, reason);
    }
    return t("cookieStatus.status.reasons.unknown");
  },

  // Empty cookie status state for initial rendering
  emptyCookieStatus,
};

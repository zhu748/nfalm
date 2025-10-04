// frontend/src/types/cookie.types.ts
export interface UsageBreakdown {
  total_input_tokens?: number;
  total_output_tokens?: number;
  sonnet_input_tokens?: number;
  sonnet_output_tokens?: number;
  opus_input_tokens?: number;
  opus_output_tokens?: number;
}

export interface CookieStatus {
  cookie: string;
  reset_time: number | null;
  supports_claude_1m?: boolean | null;
  count_tokens_allowed?: boolean | null;
  // New usage buckets
  session_usage?: UsageBreakdown;
  weekly_usage?: UsageBreakdown;
  weekly_opus_usage?: UsageBreakdown;
  lifetime_usage?: UsageBreakdown;
  // Ephemeral quota utilizations (percent), attached by /api/cookies only
  session_utilization?: number;
  seven_day_utilization?: number;
  seven_day_opus_utilization?: number;
  // Resets at timestamps (ISO8601), attached by /api/cookies only
  session_resets_at?: string | null;
  seven_day_resets_at?: string | null;
  seven_day_opus_resets_at?: string | null;
}

export interface UselessCookie {
  cookie: string;
  reason: unknown;
}

export interface CookieStatusInfo {
  valid: CookieStatus[];
  exhausted: CookieStatus[];
  invalid: UselessCookie[];
}

export type CookieItem = Partial<CookieStatus> & Pick<CookieStatus, "cookie"> & {
  reason?: unknown;
};

export interface CookieFormState {
  cookie: string;
  isSubmitting: boolean;
  status: {
    type: "idle" | "success" | "error";
    message: string;
  };
}

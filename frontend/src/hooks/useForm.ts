import { useState } from "react";

export function useForm<T>(initialValues: T) {
  const [values, setValues] = useState<T>(initialValues);
  const [errors, setErrors] = useState<Partial<Record<keyof T, string>>>({});
  const [isSubmitting, setIsSubmitting] = useState(false);

  const handleChange = (
    e: React.ChangeEvent<
      HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement
    >
  ) => {
    const { name, value, type } = e.target;

    if (type === "checkbox") {
      const checked = (e.target as HTMLInputElement).checked;
      setValues({
        ...values,
        [name]: checked,
      });
      return;
    }

    if (type === "number") {
      setValues({
        ...values,
        [name]: value === "" ? 0 : Number(value),
      });
      return;
    }

    setValues({
      ...values,
      [name]: value,
    });

    // Clear error when field is changed
    if (errors[name as keyof T]) {
      setErrors({
        ...errors,
        [name]: undefined,
      });
    }
  };

  const resetForm = () => {
    setValues(initialValues);
    setErrors({});
  };

  return {
    values,
    setValues,
    errors,
    setErrors,
    isSubmitting,
    setIsSubmitting,
    handleChange,
    resetForm,
  };
}

// frontend/src/types/config.types.ts
export interface ConfigData {
  // Server settings
  ip: string;
  port: number;
  enable_oai: boolean;

  // App settings
  check_update: boolean;
  auto_update: boolean;

  // Network settings
  password: string;
  proxy: string | null;
  rproxy: string | null;

  // API settings
  max_retries: number;
  pass_params: boolean;
  preserve_chats: boolean;

  // Cookie settings
  skip_warning: boolean;
  skip_restricted: boolean;
  skip_non_pro: boolean;

  // Prompt configurations
  use_real_roles: boolean;
  custom_h: string | null;
  custom_a: string | null;
  custom_prompt: string;
  padtxt_file: string | null;
  padtxt_len: number;
}

// frontend/src/types/cookie.types.ts
export interface CookieStatus {
  cookie: string;
  reset_time: number | null;
}

export interface UselessCookie {
  cookie: string;
  reason: string | any;
}

export interface CookieStatusInfo {
  valid: CookieStatus[];
  dispatched: [CookieStatus, number][];
  exhausted: CookieStatus[];
  invalid: UselessCookie[];
}

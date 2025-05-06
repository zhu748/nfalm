// frontend/src/types/key.types.ts
export interface KeyStatus {
  key: string;
}

export interface KeyStatusInfo {
  valid: KeyStatus[];
}

export interface KeyFormState {
  key: string;
  isSubmitting: boolean;
  status: {
    type: "idle" | "success" | "error";
    message: string;
  };
}

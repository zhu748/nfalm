import React, { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import Button from "../common/Button";
import FormInput from "../common/FormInput";
import StatusMessage from "../common/StatusMessage";
import { useAuth } from "../../hooks/useAuth";

interface AuthGatekeeperProps {
  onAuthenticated?: (status: boolean) => void;
}

const AuthGatekeeper: React.FC<AuthGatekeeperProps> = ({ onAuthenticated }) => {
  const { t } = useTranslation();
  const {
    authToken,
    setAuthToken,
    isLoading,
    error,
    savedToken,
    login,
    logout,
  } = useAuth(onAuthenticated);

  const [statusMessage, setStatusMessage] = useState({
    type: "idle" as "idle" | "success" | "error",
    message: "",
  });

  // Update status message when error changes
  useEffect(() => {
    if (error) {
      setStatusMessage({
        type: "error",
        message: error,
      });
    }
  }, [error]);

  // Clear status message after delay
  useEffect(() => {
    if (statusMessage.message) {
      const timer = setTimeout(() => {
        setStatusMessage({ type: "idle", message: "" });
      }, 3000);
      return () => clearTimeout(timer);
    }
  }, [statusMessage]);

  const handleSubmit = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();

    try {
      await login(authToken);
      setStatusMessage({
        type: "success",
        message: t("auth.success"),
      });
    } catch {
      // Error is already handled in the useAuth hook
    }
  };

  const handleClearToken = () => {
    logout();
    setStatusMessage({
      type: "success",
      message: t("auth.tokenCleared"),
    });
  };

  return (
    <div>
      <form onSubmit={handleSubmit} className="space-y-6">
        <FormInput
          id="authToken"
          name="authToken"
          type="password"
          value={authToken}
          onChange={(e) => setAuthToken(e.target.value)}
          label={t("auth.token")}
          placeholder={t("auth.tokenPlaceholder")}
          disabled={isLoading}
          onClear={() => setAuthToken("")}
        />

        {savedToken && (
          <div className="flex items-center justify-between mt-2">
            <p className="text-xs text-gray-400">
              {t("auth.previousToken")}{" "}
              <span className="font-mono">{savedToken}</span>
            </p>
            <button
              type="button"
              onClick={handleClearToken}
              className="text-xs text-red-400 hover:text-red-300"
              disabled={isLoading}
            >
              {t("auth.clear")}
            </button>
          </div>
        )}

        {statusMessage.message && (
          <StatusMessage
            type={statusMessage.type === "success" ? "success" : "error"}
            message={statusMessage.message}
          />
        )}

        <Button
          type="submit"
          isLoading={isLoading}
          disabled={isLoading}
          className="w-full"
          variant="primary"
        >
          {isLoading ? t("auth.verifying") : t("auth.submitButton")}
        </Button>
      </form>
    </div>
  );
};

export default AuthGatekeeper;

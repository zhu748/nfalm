// frontend/src/AuthGatekeeper.tsx
import React, { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";

interface AuthGatekeeperProps {
  onAuthenticated?: (status: boolean) => void;
}

const AuthGatekeeper: React.FC<AuthGatekeeperProps> = ({ onAuthenticated }) => {
  const { t } = useTranslation();
  const [authToken, setAuthToken] = useState("");
  const [status, setStatus] = useState({
    type: "idle",
    message: "",
  });
  const [isLoading, setIsLoading] = useState(false);
  const [savedToken, setSavedToken] = useState("");

  // Check for existing token on component mount and validate it
  useEffect(() => {
    const storedToken = localStorage.getItem("authToken");
    if (storedToken) {
      // Only show first few and last few characters for security
      const maskedToken = maskToken(storedToken);
      setSavedToken(maskedToken);
      validateToken(storedToken);
    }
  }, []);

  const maskToken = (token: string) => {
    if (token.length <= 8) return "••••••••";
    return (
      token.substring(0, 4) + "••••••••" + token.substring(token.length - 4)
    );
  };

  const validateToken = async (token: string) => {
    setIsLoading(true);
    try {
      const response = await fetch("/api/auth", {
        method: "GET",
        headers: {
          Authorization: `Bearer ${token}`,
          "Content-Type": "application/json",
        },
      });

      if (response.ok) {
        setStatus({
          type: "success",
          message: t("auth.success"),
        });
        // Call the onAuthenticated callback with true if it exists
        if (onAuthenticated) {
          onAuthenticated(true);
        }
      } else {
        localStorage.removeItem("authToken");
        setSavedToken("");
        setStatus({
          type: "error",
          message: t("auth.invalid"),
        });
        // Call the onAuthenticated callback with false if it exists
        if (onAuthenticated) {
          onAuthenticated(false);
        }
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : "Unknown error";
      setStatus({
        type: "error",
        message: t("auth.error", { message }),
      });
      // Call the onAuthenticated callback with false if it exists
      if (onAuthenticated) {
        onAuthenticated(false);
      }
    } finally {
      setIsLoading(false);
      // Clear status message after 3 seconds
      setTimeout(() => {
        setStatus({ type: "idle", message: "" });
      }, 3000);
    }
  };

  const handleSubmit = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();

    if (!authToken.trim()) {
      setStatus({
        type: "error",
        message: t("auth.enterToken"),
      });
      return;
    }

    try {
      // Validate the token with API
      await validateToken(authToken);

      // Save token to localStorage (validateToken will handle authentication state)
      localStorage.setItem("authToken", authToken);
      setSavedToken(maskToken(authToken));
      // Clear the input on completion
      setAuthToken("");
    } catch (error) {
      const message = error instanceof Error ? error.message : "Unknown error";
      setStatus({
        type: "error",
        message: t("auth.error", { message }),
      });
    }
  };

  const handleClearToken = () => {
    localStorage.removeItem("authToken");
    setSavedToken("");
    setStatus({
      type: "success",
      message: t("auth.success"),
    });

    // Call the onAuthenticated callback with false if it exists
    if (onAuthenticated) {
      onAuthenticated(false);
    }

    // Clear success message after 3 seconds
    setTimeout(() => {
      setStatus({ type: "idle", message: "" });
    }, 3000);
  };

  return (
    <div>
      <form onSubmit={handleSubmit} className="space-y-6">
        <div className="space-y-2">
          <label
            htmlFor="authToken"
            className="block text-sm font-medium text-gray-300 mb-1"
          >
            {t("auth.token")}
          </label>
          <div className="relative">
            <input
              id="authToken"
              type="password"
              value={authToken}
              onChange={(e) => setAuthToken(e.target.value)}
              placeholder={t("auth.tokenPlaceholder")}
              className="w-full p-4 bg-gray-700 border border-gray-600 rounded-md focus:ring-2 focus:ring-cyan-500 focus:border-cyan-500 text-sm text-gray-200 shadow-sm transition-all duration-200 placeholder-gray-400"
              disabled={isLoading}
            />
            {authToken && (
              <button
                type="button"
                className="absolute top-3 right-2 text-gray-400 hover:text-gray-200"
                onClick={() => setAuthToken("")}
                disabled={isLoading}
              >
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  className="h-5 w-5"
                  viewBox="0 0 20 20"
                  fill="currentColor"
                >
                  <path
                    fillRule="evenodd"
                    d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z"
                    clipRule="evenodd"
                  />
                </svg>
              </button>
            )}
          </div>
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
        </div>

        {status.message && (
          <div
            className={`p-4 rounded-md flex items-start ${
              status.type === "success"
                ? "bg-green-900 text-green-200 border border-green-700"
                : status.type === "error"
                  ? "bg-red-900 text-red-200 border border-red-700"
                  : ""
            }`}
          >
            <div className="flex-shrink-0 mr-3">
              {status.type === "success" ? (
                <svg
                  className="h-5 w-5 text-green-300"
                  xmlns="http://www.w3.org/2000/svg"
                  viewBox="0 0 20 20"
                  fill="currentColor"
                >
                  <path
                    fillRule="evenodd"
                    d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z"
                    clipRule="evenodd"
                  />
                </svg>
              ) : (
                <svg
                  className="h-5 w-5 text-red-300"
                  xmlns="http://www.w3.org/2000/svg"
                  viewBox="0 0 20 20"
                  fill="currentColor"
                >
                  <path
                    fillRule="evenodd"
                    d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z"
                    clipRule="evenodd"
                  />
                </svg>
              )}
            </div>
            <span className="text-sm">{status.message}</span>
          </div>
        )}

        <button
          type="submit"
          className={`w-full py-3 px-4 rounded-md text-white font-medium transition-all duration-200 ${
            isLoading
              ? "bg-purple-700 cursor-not-allowed"
              : "bg-purple-600 hover:bg-purple-500 focus:outline-none focus:ring-2 focus:ring-purple-500 focus:ring-offset-2 focus:ring-offset-gray-800 shadow-md hover:shadow-lg"
          }`}
          disabled={isLoading}
        >
          {isLoading ? (
            <div className="flex items-center justify-center">
              <svg
                className="animate-spin -ml-1 mr-3 h-5 w-5 text-white"
                xmlns="http://www.w3.org/2000/svg"
                fill="none"
                viewBox="0 0 24 24"
              >
                <circle
                  className="opacity-25"
                  cx="12"
                  cy="12"
                  r="10"
                  stroke="currentColor"
                  strokeWidth="4"
                ></circle>
                <path
                  className="opacity-75"
                  fill="currentColor"
                  d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                ></path>
              </svg>
              {t("auth.verifying")}
            </div>
          ) : (
            t("auth.submitButton")
          )}
        </button>
      </form>
    </div>
  );
};

export default AuthGatekeeper;

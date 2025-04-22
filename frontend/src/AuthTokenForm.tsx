import React, { useState, useEffect } from "react";

const AuthTokenForm = () => {
  const [authToken, setAuthToken] = useState("");
  const [status, setStatus] = useState({
    type: "idle",
    message: "",
  });
  const [savedToken, setSavedToken] = useState("");

  // Check for existing token on component mount
  useEffect(() => {
    const storedToken = localStorage.getItem("authToken");
    if (storedToken) {
      // Only show first few and last few characters for security
      const maskedToken = maskToken(storedToken);
      setSavedToken(maskedToken);
    }
  }, []);

  const maskToken = (token: string) => {
    if (token.length <= 8) return "••••••••";
    return (
      token.substring(0, 4) + "••••••••" + token.substring(token.length - 4)
    );
  };

  const handleSubmit = (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();

    if (!authToken.trim()) {
      setStatus({
        type: "error",
        message: "Please enter an auth token",
      });
      return;
    }

    try {
      localStorage.setItem("authToken", authToken);
      setStatus({
        type: "success",
        message: "Auth token saved successfully!",
      });
      setSavedToken(maskToken(authToken));

      // Clear the input on success
      setAuthToken("");

      // Clear success message after 3 seconds
      setTimeout(() => {
        setStatus({ type: "idle", message: "" });
      }, 3000);
    } catch (error) {
      const message = error instanceof Error ? error.message : "Unknown error";
      setStatus({
        type: "error",
        message: "Failed to save token: " + message,
      });
    }
  };

  const handleClearToken = () => {
    localStorage.removeItem("authToken");
    setSavedToken("");
    setStatus({
      type: "success",
      message: "Auth token removed successfully!",
    });

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
            Auth Token
          </label>
          <div className="relative">
            <input
              id="authToken"
              type="password"
              value={authToken}
              onChange={(e) => setAuthToken(e.target.value)}
              placeholder="Enter your auth token..."
              className="w-full p-4 bg-gray-700 border border-gray-600 rounded-md focus:ring-2 focus:ring-cyan-500 focus:border-cyan-500 text-sm text-gray-200 shadow-sm transition-all duration-200 placeholder-gray-400"
            />
            {authToken && (
              <button
                type="button"
                className="absolute top-3 right-2 text-gray-400 hover:text-gray-200"
                onClick={() => setAuthToken("")}
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
                Current token: <span className="font-mono">{savedToken}</span>
              </p>
              <button
                type="button"
                onClick={handleClearToken}
                className="text-xs text-red-400 hover:text-red-300"
              >
                Clear
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
          className="w-full py-3 px-4 rounded-md text-white font-medium transition-all duration-200 bg-purple-600 hover:bg-purple-500 focus:outline-none focus:ring-2 focus:ring-purple-500 focus:ring-offset-2 focus:ring-offset-gray-800 shadow-md hover:shadow-lg"
        >
          Save Auth Token
        </button>
      </form>
    </div>
  );
};

export default AuthTokenForm;

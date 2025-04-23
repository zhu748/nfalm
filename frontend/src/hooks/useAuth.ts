import { useState, useEffect } from "react";
import { maskToken } from "../utils/formatters";

export const useAuth = (onAuthenticated?: (status: boolean) => void) => {
  const [authToken, setAuthToken] = useState("");
  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState("");
  const [savedToken, setSavedToken] = useState("");

  // Check for existing token on mount
  useEffect(() => {
    const storedToken = localStorage.getItem("authToken");
    if (storedToken) {
      setSavedToken(maskToken(storedToken));
      validateToken(storedToken);
    }
  }, []);

  const validateToken = async (token: string) => {
    setIsLoading(true);
    setError("");

    try {
      const response = await fetch("/api/auth", {
        method: "GET",
        headers: {
          Authorization: `Bearer ${token}`,
          "Content-Type": "application/json",
        },
      });

      if (response.ok) {
        setIsAuthenticated(true);
        if (onAuthenticated) {
          onAuthenticated(true);
        }
      } else {
        localStorage.removeItem("authToken");
        setSavedToken("");
        setIsAuthenticated(false);
        setError("Invalid token. Please try again.");
        if (onAuthenticated) {
          onAuthenticated(false);
        }
      }
    } catch (err) {
      setIsAuthenticated(false);
      setError(err instanceof Error ? err.message : "Unknown error");
      if (onAuthenticated) {
        onAuthenticated(false);
      }
    } finally {
      setIsLoading(false);
    }
  };

  const login = async (token: string) => {
    if (!token.trim()) {
      setError("Please enter an auth token");
      return;
    }

    try {
      await validateToken(token);

      if (isAuthenticated) {
        localStorage.setItem("authToken", token);
        setSavedToken(maskToken(token));
        setAuthToken("");
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Unknown error");
    }
  };

  const logout = () => {
    localStorage.removeItem("authToken");
    setSavedToken("");
    setIsAuthenticated(false);
    if (onAuthenticated) {
      onAuthenticated(false);
    }
  };

  return {
    authToken,
    setAuthToken,
    isAuthenticated,
    isLoading,
    error,
    savedToken,
    login,
    logout,
    validateToken,
  };
};

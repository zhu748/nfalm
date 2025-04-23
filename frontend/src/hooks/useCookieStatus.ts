// frontend/src/hooks/useCookieStatus.ts
import { useState, useEffect, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { CookieStatusInfo } from "../types/cookie.types";
import { cookieService } from "../services/cookieService";

export const useCookieStatus = () => {
  const { t } = useTranslation();
  const [cookieStatus, setCookieStatus] = useState<CookieStatusInfo>(
    cookieService.emptyCookieStatus,
  );
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [refreshTrigger, setRefreshTrigger] = useState(0);
  const [deletingCookie, setDeletingCookie] = useState<string | null>(null);

  // Fetch cookie data
  const fetchCookieStatus = useCallback(async () => {
    setLoading(true);
    setError(null);

    try {
      const data = await cookieService.fetchCookieStatus();
      setCookieStatus(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setCookieStatus(cookieService.emptyCookieStatus);
    } finally {
      setLoading(false);
    }
  }, []);

  // Refresh handler
  const refreshCookieStatus = useCallback(() => {
    setRefreshTrigger((prev) => prev + 1);
  }, []);

  // Delete cookie handler
  const handleDeleteCookie = useCallback(
    async (cookie: string) => {
      if (!window.confirm(t("cookieStatus.deleteConfirm"))) return;

      setDeletingCookie(cookie);
      setError(null);

      try {
        await cookieService.deleteCookie(cookie);
        refreshCookieStatus();
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      } finally {
        setDeletingCookie(null);
      }
    },
    [t, refreshCookieStatus],
  );

  // Get reason text - convenience wrapper for the service
  const getReasonText = useCallback(
    (reason: any): string => {
      return cookieService.getReasonText(reason, t);
    },
    [t],
  );

  // Fetch cookie status on mount and when refreshTrigger changes
  useEffect(() => {
    fetchCookieStatus();
  }, [fetchCookieStatus, refreshTrigger]);

  // Calculate total cookie count
  const totalCookieCount =
    cookieStatus.valid.length +
    cookieStatus.dispatched.length +
    cookieStatus.exhausted.length +
    cookieStatus.invalid.length;

  return {
    cookieStatus,
    loading,
    error,
    deletingCookie,
    refreshCookieStatus,
    handleDeleteCookie,
    getReasonText,
    totalCookieCount,
  };
};

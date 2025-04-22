import React, { useState, useEffect } from "react";
import { getCookieStatus } from "./api";

// Updated interfaces to match the backend response structure
interface CookieStatus {
  cookie: string;  // Direct string value
  reset_time: number | null;
}

interface UselessCookie {
  cookie: string;  // Direct string value
  reason: string | any;  // Support both string and object formats
}

interface CookieStatusInfo {
  valid: CookieStatus[];
  dispatched: [CookieStatus, number][];
  exhausted: CookieStatus[];
  invalid: UselessCookie[];
}

// Default empty state to avoid undefined errors
const emptyCookieStatus: CookieStatusInfo = {
  valid: [],
  dispatched: [],
  exhausted: [],
  invalid: []
};

const CookieVisualization: React.FC = () => {
  const [cookieStatus, setCookieStatus] = useState<CookieStatusInfo>(emptyCookieStatus);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [refreshCounter, setRefreshCounter] = useState(0);

  const fetchCookieStatus = async () => {
    setLoading(true);
    setError(null);
    
    try {
      // Use the API helper function
      const data = await getCookieStatus();
      
      // Ensure data has proper structure, use defaults for any missing properties
      const safeData: CookieStatusInfo = {
        valid: Array.isArray(data?.valid) ? data.valid : [],
        dispatched: Array.isArray(data?.dispatched) ? data.dispatched : [],
        exhausted: Array.isArray(data?.exhausted) ? data.exhausted : [],
        invalid: Array.isArray(data?.invalid) ? data.invalid : []
      };
      
      setCookieStatus(safeData);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setCookieStatus(emptyCookieStatus); // Reset to empty state on error
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchCookieStatus();
  }, [refreshCounter]);

  const handleRefresh = () => {
    setRefreshCounter(prev => prev + 1);
  };

  const formatTimestamp = (timestamp: number): string => {
    if (!timestamp) return "N/A";
    try {
      const date = new Date(timestamp * 1000);
      return date.toLocaleString();
    } catch (e) {
      return "Invalid date";
    }
  };

  const formatTimeElapsed = (seconds: number): string => {
    if (!seconds && seconds !== 0) return "unknown";
    
    if (seconds < 60) return `${seconds} sec`;
    if (seconds < 3600) return `${Math.floor(seconds / 60)} min ${seconds % 60} sec`;
    return `${Math.floor(seconds / 3600)} hr ${Math.floor((seconds % 3600) / 60)} min`;
  };

  const getReasonText = (reason: any): string => {
    if (!reason) return "Unknown";
    
    // If reason is a string (like "Banned"), return it directly
    if (typeof reason === "string") {
      return reason;
    }
    
    // Handle object format
    try {
      if ("NonPro" in reason) return "Free account";
      if ("Disabled" in reason) return "Organization Disabled";
      if ("Banned" in reason) return "Banned";
      if ("Null" in reason) return "Invalid";
      if ("Restricted" in reason && typeof reason.Restricted === "number") 
        return `Restricted until ${formatTimestamp(reason.Restricted)}`;
      if ("TooManyRequest" in reason && typeof reason.TooManyRequest === "number") 
        return `Rate limited until ${formatTimestamp(reason.TooManyRequest)}`;
    } catch (e) {
      console.error("Error parsing reason:", e, reason);
    }
    
    return "Unknown";
  };

  const formatCookieValue = (cookie: string): string => {
    if (!cookie) return "";
    // Return the full cookie value
    return cookie;
  };

  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center mb-4">
        <div>
          <h3 className="text-lg font-semibold text-white">Cookie Status</h3>
          <p className="text-xs text-gray-400 mt-1">
            Total: {cookieStatus.valid.length + cookieStatus.dispatched.length + 
            cookieStatus.exhausted.length + cookieStatus.invalid.length} cookies
          </p>
        </div>
        <button 
          onClick={handleRefresh}
          className="p-2 bg-gray-700 hover:bg-gray-600 rounded-md transition-colors text-sm"
          disabled={loading}
        >
          {loading ? (
            <span className="flex items-center">
              <svg className="animate-spin h-4 w-4 mr-2" fill="none" viewBox="0 0 24 24">
                <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"></circle>
                <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
              </svg>
              Refreshing...
            </span>
          ) : (
            <span className="flex items-center">
              <svg xmlns="http://www.w3.org/2000/svg" className="h-4 w-4 mr-2" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
              </svg>
              Refresh
            </span>
          )}
        </button>
      </div>

      {error && (
        <div className="p-4 bg-red-900 text-red-200 border border-red-700 rounded-md">
          <p className="text-sm">{error}</p>
        </div>
      )}

      {loading && cookieStatus.valid.length === 0 && 
       cookieStatus.dispatched.length === 0 && 
       cookieStatus.exhausted.length === 0 && 
       cookieStatus.invalid.length === 0 && (
        <div className="flex justify-center py-8">
          <svg className="animate-spin h-8 w-8 text-cyan-500" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
            <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"></circle>
            <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
          </svg>
        </div>
      )}

      {cookieStatus && (
        <div className="space-y-6">
          {/* Valid Cookies */}
          <div className="rounded-lg border border-green-600 bg-gray-800 overflow-hidden">
            <div className="bg-green-900 px-4 py-2 flex justify-between items-center">
              <h4 className="font-medium text-green-100">Valid Cookies</h4>
              <span className="bg-green-700 text-green-100 text-xs px-2 py-1 rounded-full">
                {cookieStatus.valid.length}
              </span>
            </div>
            {cookieStatus?.valid?.length > 0 ? (
              <div className="p-4 divide-y divide-gray-700">
                {cookieStatus.valid.map((status, index) => (
                  <div key={index} className="py-2 text-sm text-gray-300 flex justify-between">
                    <code className="font-mono text-green-300">{formatCookieValue(status.cookie)}</code>
                    <span className="text-gray-400">Available</span>
                  </div>
                ))}
              </div>
            ) : (
              <div className="p-4 text-sm text-gray-400 italic">No valid cookies available</div>
            )}
          </div>

          {/* Dispatched Cookies */}
          <div className="rounded-lg border border-blue-600 bg-gray-800 overflow-hidden">
            <div className="bg-blue-900 px-4 py-2 flex justify-between items-center">
              <h4 className="font-medium text-blue-100">In-Use Cookies</h4>
              <span className="bg-blue-700 text-blue-100 text-xs px-2 py-1 rounded-full">
                {cookieStatus.dispatched.length}
              </span>
            </div>
            {cookieStatus?.dispatched?.length > 0 ? (
              <div className="p-4 divide-y divide-gray-700">
                {cookieStatus.dispatched.map(([status, time], index) => (
                  <div key={index} className="py-2 flex justify-between text-sm">
                    <code className="font-mono text-blue-300">{formatCookieValue(status.cookie)}</code>
                    <span className="text-gray-400">Used for {formatTimeElapsed(time)}</span>
                  </div>
                ))}
              </div>
            ) : (
              <div className="p-4 text-sm text-gray-400 italic">No cookies currently in use</div>
            )}
          </div>

          {/* Exhausted Cookies */}
          <div className="rounded-lg border border-yellow-600 bg-gray-800 overflow-hidden">
            <div className="bg-yellow-900 px-4 py-2 flex justify-between items-center">
              <h4 className="font-medium text-yellow-100">Exhausted Cookies</h4>
              <span className="bg-yellow-700 text-yellow-100 text-xs px-2 py-1 rounded-full">
                {cookieStatus.exhausted.length}
              </span>
            </div>
            {cookieStatus?.exhausted?.length > 0 ? (
              <div className="p-4 divide-y divide-gray-700">
                {cookieStatus.exhausted.map((status, index) => (
                  <div key={index} className="py-2 flex justify-between text-sm">
                    <code className="font-mono text-yellow-300">{formatCookieValue(status.cookie)}</code>
                    <span className="text-gray-400">
                      {status.reset_time 
                        ? `Resets at ${formatTimestamp(status.reset_time)}` 
                        : "Unknown reset time"}
                    </span>
                  </div>
                ))}
              </div>
            ) : (
              <div className="p-4 text-sm text-gray-400 italic">No exhausted cookies</div>
            )}
          </div>

          {/* Invalid Cookies */}
          <div className="rounded-lg border border-red-600 bg-gray-800 overflow-hidden">
            <div className="bg-red-900 px-4 py-2 flex justify-between items-center">
              <h4 className="font-medium text-red-100">Invalid Cookies</h4>
              <span className="bg-red-700 text-red-100 text-xs px-2 py-1 rounded-full">
                {cookieStatus.invalid.length}
              </span>
            </div>
            {cookieStatus?.invalid?.length > 0 ? (
              <div className="p-4 divide-y divide-gray-700">
                {cookieStatus.invalid.map((status, index) => (
                  <div key={index} className="py-2 flex justify-between text-sm">
                    <code className="font-mono text-red-300">{formatCookieValue(status.cookie)}</code>
                    <span className="text-gray-400">{getReasonText(status.reason)}</span>
                  </div>
                ))}
              </div>
            ) : (
              <div className="p-4 text-sm text-gray-400 italic">No invalid cookies</div>
            )}
          </div>
        </div>
      )}
    </div>
  );
};

export default CookieVisualization;
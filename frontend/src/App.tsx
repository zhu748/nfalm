// frontend/src/App.tsx
import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import "./App.css";
import { getVersion } from "./api";
import MainLayout from "./components/layout/MainLayout";
import Card from "./components/common/Card";
import TabNavigation from "./components/common/TabNavigation";
import AuthGatekeeper from "./components/auth/AuthGatekeeper";
import LogoutPanel from "./components/auth/LogoutPanel";
import CookieTabs from "./components/cookie";
import ConfigTab from "./components/config";
import StatusMessage from "./components/common/StatusMessage";
import "./i18n"; // Import i18n configuration

function App() {
  const { t } = useTranslation();
  const [version, setVersion] = useState("");
  const [activeTab, setActiveTab] = useState("cookie"); // "cookie", "config", or "token"
  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const [passwordChanged, setPasswordChanged] = useState(false);

  useEffect(() => {
    // Fetch and set the version when component mounts
    getVersion().then((v) => setVersion(v));

    // Check if redirected due to password change
    const params = new URLSearchParams(window.location.search);
    if (params.get("passwordChanged") === "true") {
      setPasswordChanged(true);
      // Clean up the URL
      window.history.replaceState({}, document.title, window.location.pathname);
    }

    // Check for authentication status
    const checkAuth = async () => {
      const storedToken = localStorage.getItem("authToken");
      if (storedToken) {
        try {
          const response = await fetch("/api/auth", {
            method: "GET",
            headers: {
              Authorization: `Bearer ${storedToken}`,
              "Content-Type": "application/json",
            },
          });

          if (response.ok) {
            setIsAuthenticated(true);
          } else {
            // Invalid token, clear it
            localStorage.removeItem("authToken");
            setIsAuthenticated(false);
          }
        } catch (error) {
          console.error("Authentication check failed:", error);
          setIsAuthenticated(false);
        }
      }
    };

    checkAuth();
  }, []);

  // Function to handle successful authentication
  const handleAuthenticated = (status: boolean) => {
    setIsAuthenticated(status);
  };

  // Function to handle logout
  const handleLogout = () => {
    localStorage.removeItem("authToken");
    setIsAuthenticated(false);
  };

  // Define tabs for the authenticated user
  const tabs = [
    { id: "cookie", label: t("tabs.cookie"), color: "cyan" },
    { id: "config", label: t("tabs.config"), color: "green" },
    { id: "token", label: t("tabs.auth"), color: "purple" },
  ];

  return (
    <MainLayout version={version}>
      {isAuthenticated ? (
        // Protected content - only shown when authenticated
        <Card className="w-full max-w-md sm:max-w-lg md:max-w-xl mx-auto">
          <TabNavigation
            tabs={tabs}
            activeTab={activeTab}
            onTabChange={(tabId) => setActiveTab(tabId)}
            className="mb-6"
          />

          {activeTab === "cookie" ? (
            <CookieTabs />
          ) : activeTab === "config" ? (
            <ConfigTab />
          ) : (
            <LogoutPanel onLogout={handleLogout} />
          )}
        </Card>
      ) : (
        // Auth gatekeeper - shown when not authenticated
        <Card className="w-full max-w-md sm:max-w-lg md:max-w-xl mx-auto">
          <h2 className="text-xl font-semibold text-center mb-6">
            {t("auth.title")}
          </h2>

          {passwordChanged && (
            <StatusMessage type="info" message={t("auth.passwordChanged")} />
          )}

          <p className="text-gray-400 text-sm mb-6 text-center">
            {t("auth.description")}
          </p>

          <AuthGatekeeper onAuthenticated={handleAuthenticated} />
        </Card>
      )}
    </MainLayout>
  );
}

export default App;

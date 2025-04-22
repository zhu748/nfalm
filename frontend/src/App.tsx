import "./App.css";
import { getVersion } from "./api";
import { useState, useEffect } from "react";
import CookieSubmitForm from "./SubmitCookieForm";
import AuthTokenForm from "./AuthTokenForm";

function App() {
  const [version, setVersion] = useState("");
  const [activeTab, setActiveTab] = useState("token"); // "cookie" or "token"

  useEffect(() => {
    // Fetch and set the version when component mounts
    getVersion().then((v) => setVersion(v));
  }, []);

  return (
    <div className="min-h-screen bg-gradient-to-b from-gray-900 to-gray-800 text-white">
      <div className="container mx-auto px-4 py-10">
        <header className="mb-10 text-center">
          <h1 className="text-4xl font-bold mb-2 text-transparent bg-clip-text bg-gradient-to-r from-cyan-400 to-purple-500">
            ClewdR
          </h1>
          <h2 className="text-sm font-mono text-gray-400">{version}</h2>
        </header>

        <div className="max-w-md mx-auto rounded-xl shadow-xl p-6 border border-gray-700 bg-gray-800/50 backdrop-blur-sm">
          <div className="flex mb-6 border-b border-gray-700">
            <button
              onClick={() => setActiveTab("cookie")}
              className={`flex-1 py-2 font-medium text-sm transition-colors duration-200 ${
                activeTab === "cookie"
                  ? "text-cyan-400 border-b-2 border-cyan-400"
                  : "text-gray-400 hover:text-gray-300"
              }`}
            >
              Cookie
            </button>
            <button
              onClick={() => setActiveTab("token")}
              className={`flex-1 py-2 font-medium text-sm transition-colors duration-200 ${
                activeTab === "token"
                  ? "text-purple-400 border-b-2 border-purple-400"
                  : "text-gray-400 hover:text-gray-300"
              }`}
            >
              Auth Token
            </button>
          </div>

          {activeTab === "cookie" ? <CookieSubmitForm /> : <AuthTokenForm />}
        </div>

        <footer className="mt-12 text-center text-gray-500 text-sm">
          <p>© {new Date().getFullYear()} ClewdR - All rights reserved</p>
        </footer>
      </div>
    </div>
  );
}

export default App;

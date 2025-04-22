import "./App.css";
import { getVersion } from "./api";
import { useState, useEffect } from "react";
import CookieSubmitForm from "./submit";

function App() {
  const [version, setVersion] = useState("");

  useEffect(() => {
    // Fetch and set the version when component mounts
    getVersion().then((v) => setVersion(v));
  }, []);

  return (
    <div className="min-h-screen bg-gradient-to-b from-gray-900 to-gray-800 text-white">
      <div className="container mx-auto px-4 py-10">
        <header className="mb-10 text-center">
          <h1 className="text-4xl font-bold mb-2 text-transparent bg-clip-text bg-gradient-to-r from-cyan-400 to-purple-500">ClewdR</h1>
          <h2 className="text-sm font-mono text-gray-400">{version}</h2>
        </header>
        
        <div className="max-w-md mx-auto rounded-xl shadow-xl p-6 border border-gray-700 bg-gray-800/50 backdrop-blur-sm">
          <CookieSubmitForm />
        </div>
        
        <footer className="mt-12 text-center text-gray-500 text-sm">
          <p>© {new Date().getFullYear()} ClewdR - All rights reserved</p>
        </footer>
      </div>
    </div>
  );
}

export default App;
import React, { useState } from "react";
import CookieSubmitForm from "./SubmitCookieForm";
import CookieVisualization from "./CookieVisualization";

const CookieTabs: React.FC = () => {
  const [activeTab, setActiveTab] = useState<"submit" | "status">("submit");

  return (
    <div className="w-full">
      <div className="flex mb-6 border-b border-gray-700 w-full">
        <button
          onClick={() => setActiveTab("submit")}
          className={`flex-1 py-2 font-medium text-sm transition-colors duration-200 ${
            activeTab === "submit"
              ? "text-cyan-400 border-b-2 border-cyan-400"
              : "text-gray-400 hover:text-gray-300"
          }`}
        >
          Submit Cookie
        </button>
        <button
          onClick={() => setActiveTab("status")}
          className={`flex-1 py-2 font-medium text-sm transition-colors duration-200 ${
            activeTab === "status"
              ? "text-cyan-400 border-b-2 border-cyan-400"
              : "text-gray-400 hover:text-gray-300"
          }`}
        >
          Cookie Status
        </button>
      </div>

      {activeTab === "submit" ? <CookieSubmitForm /> : <CookieVisualization />}
    </div>
  );
};

export default CookieTabs;
import React from "react";
import { Tab } from "../../types/ui.types";

interface TabNavigationProps {
  tabs: Tab[];
  activeTab: string;
  onTabChange: (tabId: string) => void;
  className?: string;
}
const TabNavigation: React.FC<TabNavigationProps> = ({
  tabs,
  activeTab,
  onTabChange,
  className = "",
}) => {
  const getTabStyle = (tab: Tab) => {
    const color = tab.color || "cyan";
    return activeTab === tab.id
      ? `text-${color}-400 border-b-2 border-${color}-400`
      : "text-gray-400 hover:text-gray-300";
  };

  return (
    <div className={`flex border-b border-gray-700 ${className}`}>
      {tabs.map((tab) => (
        <button
          key={tab.id}
          onClick={() => onTabChange(tab.id)}
          className={`flex-1 py-2 font-medium text-sm transition-colors duration-200 ${getTabStyle(
            tab,
          )}`}
        >
          {tab.label}
        </button>
      ))}
    </div>
  );
};

export default TabNavigation;

// frontend/src/components/keys/KeyValue.tsx
import React, { useState } from "react";
import { useTranslation } from "react-i18next";

interface KeyValueProps {
  keyString: string;
}

const KeyValue: React.FC<KeyValueProps> = ({ keyString }) => {
  const { t } = useTranslation();
  const [isRevealed, setIsRevealed] = useState(false);

  // Function to reveal/hide the full key
  const toggleReveal = () => {
    setIsRevealed(!isRevealed);
  };

  // Function to mask part of the key for security
  const getMaskedKey = () => {
    if (keyString.length <= 8) {
      return "****";
    }

    const prefix = keyString.substring(0, 4);
    const suffix = keyString.substring(keyString.length - 4);
    const maskedMiddle = "****";

    return `${prefix}${maskedMiddle}${suffix}`;
  };

  // Function to copy key to clipboard
  const copyToClipboard = () => {
    navigator.clipboard.writeText(keyString);
  };

  return (
    <div className="flex items-center">
      <code className="font-mono truncate">
        {isRevealed ? keyString : getMaskedKey()}
      </code>
      <button
        onClick={toggleReveal}
        className="ml-2 text-xs text-gray-400 hover:text-gray-300"
        title={isRevealed ? t("common.hide") : t("common.reveal")}
      >
        {isRevealed ? (
          <span>👁️</span>
        ) : (
          <span>👁️‍🗨️</span>
        )}
      </button>
      <button
        onClick={copyToClipboard}
        className="ml-2 text-xs text-gray-400 hover:text-gray-300"
        title={t("common.copy")}
      >
        📋
      </button>
    </div>
  );
};

export default KeyValue;

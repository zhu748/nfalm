// frontend/src/components/keys/DeleteButton.tsx
import React from "react";
import { useTranslation } from "react-i18next";

interface DeleteButtonProps {
  keyString: string;
  onDelete: (key: string) => Promise<void>;
  isDeleting: boolean;
}

const DeleteButton: React.FC<DeleteButtonProps> = ({
  keyString,
  onDelete,
  isDeleting,
}) => {
  const { t } = useTranslation();

  return (
    <button
      onClick={() => onDelete(keyString)}
      disabled={isDeleting}
      className="ml-3 text-sm text-gray-400 hover:text-red-400 transition-colors disabled:opacity-50"
      title={t("keyStatus.delete")}
    >
      {isDeleting ? (
        <span className="animate-pulse">⏳</span>
      ) : (
        <span>🗑️</span>
      )}
    </button>
  );
};

export default DeleteButton;

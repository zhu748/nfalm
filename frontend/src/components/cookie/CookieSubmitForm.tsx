import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { postCookie } from "../../api";
import Button from "../common/Button";
import FormInput from "../common/FormInput";
import StatusMessage from "../common/StatusMessage";

const CookieSubmitForm: React.FC = () => {
  const { t } = useTranslation();
  const [cookie, setCookie] = useState("");
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [status, setStatus] = useState({
    type: "idle" as "idle" | "success" | "error",
    message: "",
  });

  const handleSubmit = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();

    if (!cookie.trim()) {
      setStatus({
        type: "error",
        message: t("cookieSubmit.error.empty"),
      });
      return;
    }

    setIsSubmitting(true);
    setStatus({ type: "idle", message: "" });

    try {
      await postCookie(cookie);
      setStatus({
        type: "success",
        message: t("cookieSubmit.success"),
      });
      setCookie(""); // Clear the input field after successful submission
    } catch (e) {
      // Try to match specific error messages to translations
      const errorMessage = e instanceof Error ? e.message : "Unknown error";

      if (errorMessage.includes("Invalid cookie format")) {
        setStatus({
          type: "error",
          message: t("cookieSubmit.error.format"),
        });
      } else if (errorMessage.includes("Authentication failed")) {
        setStatus({
          type: "error",
          message: t("cookieSubmit.error.auth"),
        });
      } else if (errorMessage.includes("Server error")) {
        setStatus({
          type: "error",
          message: t("cookieSubmit.error.server"),
        });
      } else {
        setStatus({
          type: "error",
          message: errorMessage,
        });
      }
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div>
      <form onSubmit={handleSubmit} className="space-y-6">
        <FormInput
          id="cookie"
          name="cookie"
          value={cookie}
          onChange={(e) => setCookie(e.target.value)}
          placeholder={t("cookieSubmit.placeholder")}
          label={t("cookieSubmit.value")}
          isTextarea={true}
          onClear={() => setCookie("")}
          disabled={isSubmitting}
        />

        <p className="text-xs text-gray-400 mt-1">
          {t("cookieSubmit.description")}
        </p>

        {status.message && (
          <StatusMessage
            type={status.type === "success" ? "success" : "error"}
            message={status.message}
          />
        )}

        <Button
          type="submit"
          disabled={isSubmitting}
          isLoading={isSubmitting}
          className="w-full"
        >
          {isSubmitting
            ? t("cookieSubmit.submitting")
            : t("cookieSubmit.submitButton")}
        </Button>
      </form>
    </div>
  );
};

export default CookieSubmitForm;

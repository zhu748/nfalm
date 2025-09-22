import React, { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { getConfig, saveConfig, storageImport, storageExport, storageStatus } from "../../api";
import { toast } from "react-hot-toast";
import { ConfigData } from "../../types/config.types";
import Button from "../common/Button";
import LoadingSpinner from "../common/LoadingSpinner";
import ConfigForm from "./ConfigForm";
import { StorageSummary } from "./StorageSummary";

const ConfigTab: React.FC = () => {
  const { t } = useTranslation();
  const [config, setConfig] = useState<ConfigData | null>(null);
  const [originalPassword, setOriginalPassword] = useState<string>("");
  const [originalAdminPassword, setOriginalAdminPassword] =
    useState<string>("");
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  const [status, setStatus] = useState<any>(null);

  // Fetch config on component mount
  useEffect(() => {
    fetchConfig();
    // fetch storage status once
    storageStatus().then(setStatus).catch(() => setStatus(null));
  }, []);

  const fetchConfig = async () => {
    setLoading(true);
    setError("");
    try {
      const data = await getConfig();
      setConfig(data);
      // Store the original passwords for comparison later
      setOriginalPassword(data.password || "");
      setOriginalAdminPassword(data.admin_password || "");
    } catch (err) {
      setError(
        t("common.error", {
          message: err instanceof Error ? err.message : String(err),
        })
      );
      console.error("Config fetch error:", err);
    } finally {
      setLoading(false);
    }
  };

  const handleSave = async () => {
    if (!config) return;

    setSaving(true);
    setError("");
    try {
      await saveConfig(config);
      toast.success(t("config.success"));

      // Check if admin password was changed - this affects the current session
      const adminPasswordChanged =
        config.admin_password !== originalAdminPassword;

      // Check if regular password was changed - doesn't affect current session
      const regularPasswordChanged = config.password !== originalPassword;

      if (regularPasswordChanged) {
        toast.success(t("config.passwordChanged"), {
          duration: 2000,
          icon: "🔑",
        });
      }

      // If admin password changed, we need to log out and redirect
      if (adminPasswordChanged) {
        // Show toast notification about admin password change
        toast.success(t("config.adminPasswordChanged"), {
          duration: 3000,
          icon: "🔐",
        });

        // Wait 3 seconds before logging out to allow user to see the toast
        setTimeout(() => {
          localStorage.removeItem("authToken");
          // Redirect with a query parameter to indicate password change
          window.location.href = "/?passwordChanged=true";
        }, 3000);
      }
    } catch (err) {
      setError(
        t("common.error", {
          message: err instanceof Error ? err.message : String(err),
        })
      );
      console.error("Config save error:", err);
      toast.error(t("config.error"));
    } finally {
      setSaving(false);
    }
  };

  const handleChange = (
    e: React.ChangeEvent<
      HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement
    >
  ) => {
    if (!config) return;

    const { name, value, type } = e.target;

    // Handle checkboxes
    if (type === "checkbox") {
      const checked = (e.target as HTMLInputElement).checked;
      setConfig({
        ...config,
        [name]: checked,
      });
      return;
    }

    // Handle numbers
    if (type === "number") {
      setConfig({
        ...config,
        [name]: value === "" ? 0 : Number(value),
      });
      return;
    }

    // Handle empty strings for nullable fields
    if (name.startsWith("vertex.")) {
      const vertexField = name.split(".")[1]; // Gets 'auth\_token', 'project\_id', or 'model\_id'
      setConfig({
        ...config,
        vertex: {
          ...config.vertex,
          [vertexField]: value === "" ? null : value,
        },
      });
      return;
    } // Handle empty strings for nullable fields
    if (
      ["proxy", "rproxy", "custom_h", "custom_a"].includes(name) &&
      value === ""
    ) {
      setConfig({ ...config, [name]: null });
      return;
    }

    // Handle regular text inputs
    setConfig({
      ...config,
      [name]: value,
    });
  };

  if (loading) {
    return (
      <div className="flex justify-center items-center p-8">
        <LoadingSpinner size="md" />
      </div>
    );
  }

  if (error) {
    return (
      <div className="bg-red-500/20 border border-red-500 rounded-lg p-4 mb-4">
        <p className="text-red-200">{error}</p>
        <Button
          onClick={fetchConfig}
          className="mt-2 py-1 px-3"
          variant="danger"
        >
          {t("config.retry")}
        </Button>
      </div>
    );
  }

  if (!config) {
    return (
      <div className="bg-amber-500/20 border border-amber-500 rounded-lg p-4">
        <p className="text-amber-200">{t("config.noData")}</p>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center">
        <h3 className="text-lg font-medium text-white">{t("config.title")}</h3>
        <Button
          onClick={handleSave}
          disabled={saving}
          isLoading={saving}
          className="py-2 px-4 bg-gradient-to-r from-cyan-500 to-purple-500 hover:from-cyan-400 hover:to-purple-400"
          variant="primary"
        >
          {saving ? t("config.saving") : t("config.saveButton")}
        </Button>
      </div>

      <div className="rounded-lg border border-white/10 bg-white/5 p-4">
        <div className="flex flex-col gap-4 md:flex-row md:items-start md:justify-between">
          <StorageSummary status={status} mode={config?.persistence?.mode} />
          <div className="flex gap-2">
            <Button
              onClick={async () => {
                try {
                  await storageImport();
                  toast.success(t("config.storage.importSuccess"));
                } catch (e) {
                  toast.error((e as Error).message);
                }
              }}
              variant="secondary"
              className="py-1 px-3"
              disabled={(config?.persistence?.mode ?? "file") === "file"}
            >
              {t("config.storage.import")}
            </Button>
            <Button
              onClick={async () => {
                try {
                  const res = await storageExport();
                  if (res?.toml) {
                    const blob = new Blob([res.toml], {
                      type: "text/plain;charset=utf-8",
                    });
                    const url = URL.createObjectURL(blob);
                    const link = document.createElement("a");
                    link.href = url;
                    link.download = "clewdr-export.toml";
                    document.body.appendChild(link);
                    link.click();
                    document.body.removeChild(link);
                    URL.revokeObjectURL(url);
                  }
                  toast.success(t("config.storage.exportSuccess"));
                } catch (e) {
                  toast.error((e as Error).message);
                }
              }}
              variant="secondary"
              className="py-1 px-3"
              disabled={(config?.persistence?.mode ?? "file") === "file"}
            >
              {t("config.storage.export")}
            </Button>
          </div>
        </div>
      </div>

      <ConfigForm config={config} onChange={handleChange} />
    </div>
  );
};

export default ConfigTab;

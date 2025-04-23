import { useState, useEffect } from "react";
import { toast } from "react-hot-toast";

interface ConfigData {
  // Server settings
  ip: string;
  port: number;
  enable_oai: boolean;

  // App settings
  check_update: boolean;
  auto_update: boolean;

  // Network settings
  password: string;
  proxy: string | null;
  rproxy: string | null;

  // API settings
  max_retries: number;
  pass_params: boolean;
  preserve_chats: boolean;

  // Cookie settings
  skip_warning: boolean;
  skip_restricted: boolean;
  skip_non_pro: boolean;

  // Prompt configurations
  use_real_roles: boolean;
  custom_h: string | null;
  custom_a: string | null;
  custom_prompt: string;
  padtxt_file: string | null;
  padtxt_len: number;
}

const ConfigTab = () => {
  const [config, setConfig] = useState<ConfigData | null>(null);
  const [originalPassword, setOriginalPassword] = useState<string>("");
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  
  // Fetch config on component mount
  useEffect(() => {
    fetchConfig();
  }, []);

  const fetchConfig = async () => {
    setLoading(true);
    setError("");
    try {
      const token = localStorage.getItem("authToken");
      const response = await fetch("/api/config", {
        method: "GET",
        headers: {
          "Content-Type": "application/json",
          "Authorization": `Bearer ${token}`
        },
      });

      if (!response.ok) {
        throw new Error(`Failed to fetch config: ${response.status}`);
      }

      const data = await response.json();
      setConfig(data);
      // Store the original password for comparison later
      setOriginalPassword(data.password || "");
    } catch (err) {
      setError(`Error fetching configuration: ${err instanceof Error ? err.message : String(err)}`);
      console.error("Config fetch error:", err);
    } finally {
      setLoading(false);
    }
  };

  const saveConfig = async () => {
    if (!config) return;
    
    setSaving(true);
    setError("");
    try {
      const token = localStorage.getItem("authToken");
      const response = await fetch("/api/config", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "Authorization": `Bearer ${token}`
        },
        body: JSON.stringify(config),
      });

      if (!response.ok) {
        throw new Error(`Failed to save config: ${response.status}`);
      }

      toast.success("Configuration saved successfully");
      
      // Check if password was changed
      if (config.password !== originalPassword) {
        // Show toast notification
        toast.success("Password changed. You will be redirected to login page.", {
          duration: 3000,
          icon: '🔐',
        });
        
        // Wait 3 seconds before logging out to allow user to see the toast
        setTimeout(() => {
          localStorage.removeItem("authToken");
          // Redirect with a query parameter to indicate password change
          window.location.href = '/?passwordChanged=true';
        }, 3000);
      }
    } catch (err) {
      setError(`Error saving configuration: ${err instanceof Error ? err.message : String(err)}`);
      console.error("Config save error:", err);
      toast.error("Failed to save configuration");
    } finally {
      setSaving(false);
    }
  };

  const handleChange = (e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement>) => {
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
    if (["proxy", "rproxy", "custom_h", "custom_a", "padtxt_file"].includes(name) && value === "") {
      setConfig({
        ...config,
        [name]: null,
      });
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
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-cyan-400"></div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="bg-red-500/20 border border-red-500 rounded-lg p-4 mb-4">
        <p className="text-red-200">{error}</p>
        <button 
          onClick={fetchConfig}
          className="mt-2 py-1 px-3 bg-red-500 hover:bg-red-400 text-white rounded-md text-sm transition-colors duration-200"
        >
          Retry
        </button>
      </div>
    );
  }

  if (!config) {
    return (
      <div className="bg-amber-500/20 border border-amber-500 rounded-lg p-4">
        <p className="text-amber-200">No configuration data available.</p>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center">
        <h3 className="text-lg font-medium text-white">Configuration</h3>
        <button
          onClick={saveConfig}
          disabled={saving}
          className={`py-2 px-4 ${
            saving
              ? "bg-gray-500 cursor-not-allowed"
              : "bg-gradient-to-r from-cyan-500 to-purple-500 hover:from-cyan-400 hover:to-purple-400"
          } text-white rounded-md text-sm font-medium transition-colors duration-200`}
        >
          {saving ? "Saving..." : "Save Configuration"}
        </button>
      </div>

      <div className="space-y-6">
        {/* Server Settings Section */}
        <div className="bg-gray-700/60 p-4 rounded-lg">
          <h4 className="text-md font-medium text-cyan-300 mb-3">Server Settings</h4>
          <p className="text-xs text-gray-400 mb-3">These settings require a restart to take effect.</p>
          
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <label className="block text-sm font-medium text-gray-300 mb-1">IP Address</label>
              <input
                type="text"
                name="ip"
                value={config.ip}
                onChange={handleChange}
                className="w-full bg-gray-800 border border-gray-600 rounded-md py-2 px-3 text-sm text-white focus:outline-none focus:ring-1 focus:ring-cyan-500 focus:border-cyan-500"
              />
            </div>
            
            <div>
              <label className="block text-sm font-medium text-gray-300 mb-1">Port</label>
              <input
                type="number"
                name="port"
                value={config.port}
                onChange={handleChange}
                className="w-full bg-gray-800 border border-gray-600 rounded-md py-2 px-3 text-sm text-white focus:outline-none focus:ring-1 focus:ring-cyan-500 focus:border-cyan-500"
              />
            </div>
          </div>
          
          <div className="mt-3">
            <label className="flex items-center space-x-2 cursor-pointer">
              <input
                type="checkbox"
                name="enable_oai"
                checked={config.enable_oai}
                onChange={handleChange}
                className="w-4 h-4 bg-gray-800 border-gray-600 rounded text-cyan-500 focus:ring-cyan-500 focus:ring-opacity-25"
              />
              <span className="text-sm text-gray-300">Enable OpenAI compatibility</span>
            </label>
          </div>
        </div>
        
        {/* App Settings Section */}
        <div className="bg-gray-700/60 p-4 rounded-lg">
          <h4 className="text-md font-medium text-cyan-300 mb-3">App Settings</h4>
          
          <div className="space-y-3">
            <label className="flex items-center space-x-2 cursor-pointer">
              <input
                type="checkbox"
                name="check_update"
                checked={config.check_update}
                onChange={handleChange}
                className="w-4 h-4 bg-gray-800 border-gray-600 rounded text-cyan-500 focus:ring-cyan-500 focus:ring-opacity-25"
              />
              <span className="text-sm text-gray-300">Check for updates</span>
            </label>
            
            <label className="flex items-center space-x-2 cursor-pointer">
              <input
                type="checkbox"
                name="auto_update"
                checked={config.auto_update}
                onChange={handleChange}
                className="w-4 h-4 bg-gray-800 border-gray-600 rounded text-cyan-500 focus:ring-cyan-500 focus:ring-opacity-25"
              />
              <span className="text-sm text-gray-300">Auto update</span>
            </label>
          </div>
        </div>
        
        {/* Network Settings Section */}
        <div className="bg-gray-700/60 p-4 rounded-lg">
          <h4 className="text-md font-medium text-cyan-300 mb-3">Network Settings</h4>
          
          <div className="space-y-4">
            <div>
              <label className="block text-sm font-medium text-gray-300 mb-1">Password</label>
              <input
                type="password"
                name="password"
                value={config.password}
                onChange={handleChange}
                className="w-full bg-gray-800 border border-gray-600 rounded-md py-2 px-3 text-sm text-white focus:outline-none focus:ring-1 focus:ring-cyan-500 focus:border-cyan-500"
              />
            </div>
            
            <div>
              <label className="block text-sm font-medium text-gray-300 mb-1">Proxy (optional)</label>
              <input
                type="text"
                name="proxy"
                value={config.proxy || ""}
                onChange={handleChange}
                placeholder="http://proxy:port"
                className="w-full bg-gray-800 border border-gray-600 rounded-md py-2 px-3 text-sm text-white focus:outline-none focus:ring-1 focus:ring-cyan-500 focus:border-cyan-500"
              />
            </div>
            
            <div>
              <label className="block text-sm font-medium text-gray-300 mb-1">Reverse Proxy (optional)</label>
              <input
                type="text"
                name="rproxy"
                value={config.rproxy || ""}
                onChange={handleChange}
                placeholder="https://example.com"
                className="w-full bg-gray-800 border border-gray-600 rounded-md py-2 px-3 text-sm text-white focus:outline-none focus:ring-1 focus:ring-cyan-500 focus:border-cyan-500"
              />
            </div>
          </div>
        </div>
        
        {/* API Settings Section */}
        <div className="bg-gray-700/60 p-4 rounded-lg">
          <h4 className="text-md font-medium text-cyan-300 mb-3">API Settings</h4>
          
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-3">
            <div>
              <label className="block text-sm font-medium text-gray-300 mb-1">Max Retries</label>
              <input
                type="number"
                name="max_retries"
                value={config.max_retries}
                onChange={handleChange}
                className="w-full bg-gray-800 border border-gray-600 rounded-md py-2 px-3 text-sm text-white focus:outline-none focus:ring-1 focus:ring-cyan-500 focus:border-cyan-500"
              />
            </div>
          </div>
          
          <div className="space-y-3">
            <label className="flex items-center space-x-2 cursor-pointer">
              <input
                type="checkbox"
                name="pass_params"
                checked={config.pass_params}
                onChange={handleChange}
                className="w-4 h-4 bg-gray-800 border-gray-600 rounded text-cyan-500 focus:ring-cyan-500 focus:ring-opacity-25"
              />
              <span className="text-sm text-gray-300">Pass Parameters</span>
            </label>
            
            <label className="flex items-center space-x-2 cursor-pointer">
              <input
                type="checkbox"
                name="preserve_chats"
                checked={config.preserve_chats}
                onChange={handleChange}
                className="w-4 h-4 bg-gray-800 border-gray-600 rounded text-cyan-500 focus:ring-cyan-500 focus:ring-opacity-25"
              />
              <span className="text-sm text-gray-300">Preserve Chats</span>
            </label>
          </div>
        </div>
        
        {/* Cookie Settings Section */}
        <div className="bg-gray-700/60 p-4 rounded-lg">
          <h4 className="text-md font-medium text-cyan-300 mb-3">Cookie Settings</h4>
          
          <div className="space-y-3">
            <label className="flex items-center space-x-2 cursor-pointer">
              <input
                type="checkbox"
                name="skip_warning"
                checked={config.skip_warning}
                onChange={handleChange}
                className="w-4 h-4 bg-gray-800 border-gray-600 rounded text-cyan-500 focus:ring-cyan-500 focus:ring-opacity-25"
              />
              <span className="text-sm text-gray-300">Skip Warning</span>
            </label>
            
            <label className="flex items-center space-x-2 cursor-pointer">
              <input
                type="checkbox"
                name="skip_restricted"
                checked={config.skip_restricted}
                onChange={handleChange}
                className="w-4 h-4 bg-gray-800 border-gray-600 rounded text-cyan-500 focus:ring-cyan-500 focus:ring-opacity-25"
              />
              <span className="text-sm text-gray-300">Skip Restricted</span>
            </label>
            
            <label className="flex items-center space-x-2 cursor-pointer">
              <input
                type="checkbox"
                name="skip_non_pro"
                checked={config.skip_non_pro}
                onChange={handleChange}
                className="w-4 h-4 bg-gray-800 border-gray-600 rounded text-cyan-500 focus:ring-cyan-500 focus:ring-opacity-25"
              />
              <span className="text-sm text-gray-300">Skip Non-Pro</span>
            </label>
          </div>
        </div>
        
        {/* Prompt Configurations Section */}
        <div className="bg-gray-700/60 p-4 rounded-lg">
          <h4 className="text-md font-medium text-cyan-300 mb-3">Prompt Configurations</h4>
          
          <div className="space-y-4">
            <label className="flex items-center space-x-2 cursor-pointer mb-3">
              <input
                type="checkbox"
                name="use_real_roles"
                checked={config.use_real_roles}
                onChange={handleChange}
                className="w-4 h-4 bg-gray-800 border-gray-600 rounded text-cyan-500 focus:ring-cyan-500 focus:ring-opacity-25"
              />
              <span className="text-sm text-gray-300">Use Real Roles</span>
            </label>
            
            <div>
              <label className="block text-sm font-medium text-gray-300 mb-1">Custom Human (optional)</label>
              <input
                type="text"
                name="custom_h"
                value={config.custom_h || ""}
                onChange={handleChange}
                className="w-full bg-gray-800 border border-gray-600 rounded-md py-2 px-3 text-sm text-white focus:outline-none focus:ring-1 focus:ring-cyan-500 focus:border-cyan-500"
              />
            </div>
            
            <div>
              <label className="block text-sm font-medium text-gray-300 mb-1">Custom Assistant (optional)</label>
              <input
                type="text"
                name="custom_a"
                value={config.custom_a || ""}
                onChange={handleChange}
                className="w-full bg-gray-800 border border-gray-600 rounded-md py-2 px-3 text-sm text-white focus:outline-none focus:ring-1 focus:ring-cyan-500 focus:border-cyan-500"
              />
            </div>
            
            <div>
              <label className="block text-sm font-medium text-gray-300 mb-1">Custom Prompt</label>
              <textarea
                name="custom_prompt"
                value={config.custom_prompt}
                onChange={handleChange}
                rows={3}
                className="w-full bg-gray-800 border border-gray-600 rounded-md py-2 px-3 text-sm text-white focus:outline-none focus:ring-1 focus:ring-cyan-500 focus:border-cyan-500"
              />
            </div>
            
            <div>
              <label className="block text-sm font-medium text-gray-300 mb-1">Pad Text File (optional)</label>
              <input
                type="text"
                name="padtxt_file"
                value={config.padtxt_file || ""}
                onChange={handleChange}
                className="w-full bg-gray-800 border border-gray-600 rounded-md py-2 px-3 text-sm text-white focus:outline-none focus:ring-1 focus:ring-cyan-500 focus:border-cyan-500"
              />
            </div>
            
            <div>
              <label className="block text-sm font-medium text-gray-300 mb-1">Pad Text Length</label>
              <input
                type="number"
                name="padtxt_len"
                value={config.padtxt_len}
                onChange={handleChange}
                className="w-full bg-gray-800 border border-gray-600 rounded-md py-2 px-3 text-sm text-white focus:outline-none focus:ring-1 focus:ring-cyan-500 focus:border-cyan-500"
              />
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

export default ConfigTab;
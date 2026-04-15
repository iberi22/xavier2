import { useState } from "react";
import { Key, Bell, Moon, Globe, CreditCard, Eye, EyeOff } from "lucide-react";

export function Settings() {
  const [darkMode, setDarkMode] = useState(() =>
    document.documentElement.classList.contains("dark"),
  );
  const [apiKeys, setApiKeys] = useState<{ key: string; value: string; show: boolean }[]>([
    { key: "OpenAI", value: "", show: false },
    { key: "Anthropic", value: "", show: false },
    { key: "Tavily", value: "", show: false },
  ]);
  const [saved, setSaved] = useState(false);

  const toggleDarkMode = () => {
    document.documentElement.classList.toggle("dark");
    setDarkMode(!darkMode);
  };

  const toggleShow = (i: number) => {
    setApiKeys((prev) =>
      prev.map((item, idx) => (idx === i ? { ...item, show: !item.show } : item)),
    );
  };

  const handleSaveKeys = (e: React.FormEvent) => {
    e.preventDefault();
    // In a real app, would send to backend
    setSaved(true);
    setTimeout(() => setSaved(false), 2000);
  };

  return (
    <div className="space-y-8 max-w-2xl">
      <div>
        <h1 className="text-2xl font-bold text-stone-900 dark:text-stone-100">Settings</h1>
        <p className="text-stone-500 text-sm mt-1">Configure your Xavier2 dashboard</p>
      </div>

      {/* API Keys */}
      <section className="bg-white dark:bg-stone-800 rounded-xl border border-stone-200 dark:border-stone-700 p-6">
        <div className="flex items-center gap-3 mb-5">
          <div className="p-2 rounded-lg bg-primary-100 dark:bg-primary-900/30 text-primary-600 dark:text-primary-400">
            <Key size={18} />
          </div>
          <div>
            <h2 className="font-semibold text-stone-900 dark:text-stone-100">API Keys</h2>
            <p className="text-sm text-stone-500 dark:text-stone-400">
              Manage your external API keys
            </p>
          </div>
        </div>

        <form onSubmit={handleSaveKeys} className="space-y-4">
          {apiKeys.map((item, i) => (
            <div key={item.key} className="flex items-center gap-3">
              <label className="w-28 text-sm font-medium text-stone-700 dark:text-stone-300">
                {item.key}
              </label>
              <div className="relative flex-1">
                <input
                  type={item.show ? "text" : "password"}
                  value={item.value}
                  onChange={(e) =>
                    setApiKeys((prev) =>
                      prev.map((x, idx) => (idx === i ? { ...x, value: e.target.value } : x)),
                    )
                  }
                  placeholder={`Your ${item.key} API key`}
                  className="w-full px-3 py-2 pr-10 rounded-lg border border-stone-300 dark:border-stone-600 bg-white dark:bg-stone-900 text-stone-900 dark:text-stone-100 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500"
                />
                <button
                  type="button"
                  onClick={() => toggleShow(i)}
                  className="absolute right-3 top-1/2 -translate-y-1/2 text-stone-400 hover:text-stone-600 dark:hover:text-stone-300"
                >
                  {item.show ? <EyeOff size={16} /> : <Eye size={16} />}
                </button>
              </div>
            </div>
          ))}
          <div className="flex items-center gap-3 pt-2">
            <button
              type="submit"
              className="px-4 py-2 bg-primary-600 text-white rounded-lg text-sm font-medium hover:bg-primary-700 transition-colors"
            >
              Save Keys
            </button>
            {saved && (
              <span className="text-sm text-green-600 dark:text-green-400">
                ✓ Saved successfully
              </span>
            )}
          </div>
        </form>
      </section>

      {/* Plan / Billing */}
      <section className="bg-white dark:bg-stone-800 rounded-xl border border-stone-200 dark:border-stone-700 p-6">
        <div className="flex items-center gap-3 mb-5">
          <div className="p-2 rounded-lg bg-blue-100 dark:bg-blue-900/30 text-blue-600 dark:text-blue-400">
            <CreditCard size={18} />
          </div>
          <div>
            <h2 className="font-semibold text-stone-900 dark:text-stone-100">Plan & Billing</h2>
            <p className="text-sm text-stone-500 dark:text-stone-400">
              Current plan and usage information
            </p>
          </div>
        </div>
        <div className="space-y-3">
          <div className="flex items-center justify-between py-2 border-b border-stone-100 dark:border-stone-700">
            <span className="text-sm text-stone-600 dark:text-stone-300">Current Plan</span>
            <span className="text-sm font-medium text-stone-900 dark:text-stone-100">Pro</span>
          </div>
          <div className="flex items-center justify-between py-2 border-b border-stone-100 dark:border-stone-700">
            <span className="text-sm text-stone-600 dark:text-stone-300">Memory Used</span>
            <span className="text-sm font-medium text-stone-900 dark:text-stone-100">—</span>
          </div>
          <div className="flex items-center justify-between py-2">
            <span className="text-sm text-stone-600 dark:text-stone-300">Active Agents</span>
            <span className="text-sm font-medium text-stone-900 dark:text-stone-100">—</span>
          </div>
        </div>
      </section>

      {/* Preferences */}
      <section className="bg-white dark:bg-stone-800 rounded-xl border border-stone-200 dark:border-stone-700 p-6">
        <div className="flex items-center gap-3 mb-5">
          <div className="p-2 rounded-lg bg-purple-100 dark:bg-purple-900/30 text-purple-600 dark:text-purple-400">
            <Globe size={18} />
          </div>
          <div>
            <h2 className="font-semibold text-stone-900 dark:text-stone-100">Preferences</h2>
            <p className="text-sm text-stone-500 dark:text-stone-400">Customize your experience</p>
          </div>
        </div>

        <div className="space-y-4">
          {/* Dark Mode Toggle */}
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <Moon size={16} className="text-stone-400" />
              <span className="text-sm text-stone-700 dark:text-stone-200">Dark Mode</span>
            </div>
            <button
              onClick={toggleDarkMode}
              className={`relative w-11 h-6 rounded-full transition-colors ${
                darkMode ? "bg-primary-600" : "bg-stone-300 dark:bg-stone-600"
              }`}
            >
              <span
                className={`absolute top-1 w-4 h-4 bg-white rounded-full transition-transform ${
                  darkMode ? "translate-x-6" : "translate-x-1"
                }`}
              />
            </button>
          </div>

          {/* Notifications */}
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <Bell size={16} className="text-stone-400" />
              <span className="text-sm text-stone-700 dark:text-stone-200">Notifications</span>
            </div>
            <button className="relative w-11 h-6 rounded-full bg-stone-300 dark:bg-stone-600">
              <span className="absolute top-1 left-1 w-4 h-4 bg-white rounded-full" />
            </button>
          </div>
        </div>
      </section>
    </div>
  );
}

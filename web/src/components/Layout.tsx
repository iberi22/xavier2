import { ReactNode } from "react";
import { NavLink } from "react-router-dom";
import { LayoutDashboard, Database, Bot, Settings } from "lucide-react";

interface LayoutProps {
  children: ReactNode;
}

const navItems = [
  { to: "/", icon: LayoutDashboard, label: "Dashboard" },
  { to: "/memory", icon: Database, label: "Memory" },
  { to: "/agents", icon: Bot, label: "Agents" },
  { to: "/settings", icon: Settings, label: "Settings" },
];

export function Layout({ children }: LayoutProps) {
  return (
    <div className="min-h-screen flex flex-col lg:flex-row">
      {/* Sidebar */}
      <aside className="w-full lg:w-64 bg-surface-light dark:bg-surface-dark border-b lg:border-b-0 lg:border-r border-stone-200 dark:border-stone-700 shrink-0">
        <div className="p-4 lg:p-6">
          <h1 className="text-lg font-bold text-primary-600 dark:text-primary-400 mb-1">Xavier</h1>
          <p className="text-xs text-stone-500 dark:text-stone-400 uppercase tracking-widest">Dashboard</p>
        </div>
        <nav className="flex lg:flex-col flex-row overflow-x-auto lg:overflow-x-visible">
          {navItems.map(({ to, icon: Icon, label }) => (
            <NavLink
              key={to}
              to={to}
              end={to === "/"}
              className={({ isActive }) =>
                `flex items-center gap-3 px-4 lg:px-6 py-3 text-sm font-medium transition-colors whitespace-nowrap ${
                  isActive
                    ? "text-primary-600 dark:text-primary-400 bg-primary-50 dark:bg-primary-900/20 border-r-2 lg:border-r-0 lg:border-b-2 border-primary-600"
                    : "text-stone-600 dark:text-stone-300 hover:text-primary-600 dark:hover:text-primary-400"
                }`
              }
            >
              <Icon size={18} />
              <span>{label}</span>
            </NavLink>
          ))}
        </nav>
      </aside>

      {/* Main content */}
      <main className="flex-1 p-4 lg:p-8 overflow-y-auto">{children}</main>
    </div>
  );
}

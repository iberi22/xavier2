import { BrowserRouter, Routes, Route } from "react-router-dom";
import { Layout } from "./components/Layout";
import { Dashboard } from "./components/Dashboard";
import { MemoryBrowser } from "./components/MemoryBrowser";
import { AgentManager } from "./components/AgentManager";
import { Settings } from "./components/Settings";

export default function App() {
  return (
    <BrowserRouter>
      <Layout>
        <Routes>
          <Route path="/" element={<Dashboard />} />
          <Route path="/memory" element={<MemoryBrowser />} />
          <Route path="/agents" element={<AgentManager />} />
          <Route path="/settings" element={<Settings />} />
        </Routes>
      </Layout>
    </BrowserRouter>
  );
}

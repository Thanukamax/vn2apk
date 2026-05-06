import { Outlet } from "react-router-dom";
import { Toaster } from "sonner";
import { PipelineProvider } from "@/contexts/PipelineContext";
import { Sidebar } from "./Sidebar";

export function Layout() {
  return (
    <PipelineProvider>
      <div className="flex h-screen w-screen overflow-hidden bg-background text-foreground">
        <Sidebar />
        <main className="flex flex-1 flex-col overflow-hidden">
          <Outlet />
        </main>
        <Toaster theme="dark" position="bottom-right" />
      </div>
    </PipelineProvider>
  );
}

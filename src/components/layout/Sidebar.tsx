import { NavLink } from "react-router-dom";
import { Hammer, Loader2, Settings, Wrench } from "lucide-react";
import { usePipelineContext } from "@/contexts/PipelineContext";
import { cn } from "@/lib/utils";

const navItems = [
  { to: "/toolchain", label: "Toolchain", icon: Wrench },
  { to: "/build", label: "Build", icon: Hammer },
  { to: "/settings", label: "Settings", icon: Settings },
];

export function Sidebar() {
  const { buildState } = usePipelineContext();
  const isBuilding = buildState === "building";

  return (
    <aside className="flex h-full w-56 flex-col border-r border-border bg-card">
      <div className="flex h-14 items-center px-4 border-b border-border">
        <span className="text-sm font-bold tracking-widest text-foreground/80 uppercase">
          vn2apk
        </span>
      </div>
      <nav className="flex flex-col gap-1 p-2 flex-1">
        {navItems.map(({ to, label, icon: Icon }) => (
          <NavLink
            key={to}
            to={to}
            className={({ isActive }) =>
              cn(
                "flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors",
                isActive
                  ? "bg-accent text-accent-foreground"
                  : "text-muted-foreground hover:bg-accent/50 hover:text-foreground"
              )
            }
          >
            <Icon className="h-4 w-4 shrink-0" />
            {label}
            {/* Show spinner on Build nav item while a build is running */}
            {to === "/build" && isBuilding && (
              <Loader2 className="ml-auto h-3 w-3 animate-spin text-primary" />
            )}
          </NavLink>
        ))}
      </nav>
      <div className="p-4 text-xs text-muted-foreground/50">v0.1.0</div>
    </aside>
  );
}
